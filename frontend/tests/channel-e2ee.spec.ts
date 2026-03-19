import { test, expect, type Browser } from '@playwright/test';
import {
	loginViaApi,
	setInstallationId,
	seedTrustedPrimaryDevice,
	seedTrustedPrimaryDeviceB,
	PRIMARY_INSTALLATION_ID,
	B_PRIMARY_INSTALLATION_ID,
} from './auth-helper.js';
import { navigateClientSide } from './helpers/wait-for.js';

/**
 * Cross-user channel E2E encryption tests.
 *
 * Verifies that a message sent by User A in a server channel is correctly
 * decrypted by User B — catching regressions in the Sender Key distribution
 * protocol (the "Unable to decrypt" class of bugs).
 *
 * Requires two accounts: E2E_EMAIL/PASSWORD and E2E_EMAIL_2/PASSWORD_2.
 * Each user logs in as a fresh browser context (no shared storage state)
 * to simulate independent devices with separate key material.
 */

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';

// Use the seeded primary installation IDs so both devices are trusted across runs.
const USER_A_CHANNEL_INSTALLATION = PRIMARY_INSTALLATION_ID;
const USER_B_CHANNEL_INSTALLATION = B_PRIMARY_INSTALLATION_ID;

interface Session {
	accessToken: string;
	csrfToken: string;
	userId: string;
	username: string;
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
	};
}

async function createServer(session: Session): Promise<{ serverId: string; channelId: string }> {
	const name = `E2E Channel E2EE ${Date.now()}`;
	const createRes = await fetch(`${API_URL}/api/v1/servers`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: JSON.stringify({ name }),
	});
	if (!createRes.ok) throw new Error(`createServer failed: ${createRes.status}`);
	const server = (await createRes.json()) as { id: string };

	const chRes = await fetch(`${API_URL}/api/v1/servers/${server.id}/channels`, {
		headers: { Authorization: `Bearer ${session.accessToken}` },
	});
	if (!chRes.ok) throw new Error(`listChannels failed: ${chRes.status}`);
	const channels = (await chRes.json()) as Array<{ id: string }>;
	const channelId = channels[0]?.id;
	if (!channelId) throw new Error('No channel after server creation');

	return { serverId: server.id, channelId };
}

async function createInvite(session: Session, serverId: string): Promise<string> {
	const res = await fetch(`${API_URL}/api/v1/servers/${serverId}/invite`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: JSON.stringify({ max_uses: 5 }),
	});
	if (!res.ok) throw new Error(`createInvite failed: ${res.status}`);
	const body = (await res.json()) as { code: string };
	return body.code;
}

async function joinByInvite(session: Session, code: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/v1/servers/join/${code}`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: '{}',
	});
	if (!res.ok) throw new Error(`joinByInvite failed: ${res.status} ${await res.text()}`);
}

async function loginAndWaitReady(
	page: ReturnType<Browser['newPage']> extends Promise<infer P> ? P : never,
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
	// Wait for Signal key bootstrap to finish before navigating to a channel
	await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
		timeout: 45_000,
	});
}

/**
 * Poll the key-bundle API until userId has at least one trusted device bundle
 * available. The app uploads keys asynchronously after the loading screen hides,
 * so this poll gives the upload time to complete before prepareChannel() runs.
 */
async function waitForBundles(authToken: string, userId: string): Promise<void> {
	for (let i = 0; i < 30; i++) {
		const res = await fetch(`${API_URL}/api/v2/keys/${userId}/bundles`, {
			headers: { Authorization: `Bearer ${authToken}` },
		});
		if (res.ok) {
			const bundles = (await res.json()) as unknown[];
			if (bundles.length > 0) return;
		}
		await new Promise<void>((resolve) => setTimeout(resolve, 1_000));
	}
	throw new Error(`Key bundles for user ${userId} not available after 30 s`);
}

test.describe('Channel E2EE — cross-user message decryption', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL, E2E_PASSWORD, E2E_EMAIL_2, E2E_PASSWORD_2 to run cross-user E2E tests',
	);

	test.beforeAll(() => {
		// Only device seeding — browser logins happen within each test so that
		// A and B use the SAME context for both key upload and message read.
		// (Each fresh context generates new Signal keys; uploading in beforeAll
		// then re-logging in the test overwrites those keys and breaks decryption.)
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test(
		'User B can read a channel message sent by User A (Sender Key decryption regression)',
		async ({ browser }) => {
			// Two concurrent browser logins + Signal key setup + message round-trip
			test.setTimeout(300_000);

			// ── API setup ─────────────────────────────────────────────────────────────
			const sessionA = await createApiSession(
				USER_A_EMAIL,
				USER_A_PASS,
				USER_A_CHANNEL_INSTALLATION,
				'E2E Channel A',
			);
			const sessionB = await createApiSession(
				USER_B_EMAIL,
				USER_B_PASS,
				USER_B_CHANNEL_INSTALLATION,
				'E2E Channel B',
			);

			const { serverId, channelId } = await createServer(sessionA);
			const inviteCode = await createInvite(sessionA, serverId);
			await joinByInvite(sessionB, inviteCode);

			const testMsg = `E2E E2EE channel message ${Date.now()}`;

			// Open both contexts before any message is sent so that when A calls
			// prepareChannel() the SenderKey distribution is encrypted with B's
			// freshly-uploaded public key. B then navigates using the SAME context
			// (same IndexedDB) so the private key matches the distribution.
			const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
			const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
			const pageA = await ctxA.newPage();
			const pageB = await ctxB.newPage();

			try {
				// Log in both concurrently — ensures both have uploaded prekeys before
				// A triggers SenderKey distribution by navigating to the channel.
				await Promise.all([
					loginAndWaitReady(pageA, USER_A_EMAIL, USER_A_PASS, USER_A_CHANNEL_INSTALLATION),
					loginAndWaitReady(pageB, USER_B_EMAIL, USER_B_PASS, USER_B_CHANNEL_INSTALLATION),
				]);

				// Confirm both users' key bundles are on the server before A navigates.
				// The app uploads keys async after the loading screen hides; this poll
				// gives the upload time to land so prepareChannel() can distribute to B.
				await Promise.all([
					waitForBundles(sessionA.accessToken, sessionA.userId),
					waitForBundles(sessionA.accessToken, sessionB.userId),
				]);

				// ── User A navigates and sends ────────────────────────────────────────
				// Use client-side navigation to avoid a hard reload that resets authStore
				await navigateClientSide(pageA, `/servers/${serverId}/channels/${channelId}`);
				const inputA = pageA.locator('textarea[aria-label="Message"]').first();
				await expect(inputA).toBeEnabled({ timeout: 60_000 });
				await inputA.fill(testMsg);
				await inputA.press('Enter');
				await expect(pageA.getByText(testMsg)).toBeVisible({ timeout: 15_000 });

				// ── User B opens the channel using the same context (same keys) ────────
				await navigateClientSide(pageB, `/servers/${serverId}/channels/${channelId}`);
				const inputB = pageB.locator('textarea[aria-label="Message"]').first();
				await expect(inputB).toBeEnabled({ timeout: 60_000 });
				await expect(pageB.getByText(testMsg)).toBeVisible({ timeout: 20_000 });
				await expect(pageB.getByText(/Unable to decrypt/i)).toHaveCount(0);
			} finally {
				await ctxA.close();
				await ctxB.close();
			}
		},
	);

	test(
		'Both users can send and receive messages in the same channel (bidirectional)',
		async ({ browser }) => {
			test.setTimeout(300_000);

			const sessionA = await createApiSession(
				USER_A_EMAIL,
				USER_A_PASS,
				USER_A_CHANNEL_INSTALLATION,
				'E2E Bidir A',
			);
			const sessionB = await createApiSession(
				USER_B_EMAIL,
				USER_B_PASS,
				USER_B_CHANNEL_INSTALLATION,
				'E2E Bidir B',
			);

			const { serverId, channelId } = await createServer(sessionA);
			const inviteCode = await createInvite(sessionA, serverId);
			await joinByInvite(sessionB, inviteCode);

			const msgFromA = `From A ${Date.now()}`;
			const msgFromB = `From B ${Date.now()}`;

			const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
			const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
			const pageA = await ctxA.newPage();
			const pageB = await ctxB.newPage();

			try {
				// Log in both concurrently before either navigates to the channel
				await Promise.all([
					loginAndWaitReady(pageA, USER_A_EMAIL, USER_A_PASS, USER_A_CHANNEL_INSTALLATION),
					loginAndWaitReady(pageB, USER_B_EMAIL, USER_B_PASS, USER_B_CHANNEL_INSTALLATION),
				]);

				// Confirm both users' key bundles are on the server before A navigates.
				await Promise.all([
					waitForBundles(sessionA.accessToken, sessionA.userId),
					waitForBundles(sessionA.accessToken, sessionB.userId),
				]);

				// A navigates, sends first message (distributes SenderKey to B)
				await navigateClientSide(pageA, `/servers/${serverId}/channels/${channelId}`);
				const inputA = pageA.locator('textarea[aria-label="Message"]').first();
				await expect(inputA).toBeEnabled({ timeout: 60_000 });
				await inputA.fill(msgFromA);
				await inputA.press('Enter');
				await expect(pageA.getByText(msgFromA)).toBeVisible({ timeout: 15_000 });

				// B navigates (same context → same private key → decrypts A's distribution)
				await navigateClientSide(pageB, `/servers/${serverId}/channels/${channelId}`);
				const inputB = pageB.locator('textarea[aria-label="Message"]').first();
				await expect(inputB).toBeEnabled({ timeout: 60_000 });
				await expect(pageB.getByText(msgFromA)).toBeVisible({ timeout: 20_000 });
				await expect(pageB.getByText(/Unable to decrypt/i)).toHaveCount(0);

				// B sends a reply (distributes B's SenderKey to A)
				await inputB.fill(msgFromB);
				await inputB.press('Enter');
				await expect(pageB.getByText(msgFromB)).toBeVisible({ timeout: 15_000 });

				// A re-navigates to the channel — this triggers prepareChannel() which
				// fetches B's pending SenderKey distribution (stored server-side) and
				// decrypts B's reply using the newly received distribution.
				// Navigate away then back to trigger prepareChannel() without a hard reload
				await navigateClientSide(pageA, '/explore');
				await navigateClientSide(pageA, `/servers/${serverId}/channels/${channelId}`);
				const inputA2 = pageA.locator('textarea[aria-label="Message"]').first();
				await expect(inputA2).toBeEnabled({ timeout: 60_000 });
				await expect(pageA.getByText(msgFromB)).toBeVisible({ timeout: 20_000 });
				await expect(pageA.getByText(msgFromA)).toBeVisible({ timeout: 10_000 });
				await expect(pageA.getByText(/Unable to decrypt/i)).toHaveCount(0);
			} finally {
				await ctxA.close();
				await ctxB.close();
			}
		},
	);
});
