import { test, expect, type Page } from '@playwright/test';
import { ed25519 } from '@noble/curves/ed25519.js';
import {
	loginViaApi,
	mockAuthEndpoints,
	PRIMARY_INSTALLATION_ID,
	B_PRIMARY_INSTALLATION_ID,
	seedTrustedPrimaryDevice,
	seedTrustedPrimaryDeviceB,
	setInstallationId,
} from './auth-helper.js';

/**
 * Direct Messages E2E tests.
 *
 * Single-user tests cover DM index rendering and navigation.
 * The seeded flow creates a deterministic conversation over the API first,
 * then signs in through the real UI and verifies the primary account can open
 * that conversation and send a DM.
 */

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';
const USER_A_INSTALLATION_ID = PRIMARY_INSTALLATION_ID;
const USER_B_INSTALLATION_ID = B_PRIMARY_INSTALLATION_ID;

interface SessionBootstrap {
	userId: string;
	username: string;
	displayName: string | null;
	accessToken: string;
	csrfToken: string;
}

interface KeyBundleV2Response {
	identity_sig_key: string;
	signed_prekey: string;
	signed_prekey_sig: string;
}

async function loginAs(page: Page) {
	await mockAuthEndpoints(page);
	await page.goto('/explore');
	await page.waitForURL(/\/explore/, { timeout: 20_000 });
}

async function loginFresh(page: Page, email: string, password: string, installationId: string) {
	await setInstallationId(page, installationId);
	await page.goto('/login');
	await page.fill('#email', email);
	await page.fill('#password', password);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 20_000 });
}

async function waitForAppReady(page: Page): Promise<void> {
	await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
		timeout: 45_000,
	});
}

function b64ToBytes(b64: string): Uint8Array {
	return Uint8Array.from(Buffer.from(b64, 'base64'));
}

function hasValidSignedPreKey(bundle: KeyBundleV2Response): boolean {
	return ed25519.verify(
		b64ToBytes(bundle.signed_prekey_sig),
		b64ToBytes(bundle.signed_prekey),
		b64ToBytes(bundle.identity_sig_key),
	);
}

async function waitForBundles(authToken: string, userId: string): Promise<void> {
	for (let attempt = 0; attempt < 45; attempt++) {
		const res = await fetch(`${API_URL}/api/v2/keys/${userId}/bundles`, {
			headers: { Authorization: `Bearer ${authToken}` },
		});
		if (res.ok) {
			const bundles = (await res.json()) as KeyBundleV2Response[];
			if (bundles.length > 0 && bundles.every(hasValidSignedPreKey)) return;
		}
		await new Promise<void>((resolve) => setTimeout(resolve, 1_000));
	}

	throw new Error(`Valid key bundles for user ${userId} not available after 45 s`);
}

async function createSessionBootstrap(
	email: string,
	password: string,
	installationId: string,
	label: string,
): Promise<SessionBootstrap> {
	const authData = await loginViaApi(email, password, {
		installationId,
		label,
	});
	const userId = authData.user.id;
	const username = authData.user.username;
	const displayName = authData.user.displayName ?? authData.user.display_name ?? null;

	if (typeof userId !== 'string' || typeof username !== 'string') {
		throw new Error('createSessionBootstrap response missing user identity fields');
	}

	return {
		userId,
		username,
		displayName: typeof displayName === 'string' ? displayName : null,
		accessToken: authData.accessToken,
		csrfToken: authData.csrfToken,
	};
}

function apiHeaders(session: SessionBootstrap) {
	return {
		'Content-Type': 'application/json',
		Authorization: `Bearer ${session.accessToken}`,
		Cookie: `csrf_token=${session.csrfToken}`,
		'X-CSRF-Token': session.csrfToken,
	};
}

async function createDmConversation(session: SessionBootstrap, peerId: string): Promise<string> {
	const response = await fetch(`${API_URL}/api/v1/conversations`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: JSON.stringify({ peer_id: peerId }),
	});

	if (!response.ok) {
		throw new Error(`createDmConversation failed: ${response.status} ${await response.text()}`);
	}

	const body = (await response.json()) as { id?: string };
	if (typeof body.id !== 'string') {
		throw new Error('createDmConversation response missing conversation id');
	}

	return body.id;
}

async function openConversation(
	page: Page,
	conversationId: string,
	peerLabels: string[],
): Promise<void> {
	const dmLink = page
		.getByRole('link', { name: /Direct Messages/i })
		.or(page.locator('a[href="/dm"]'));
	await dmLink.first().click();
	await page.waitForURL(/\/dm$/, { timeout: 10_000 });
	await expect(page.locator('button.conv-btn').first()).toBeVisible({ timeout: 15_000 });

	for (const label of peerLabels) {
		const button = page.locator('button.conv-btn', { hasText: label }).first();
		const visible = await button.isVisible({ timeout: 2_000 }).catch(() => false);
		if (!visible) {
			continue;
		}

		await button.click();
		await page.waitForURL(new RegExp(`/dm/${conversationId}$`), { timeout: 10_000 });
		return;
	}

	throw new Error(`Conversation ${conversationId} was not visible in the DM sidebar`);
}

// DM index page

test.describe('DM index - authenticated', () => {
	test.skip(!USER_A_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page);
	});

	test('/dm page renders', async ({ page }) => {
		await page.goto('/dm');
		await expect(page).toHaveURL('/dm');
		await expect(page.locator('body')).toBeVisible();
	});

	test('Direct Messages nav link is present in sidebar', async ({ page }) => {
		await page.goto('/explore');
		const dmLink = page
			.getByRole('link', { name: /Direct Messages/i })
			.or(page.locator('a[href="/dm"]'));
		await expect(dmLink.first()).toBeVisible();
	});

	test('sidebar shows DM section when on /dm', async ({ page }) => {
		await page.goto('/dm');
		await expect(page.locator('nav, aside, [class*="sidebar"]').first()).toBeVisible();
	});
});

// Seeded DM flow

test.describe('Seeded DM flow', () => {
	test.use({ storageState: { cookies: [], origins: [] } });
	// beforeEach does 2 API logins + browser login + waitForAppReady (up to 45s each).
	// test.setTimeout inside beforeEach only affects the test body, not the hook itself.
	// Set the timeout at describe scope so both the hook and test body use 120s.
	test.describe.configure({ timeout: 120_000 });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL, E2E_PASSWORD, E2E_EMAIL_2, E2E_PASSWORD_2 to run seeded DM tests',
	);

	let conversationId = '';
	let userBLabels: string[] = [];
	let userBId = '';
	let sessionAAccessToken = '';

	test.beforeAll(() => {
		// Only seed devices — B's Signal keys are uploaded within the test itself
		// so that the key upload context matches the read context.
		seedTrustedPrimaryDevice();
		if (USER_B_EMAIL && USER_B_PASS) {
			seedTrustedPrimaryDeviceB();
		}
	});

	test.beforeEach(async ({ page }) => {
		const sessionA = await createSessionBootstrap(
			USER_A_EMAIL,
			USER_A_PASS,
			USER_A_INSTALLATION_ID,
			'E2E Primary Browser',
		);
		const sessionB = await createSessionBootstrap(
			USER_B_EMAIL,
			USER_B_PASS,
			USER_B_INSTALLATION_ID,
			'E2E DM Browser B',
		);
		conversationId = await createDmConversation(sessionA, sessionB.userId);
		userBId = sessionB.userId;
		sessionAAccessToken = sessionA.accessToken;
		userBLabels = [sessionB.username, sessionB.displayName].filter(
			(label): label is string => typeof label === 'string' && label.length > 0,
		);

		await loginFresh(page, USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION_ID);
		await waitForAppReady(page);
	});

	test('User A can open the seeded DM with User B and send a message', async ({ page, browser }) => {
		// B login + Signal key bootstrap (~75 s) + A's navigation and send (~30 s)
		test.setTimeout(180_000);

		// Open the conversation while B logs in concurrently to upload their prekeys.
		// encryptDm(peerId) fetches B's key bundle from the server — B must have
		// uploaded keys before A calls input.press('Enter').
		const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const pageB = await ctxB.newPage();
		try {
			const bKeysDone = (async () => {
				if (!USER_B_EMAIL || !USER_B_PASS) return;
				await setInstallationId(pageB, USER_B_INSTALLATION_ID);
				await pageB.goto('/login');
				await pageB.fill('#email', USER_B_EMAIL);
				await pageB.fill('#password', USER_B_PASS);
				await pageB.getByRole('button', { name: /Sign In/i }).click();
				await pageB.waitForURL(/\/explore/, { timeout: 30_000 });
				await expect(pageB.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
					timeout: 45_000,
				});
			})();

		await openConversation(page, conversationId, userBLabels);

		// The seed script creates a secondary pending device for multi-device tests.
		// Approve it here so the Pending Device Approvals banner doesn't block E2EE chat.
		const approveBtn = page.getByRole('button', { name: /^Approve$/i }).first();
		if (await approveBtn.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await approveBtn.click();
			await page.waitForTimeout(1_500);
		}

		const input = page.locator('textarea[aria-label="Message"]').first();
		await expect(input).toBeVisible({ timeout: 20_000 });
		await expect(input).toBeEnabled({ timeout: 20_000 });

		// Wait for B's browser to finish loading (loading screen gone).
		// Then poll the API until B's key bundles are confirmed on the server —
		// the app's key-upload runs async after the loading screen hides, so we
		// need the hard API signal rather than the UI signal.
		await bKeysDone;
		await waitForBundles(sessionAAccessToken, userBId);

		const testMsg = `E2E DM test ${Date.now()}`;
		await input.fill(testMsg);
		await input.press('Enter');

			await expect(page.getByText(testMsg)).toBeVisible({ timeout: 20_000 });
		} finally {
			await ctxB.close();
		}
	});
});
