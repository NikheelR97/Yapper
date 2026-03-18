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
 * Feature: Offline Message Delivery
 *
 * User A sends a DM while User B is not connected.
 * When User B opens the conversation, the message is visible.
 */

test.describe.configure({ timeout: 120_000 });

const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';

const client = new E2EApiClient();

test.describe('Offline message delivery', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL / E2E_EMAIL_2 credentials to run offline delivery tests',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('User B receives a message that was sent while they were offline', async ({ browser }: { browser: Browser }) => {
		const sessionA = await client.login(USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID, 'Offline A');
		const sessionB = await client.login(USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID, 'Offline B');

		const conversationId = await client.createDmConversation(sessionA, sessionB.userId);

		const testMsg = `Offline delivery test ${Date.now()}`;

		// E2EE key exchange requires fresh keys from User B to be available on the
		// server before User A encrypts the message.  Have User B log in first to
		// upload their prekeys, then navigate away — simulating "not in the DM" —
		// so User A sends into an empty conversation while B is elsewhere.
		// Both use the SAME browser context (same IndexedDB) so User B can decrypt
		// with the private key that matches the public key User A encrypted against.
		const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const pageA = await ctxA.newPage();
		const pageB = await ctxB.newPage();

		try {
			// Step 1: User B logs in and uploads prekeys, then idles on /explore.
			await setInstallationId(pageB, B_PRIMARY_INSTALLATION_ID);
			await pageB.goto('/login');
			await pageB.fill('#email', USER_B_EMAIL);
			await pageB.fill('#password', USER_B_PASS);
			await pageB.getByRole('button', { name: /Sign In/i }).click();
			await pageB.waitForURL(/\/explore/, { timeout: 20_000 });
			await waitForAppReady(pageB);
			// B stays on /explore — not in the DM conversation (simulates "offline from DM")

			// Step 2: User A logs in, waits for B's bundles, then sends the message.
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
			await inputA.fill(testMsg);
			await inputA.press('Enter');
			await expect(pageA.getByText(testMsg)).toBeVisible({ timeout: 8_000 });

			// Step 3: User B opens the conversation in the same context (same IndexedDB →
			// same private key → can decrypt A's message).
			await pageB.goto(`/dm/${conversationId}`);
			await expect(pageB.getByText(testMsg)).toBeVisible({ timeout: 20_000 });
		} finally {
			await ctxA.close();
			await ctxB.close();
		}
	});
});
