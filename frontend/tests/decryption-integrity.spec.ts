import { test, expect, type Page, type Browser, type BrowserContext } from '@playwright/test';
import {
	loginViaApi,
	setInstallationId,
	seedTrustedPrimaryDevice,
	seedTrustedPrimaryDeviceB,
	PRIMARY_INSTALLATION_ID,
	B_PRIMARY_INSTALLATION_ID,
	SECONDARY_INSTALLATION_ID,
} from './auth-helper.js';
import { navigateClientSide, waitForKeyBundles } from './helpers/wait-for.js';

/**
 * Decryption integrity E2E tests.
 *
 * Verifies that neither DMs nor server channel messages show "Unable to decrypt"
 * or any decryption-error state across cross-user and multi-device scenarios.
 *
 * These tests are the last line of defence against:
 * - X3DH / symmetric-ratchet regressions in DMs
 * - Sender Key distribution failures in server channels
 * - `key_dist_request` broadcast failures (e.g. wrong table name in SQL)
 * - Multi-device key exchange races
 */

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';

interface Session {
	accessToken: string;
	csrfToken: string;
	userId: string;
	username: string;
	displayName: string | null;
}

function apiHeaders(s: Session) {
	return {
		'Content-Type': 'application/json',
		Authorization: `Bearer ${s.accessToken}`,
		Cookie: `csrf_token=${s.csrfToken}`,
		'X-CSRF-Token': s.csrfToken,
	};
}

async function createApiSession(
	email: string,
	pass: string,
	installId: string,
	label: string,
): Promise<Session> {
	const d = await loginViaApi(email, pass, { installationId: installId, label });
	return {
		accessToken: d.accessToken,
		csrfToken: d.csrfToken,
		userId: String(d.user.id),
		username: String(d.user.username),
		displayName:
			typeof d.user.displayName === 'string'
				? d.user.displayName
				: typeof d.user.display_name === 'string'
					? (d.user.display_name as string)
					: null,
	};
}

async function loginAndWaitReady(
	page: Page,
	email: string,
	pass: string,
	installId: string,
): Promise<void> {
	await setInstallationId(page, installId);
	await page.goto('/login');
	await page.fill('#email', email);
	await page.fill('#password', pass);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 30_000 });
	await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
		timeout: 45_000,
	});
}

async function createDmConversation(session: Session, peerId: string): Promise<string> {
	const res = await fetch(`${API_URL}/api/v1/conversations`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: JSON.stringify({ peer_id: peerId }),
	});
	if (!res.ok) throw new Error(`createDmConversation failed: ${res.status} ${await res.text()}`);
	const body = (await res.json()) as { id?: string };
	if (typeof body.id !== 'string') throw new Error('Missing conversation id');
	return body.id;
}

async function createServer(session: Session): Promise<{ serverId: string; channelId: string }> {
	const createRes = await fetch(`${API_URL}/api/v1/servers`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: JSON.stringify({ name: `E2E Decrypt ${Date.now()}` }),
	});
	if (!createRes.ok) throw new Error(`createServer failed: ${createRes.status}`);
	const server = (await createRes.json()) as { id: string };

	const chRes = await fetch(`${API_URL}/api/v1/servers/${server.id}/channels`, {
		headers: { Authorization: `Bearer ${session.accessToken}` },
	});
	if (!chRes.ok) throw new Error(`listChannels failed: ${chRes.status}`);
	const channels = (await chRes.json()) as Array<{ id: string }>;
	if (!channels[0]?.id) throw new Error('No channel after server creation');
	return { serverId: server.id, channelId: channels[0].id };
}

async function createInvite(session: Session, serverId: string): Promise<string> {
	const res = await fetch(`${API_URL}/api/v1/servers/${serverId}/invite`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: JSON.stringify({ max_uses: 5 }),
	});
	if (!res.ok) throw new Error(`createInvite failed: ${res.status}`);
	return ((await res.json()) as { code: string }).code;
}

async function joinByInvite(session: Session, code: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/v1/servers/join/${code}`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: '{}',
	});
	if (!res.ok) throw new Error(`joinByInvite failed: ${res.status} ${await res.text()}`);
}

/** Count "Unable to decrypt" texts visible on the page. */
async function countDecryptErrors(page: Page): Promise<number> {
	return page.getByText(/Unable to decrypt/i).count();
}

/**
 * Assert zero decrypt errors on a page, with a readable test-failure message.
 *
 * @param page - The Playwright page to inspect.
 * @param label - Human-readable label for the assertion.
 * @param baseline - Optional count of pre-existing decrypt errors to ignore.
 *   Pass the result of `countDecryptErrors(page)` captured *before* the action
 *   under test to avoid false positives from stale history messages.
 */
async function assertNoDecryptErrors(
	page: Page,
	label: string,
	baseline = 0,
): Promise<void> {
	const errors = await countDecryptErrors(page);
	const newErrors = errors - baseline;
	expect(
		newErrors,
		`${label} should have 0 NEW "Unable to decrypt" messages but found ${newErrors} (total ${errors}, baseline ${baseline})`,
	).toBe(0);
}

/**
 * Send a message via the textarea and wait until it appears in the message list.
 * Returns the unique message text.
 */
async function sendAndConfirm(page: Page, text: string): Promise<void> {
	const input = page.locator('textarea[aria-label="Message"]').first();
	await expect(input).toBeEnabled({ timeout: 60_000 });
	await input.fill(text);
	await input.press('Enter');
	await expect(page.getByText(text)).toBeVisible({ timeout: 20_000 });
}

/**
 * Wait for a message to appear on a recipient page, polling with retries
 * to handle key-exchange latency.
 */
async function waitForMessage(page: Page, text: string, timeoutMs = 60_000): Promise<void> {
	const interval = 2_000;
	const iterations = Math.ceil(timeoutMs / interval);
	for (let i = 0; i < iterations; i++) {
		const visible = await page.getByText(text).isVisible().catch(() => false);
		if (visible) return;
		await page.waitForTimeout(interval);
	}
	throw new Error(`Message "${text}" not visible after ${timeoutMs / 1000}s`);
}

// ─── DM Decryption Integrity ─────────────────────────────────────────────────

test.describe('DM decryption integrity', () => {
	test.use({ storageState: { cookies: [], origins: [] } });
	test.describe.configure({ timeout: 180_000 });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL, E2E_PASSWORD, E2E_EMAIL_2, E2E_PASSWORD_2',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('Bidirectional DM — both users decrypt each other without errors', async ({ browser }) => {
		test.setTimeout(360_000);

		const sessionA = await createApiSession(
			USER_A_EMAIL,
			USER_A_PASS,
			PRIMARY_INSTALLATION_ID,
			'E2E Decrypt DM A',
		);
		const sessionB = await createApiSession(
			USER_B_EMAIL,
			USER_B_PASS,
			B_PRIMARY_INSTALLATION_ID,
			'E2E Decrypt DM B',
		);

		const conversationId = await createDmConversation(sessionA, sessionB.userId);
		const msgFromA = `DM from A ${Date.now()}`;
		const msgFromB = `DM from B ${Date.now()}`;

		const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const pageA = await ctxA.newPage();
		const pageB = await ctxB.newPage();

		try {
			// Both log in concurrently — ensures prekeys are uploaded before either encrypts
			await Promise.all([
				loginAndWaitReady(pageA, USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID),
				loginAndWaitReady(pageB, USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID),
			]);

			// Wait for both users' key bundles to be available on the server
			await Promise.all([
				waitForKeyBundles(sessionA.accessToken, sessionA.userId),
				waitForKeyBundles(sessionA.accessToken, sessionB.userId),
			]);

			// Determine the labels we can use to find the conversation in the sidebar
			const bLabels = [sessionB.username, sessionB.displayName].filter(
				(l): l is string => typeof l === 'string' && l.length > 0,
			);
			const aLabels = [sessionA.username, sessionA.displayName].filter(
				(l): l is string => typeof l === 'string' && l.length > 0,
			);

			// ── A opens conversation and sends ────────────────────────────────────
			await navigateClientSide(pageA, '/dm');
			await expect(pageA.locator('button.conv-btn').first()).toBeVisible({ timeout: 15_000 });
			for (const label of bLabels) {
				const btn = pageA.locator('button.conv-btn', { hasText: label }).first();
				if (await btn.isVisible({ timeout: 2_000 }).catch(() => false)) {
					await btn.click();
					break;
				}
			}
			await pageA.waitForURL(new RegExp(`/dm/${conversationId}`), { timeout: 10_000 });
			// Wait for the textarea to be enabled: this signals that the message history
			// REST fetch has completed and any old "Unable to decrypt" entries from prior
			// runs are now rendered. Capturing baseline BEFORE the history loads yields
			// baseline=0, causing false positives on the 50+ historical decrypt errors.
			const inputA = pageA.locator('textarea[aria-label="Message"]').first();
			await expect(inputA).toBeEnabled({ timeout: 60_000 });
			// 10 s settle — with 50+ historical messages each rendered as a DOM node,
			// 2 s was insufficient for all "Unable to decrypt" nodes to finish mounting
			// before the baseline count was captured.
			await pageA.waitForTimeout(10_000);
			const baselineA = await countDecryptErrors(pageA);
			await sendAndConfirm(pageA, msgFromA);
			await assertNoDecryptErrors(pageA, 'User A after sending', baselineA);

			// ── B opens conversation and reads A's message ────────────────────────
			await navigateClientSide(pageB, '/dm');
			await expect(pageB.locator('button.conv-btn').first()).toBeVisible({ timeout: 15_000 });
			for (const label of aLabels) {
				const btn = pageB.locator('button.conv-btn', { hasText: label }).first();
				if (await btn.isVisible({ timeout: 2_000 }).catch(() => false)) {
					await btn.click();
					break;
				}
			}
			await pageB.waitForURL(new RegExp(`/dm/${conversationId}`), { timeout: 10_000 });
			const inputB = pageB.locator('textarea[aria-label="Message"]').first();
			await expect(inputB).toBeEnabled({ timeout: 60_000 });
			// Prove A's message decrypted: if it failed the plaintext is not visible and
			// this call would throw after 90 s (can't see "DM from A <ts>" in the DOM).
			await waitForMessage(pageB, msgFromA, 90_000);
			// Capture baseline AFTER the message history is fully rendered. Historical
			// "Unable to decrypt" nodes from prior runs (same two users → same conversation
			// ID) appear only after the initial fetch + async decrypt completes — which is
			// often triggered by the WS push of A's new message, not the initial load.
			// Capturing before waitForMessage yields baseline=0, making assertNoDecryptErrors
			// count those historical errors as "new" and fail.
			await pageB.waitForTimeout(5_000);
			const baselineB = await countDecryptErrors(pageB);

			// ── B sends reply ─────────────────────────────────────────────────────
			await sendAndConfirm(pageB, msgFromB);
			await assertNoDecryptErrors(pageB, 'User B after sending reply', baselineB);

			// ── A sees B's reply ──────────────────────────────────────────────────
			await waitForMessage(pageA, msgFromB);
			await assertNoDecryptErrors(pageA, 'User A after receiving B\'s reply', baselineA);
		} finally {
			await ctxA.close();
			await ctxB.close();
		}
	});
});

// ─── Channel Decryption Integrity ────────────────────────────────────────────

test.describe('Channel decryption integrity', () => {
	test.use({ storageState: { cookies: [], origins: [] } });
	test.describe.configure({ timeout: 180_000 });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL, E2E_PASSWORD, E2E_EMAIL_2, E2E_PASSWORD_2',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('Cross-user channel messages decrypt without errors (Sender Key distribution)', async ({
		browser,
	}) => {
		test.setTimeout(300_000);

		const sessionA = await createApiSession(
			USER_A_EMAIL,
			USER_A_PASS,
			PRIMARY_INSTALLATION_ID,
			'E2E Decrypt Ch A',
		);
		const sessionB = await createApiSession(
			USER_B_EMAIL,
			USER_B_PASS,
			B_PRIMARY_INSTALLATION_ID,
			'E2E Decrypt Ch B',
		);

		const { serverId, channelId } = await createServer(sessionA);
		const inviteCode = await createInvite(sessionA, serverId);
		await joinByInvite(sessionB, inviteCode);

		const msgFromA = `CH from A ${Date.now()}`;
		const msgFromB = `CH from B ${Date.now()}`;

		const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const pageA = await ctxA.newPage();
		const pageB = await ctxB.newPage();

		try {
			// Both log in concurrently so prekeys exist before SenderKey distribution
			await Promise.all([
				loginAndWaitReady(pageA, USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID),
				loginAndWaitReady(pageB, USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID),
			]);

			// initializeSignalKeys() fires as a void background promise right after
			// ready=true (boot-perf fix). Fresh browser contexts (empty storageState)
			// have no IndexedDB keys and must generate + upload new ones — 3 serial HTTP
			// calls that take ~2–5 s. waitForKeyBundles returns immediately when stale
			// bundles from a prior run exist, so without this extra wait A's joinChannel()
			// would encrypt the SenderKey for B's OLD public key, which B's fresh context
			// has since replaced. 15 s comfortably covers CI load spikes.
			await Promise.all([pageA.waitForTimeout(15_000), pageB.waitForTimeout(15_000)]);

			await Promise.all([
				waitForKeyBundles(sessionA.accessToken, sessionA.userId),
				waitForKeyBundles(sessionA.accessToken, sessionB.userId),
			]);

			// ── A navigates to channel and sends ──────────────────────────────────
			await navigateClientSide(pageA, `/servers/${serverId}/channels/${channelId}`);
			await sendAndConfirm(pageA, msgFromA);
			await assertNoDecryptErrors(pageA, 'User A after sending in channel');

			// ── B navigates to same channel — decrypts A's message ────────────────
			await navigateClientSide(pageB, `/servers/${serverId}/channels/${channelId}`);
			// Wait for prepareChannel() to complete: fetches A's SenderKey distribution.
			// Without this, waitForMessage sees "Unable to decrypt" instead of the text.
			const inputB = pageB.locator('textarea[aria-label="Message"]').first();
			await expect(inputB).toBeEnabled({ timeout: 60_000 });
			// Extra settle: the SenderKey round-trip (key_dist_request → key_dist_v2)
			// may complete slightly after prepareChannel() resolves. 5 s is sufficient
			// for the Fly.io JNB + Neon Azure GWC round-trip observed in production.
			await pageB.waitForTimeout(5_000);
			await waitForMessage(pageB, msgFromA, 120_000);
			await assertNoDecryptErrors(pageB, 'User B after opening channel with A\'s message');

			// ── B sends reply in channel ──────────────────────────────────────────
			await sendAndConfirm(pageB, msgFromB);
			await assertNoDecryptErrors(pageB, 'User B after sending channel reply');

			// ── A re-enters channel to pick up B's SenderKey distribution ─────────
			// Navigate away then back to trigger prepareChannel() → fetchPendingKeyDists()
			await navigateClientSide(pageA, '/explore');
			await navigateClientSide(pageA, `/servers/${serverId}/channels/${channelId}`);
			const inputARe = pageA.locator('textarea[aria-label="Message"]').first();
			await expect(inputARe).toBeEnabled({ timeout: 60_000 });
			await pageA.waitForTimeout(5_000);
			await waitForMessage(pageA, msgFromB, 120_000);
			await assertNoDecryptErrors(pageA, 'User A after re-entering channel');

			// Both messages visible on both pages, zero decrypt errors
			await expect(pageA.getByText(msgFromA)).toBeVisible();
			await expect(pageB.getByText(msgFromA)).toBeVisible();
		} finally {
			await ctxA.close();
			await ctxB.close();
		}
	});

	test('Late joiner decrypts messages sent after joining (key_dist_request regression)', async ({
		browser,
	}) => {
		// This specifically tests the bug where key_dist_request broadcast used
		// the wrong SQL table name, preventing SenderKey redistribution.
		test.setTimeout(480_000);

		const sessionA = await createApiSession(
			USER_A_EMAIL,
			USER_A_PASS,
			PRIMARY_INSTALLATION_ID,
			'E2E Late Join A',
		);
		const sessionB = await createApiSession(
			USER_B_EMAIL,
			USER_B_PASS,
			B_PRIMARY_INSTALLATION_ID,
			'E2E Late Join B',
		);

		// A creates server, but B does NOT join yet
		const { serverId, channelId } = await createServer(sessionA);

		const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const pageA = await ctxA.newPage();
		const pageB = await ctxB.newPage();

		// Capture Signal-layer console messages so we can diagnose silent failures
		// in handleKeyDistRequest / receiveSenderKeyDist / fetchAndStorePendingDists.
		const aLogs: string[] = [];
		const bLogs: string[] = [];
		pageA.on('console', (msg) => {
			const t = msg.text();
			if (t.includes('[signal]') || t.includes('[Signal]')) aLogs.push(`A: ${t}`);
		});
		pageB.on('console', (msg) => {
			const t = msg.text();
			if (t.includes('[signal]') || t.includes('[Signal]')) bLogs.push(`B: ${t}`);
		});

		try {
			// Both log in concurrently
			await Promise.all([
				loginAndWaitReady(pageA, USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID),
				loginAndWaitReady(pageB, USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID),
			]);

			// Same race as the channel test: fresh browser contexts upload NEW Signal keys
			// as a background void promise (initializeSignalKeys). Without this wait,
			// A's joinChannel() may encrypt using B's stale (pre-session) public key.
			await Promise.all([pageA.waitForTimeout(15_000), pageB.waitForTimeout(15_000)]);

			await Promise.all([
				waitForKeyBundles(sessionA.accessToken, sessionA.userId),
				waitForKeyBundles(sessionA.accessToken, sessionB.userId),
			]);

			// A navigates to channel and sends a message — A's SenderKey is created
			// but only distributed to A's own devices (B is not a member yet)
			await navigateClientSide(pageA, `/servers/${serverId}/channels/${channelId}`);
			const earlyMsg = `Before B joined ${Date.now()}`;
			await sendAndConfirm(pageA, earlyMsg);

			// Now B joins via invite
			const inviteCode = await createInvite(sessionA, serverId);
			await joinByInvite(sessionB, inviteCode);

			// A sends another message AFTER B joined. The key_dist_request broadcast
			// (now fixed to use server_memberships) should tell A to redistribute
			// its SenderKey to B.
			const afterJoinMsg = `After B joined ${Date.now()}`;

			// B navigates to channel — this triggers prepareChannel() which:
			// 1. Calls joinChannel → stores B's SenderKey distribution to A
			// 2. Backend broadcasts key_dist_request to all members (including A)
			// 3. A's handleKeyDistRequest sends its SenderKey to B via POST + key_dist_v2 WS
			await navigateClientSide(pageB, `/servers/${serverId}/channels/${channelId}`);
			const inputB = pageB.locator('textarea[aria-label="Message"]').first();
			await expect(inputB).toBeEnabled({ timeout: 60_000 });

			// Give A time to receive key_dist_request and redistribute its SenderKey.
			await pageB.waitForTimeout(10_000);

			// Navigate B away and back to force fetchPendingKeyDists (DB fallback path).
			// If the real-time key_dist_v2 WS event was missed, this GET call returns
			// A's SenderKey from the DB.
			await navigateClientSide(pageB, '/explore');
			await navigateClientSide(pageB, `/servers/${serverId}/channels/${channelId}`);
			const inputBRefresh = pageB.locator('textarea[aria-label="Message"]').first();
			await expect(inputBRefresh).toBeEnabled({ timeout: 60_000 });

			// Wait for messages to finish rendering (including any "Unable to decrypt"
			// nodes for A's pre-join message).  Capture the baseline AFTER the second
			// navigation so it includes pre-existing decrypt errors from earlyMsg.
			await pageB.waitForTimeout(10_000);
			const baselineLate = await countDecryptErrors(pageB);

			// A sends the post-join message — B now holds A's SenderKey (either from
			// the re-navigation DB fetch or from the key_dist_v2 WS event above).
			await sendAndConfirm(pageA, afterJoinMsg);

			// Print captured Signal logs before the critical send so they appear
			// in the test output even if waitForMessage times out.
			console.log('=== Signal logs before send ===');
			console.log([...aLogs, ...bLogs].join('\n') || '(none)');
			console.log('=== B decrypt errors before send:', await countDecryptErrors(pageB));

			// 90 s: allows for one extra retry cycle in case of transient WS delay.
			await waitForMessage(pageB, afterJoinMsg, 90_000);

			console.log('=== Signal logs after message visible ===');
			console.log([...aLogs, ...bLogs].join('\n') || '(none)');

			await assertNoDecryptErrors(
				pageB,
				'User B (late joiner) after receiving post-join message',
				baselineLate,
			);
		} finally {
			// Always dump logs on failure for post-mortem analysis
			if (aLogs.length || bLogs.length) {
				console.log('=== Final Signal logs ===');
				console.log([...aLogs, ...bLogs].join('\n'));
			}
			await ctxA.close();
			await ctxB.close();
		}
	});
});
