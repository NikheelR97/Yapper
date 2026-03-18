import { test, expect, type Browser } from '@playwright/test';
import {
	setInstallationId,
	seedTrustedPrimaryDevice,
	seedTrustedPrimaryDeviceB,
	PRIMARY_INSTALLATION_ID,
	B_PRIMARY_INSTALLATION_ID,
} from './auth-helper.js';
import { E2EApiClient } from './helpers/api-client.js';
import { waitForAppReady } from './helpers/wait-for.js';

/**
 * Feature: Typing Indicators
 *
 * Two-user live test: User A types → User B sees indicator.
 * Requires E2E_EMAIL + E2E_EMAIL_2 credentials.
 */

test.describe.configure({ timeout: 120_000 });

const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';

const client = new E2EApiClient();

test.describe('Typing indicators — channel', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL / E2E_EMAIL_2 credentials to run typing indicator tests',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('User A typing causes a typing indicator to appear for User B', async ({ browser }: { browser: Browser }) => {
		const sessionA = await client.login(USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID, 'Typing A');
		const sessionB = await client.login(USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID, 'Typing B');

		// Create a shared server so both users are in the same channel
		const { serverId, channelId } = await client.createServer(sessionA, `E2E Typing ${Date.now()}`);
		const inviteCode = await client.createInvite(sessionA, serverId);
		await client.joinByInvite(sessionB, inviteCode);

		// Open contexts for both users
		const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const pageA = await ctxA.newPage();
		const pageB = await ctxB.newPage();

		try {
			// Log in User A
			await setInstallationId(pageA, PRIMARY_INSTALLATION_ID);
			await pageA.goto('/login');
			await pageA.fill('#email', USER_A_EMAIL);
			await pageA.fill('#password', USER_A_PASS);
			await pageA.getByRole('button', { name: /Sign In/i }).click();
			await pageA.waitForURL(/\/explore/, { timeout: 20_000 });
			await waitForAppReady(pageA);
			await pageA.goto(`/servers/${serverId}/channels/${channelId}`);
			const inputA = pageA.locator('textarea[aria-label="Message"]').first();
			await expect(inputA).toBeEnabled({ timeout: 60_000 });

			// Log in User B
			await setInstallationId(pageB, B_PRIMARY_INSTALLATION_ID);
			await pageB.goto('/login');
			await pageB.fill('#email', USER_B_EMAIL);
			await pageB.fill('#password', USER_B_PASS);
			await pageB.getByRole('button', { name: /Sign In/i }).click();
			await pageB.waitForURL(/\/explore/, { timeout: 20_000 });
			await waitForAppReady(pageB);
			await pageB.goto(`/servers/${serverId}/channels/${channelId}`);
			const inputB = pageB.locator('textarea[aria-label="Message"]').first();
			await expect(inputB).toBeEnabled({ timeout: 60_000 });

			// User A starts typing
			await inputA.fill('Hello from User A...');

			// User B should see a typing indicator
			await expect(
				pageB.locator('.typing, .typing-indicator, [data-testid="typing-indicator"]'),
			).toBeVisible({ timeout: 8_000 });

			// After User A clears the input and waits, indicator should disappear
			await inputA.clear();
			await expect(
				pageB.locator('.typing, .typing-indicator, [data-testid="typing-indicator"]'),
			).toHaveCount(0, { timeout: 10_000 });
		} finally {
			await ctxA.close();
			await ctxB.close();
		}
	});
});

test.describe('Typing indicators — DM', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL / E2E_EMAIL_2 credentials to run DM typing indicator tests',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('User A typing in DM triggers typing indicator for User B', async ({ browser }: { browser: Browser }) => {
		const sessionA = await client.login(USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID, 'DM Typing A');
		const sessionB = await client.login(USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID, 'DM Typing B');

		const conversationId = await client.createDmConversation(sessionA, sessionB.userId);

		const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const pageA = await ctxA.newPage();
		const pageB = await ctxB.newPage();

		try {
			// Log in both users
			await setInstallationId(pageA, PRIMARY_INSTALLATION_ID);
			await pageA.goto('/login');
			await pageA.fill('#email', USER_A_EMAIL);
			await pageA.fill('#password', USER_A_PASS);
			await pageA.getByRole('button', { name: /Sign In/i }).click();
			await pageA.waitForURL(/\/explore/, { timeout: 20_000 });
			await waitForAppReady(pageA);
			await pageA.goto(`/dm/${conversationId}`);
			const inputA = pageA.locator('textarea[aria-label="Message"]').first();
			await expect(inputA).toBeEnabled({ timeout: 60_000 });

			await setInstallationId(pageB, B_PRIMARY_INSTALLATION_ID);
			await pageB.goto('/login');
			await pageB.fill('#email', USER_B_EMAIL);
			await pageB.fill('#password', USER_B_PASS);
			await pageB.getByRole('button', { name: /Sign In/i }).click();
			await pageB.waitForURL(/\/explore/, { timeout: 20_000 });
			await waitForAppReady(pageB);
			await pageB.goto(`/dm/${conversationId}`);
			const inputB = pageB.locator('textarea[aria-label="Message"]').first();
			await expect(inputB).toBeEnabled({ timeout: 60_000 });

			// User A starts typing
			await inputA.fill('typing in DM...');

			// User B should see typing indicator
			await expect(
				pageB.locator('.typing, .typing-indicator, [data-testid="typing-indicator"]'),
			).toBeVisible({ timeout: 8_000 });
		} finally {
			await ctxA.close();
			await ctxB.close();
		}
	});
});
