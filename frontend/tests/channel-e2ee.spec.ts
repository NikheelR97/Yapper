import { test, expect, type Browser } from '@playwright/test';
import { loginViaApi, setInstallationId } from './auth-helper.js';

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

// Fixed installation IDs so the backend reuses the same device records across runs.
const USER_A_CHANNEL_INSTALLATION = 'ac000000-0000-4000-8000-000000000001';
const USER_B_CHANNEL_INSTALLATION = 'bc000000-0000-4000-8000-000000000001';

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

test.describe('Channel E2EE — cross-user message decryption', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL, E2E_PASSWORD, E2E_EMAIL_2, E2E_PASSWORD_2 to run cross-user E2E tests',
	);

	test(
		'User B can read a channel message sent by User A (Sender Key decryption regression)',
		async ({ browser }) => {
			test.slow(); // Two browser contexts + Signal key setup

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

			// ── User A sends a message ────────────────────────────────────────────────
			const testMsg = `E2E E2EE channel message ${Date.now()}`;
			const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
			const pageA = await ctxA.newPage();

			try {
				await loginAndWaitReady(pageA, USER_A_EMAIL, USER_A_PASS, USER_A_CHANNEL_INSTALLATION);
				await pageA.goto(`/servers/${serverId}/channels/${channelId}`);

				const inputA = pageA.locator('textarea[aria-label="Message"]').first();
				await expect(inputA).toBeEnabled({ timeout: 60_000 });
				await inputA.fill(testMsg);
				await inputA.press('Enter');

				// Confirm the message appears for User A (sender-side render)
				await expect(pageA.getByText(testMsg)).toBeVisible({ timeout: 15_000 });
			} finally {
				await ctxA.close();
			}

			// ── User B opens the channel and must see the plaintext ───────────────────
			const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
			const pageB = await ctxB.newPage();

			try {
				await loginAndWaitReady(pageB, USER_B_EMAIL, USER_B_PASS, USER_B_CHANNEL_INSTALLATION);
				await pageB.goto(`/servers/${serverId}/channels/${channelId}`);

				const inputB = pageB.locator('textarea[aria-label="Message"]').first();
				await expect(inputB).toBeEnabled({ timeout: 60_000 });

				// The regression: before the fix, this would show "Unable to decrypt"
				// when User A's sender key distribution had a fetch failure during
				// User B joining (senderDhPub was undefined → wrong ECIES IKM).
				await expect(pageB.getByText(testMsg)).toBeVisible({ timeout: 20_000 });
				await expect(pageB.getByText(/Unable to decrypt/i)).toHaveCount(0);
			} finally {
				await ctxB.close();
			}
		},
	);

	test(
		'Both users can send and receive messages in the same channel (bidirectional)',
		async ({ browser }) => {
			test.slow();

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

			// User A sends first
			const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
			const pageA = await ctxA.newPage();
			await loginAndWaitReady(pageA, USER_A_EMAIL, USER_A_PASS, USER_A_CHANNEL_INSTALLATION);
			await pageA.goto(`/servers/${serverId}/channels/${channelId}`);
			const inputA = pageA.locator('textarea[aria-label="Message"]').first();
			await expect(inputA).toBeEnabled({ timeout: 60_000 });
			await inputA.fill(msgFromA);
			await inputA.press('Enter');
			await expect(pageA.getByText(msgFromA)).toBeVisible({ timeout: 15_000 });
			await ctxA.close();

			// User B opens, sends a reply
			const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
			const pageB = await ctxB.newPage();
			await loginAndWaitReady(pageB, USER_B_EMAIL, USER_B_PASS, USER_B_CHANNEL_INSTALLATION);
			await pageB.goto(`/servers/${serverId}/channels/${channelId}`);
			const inputB = pageB.locator('textarea[aria-label="Message"]').first();
			await expect(inputB).toBeEnabled({ timeout: 60_000 });

			// Sees User A's message
			await expect(pageB.getByText(msgFromA)).toBeVisible({ timeout: 20_000 });
			await expect(pageB.getByText(/Unable to decrypt/i)).toHaveCount(0);

			// Sends their own
			await inputB.fill(msgFromB);
			await inputB.press('Enter');
			await expect(pageB.getByText(msgFromB)).toBeVisible({ timeout: 15_000 });
			await ctxB.close();
		},
	);
});
