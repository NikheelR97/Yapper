import { test, expect, type Browser } from '@playwright/test';
import {
	setInstallationId,
	seedTrustedPrimaryDevice,
	seedTrustedPrimaryDeviceB,
	PRIMARY_INSTALLATION_ID,
	B_PRIMARY_INSTALLATION_ID,
} from './auth-helper.js';
import { E2EApiClient } from './helpers/api-client.js';
import { waitForAppReady, waitForKeyBundles, navigateClientSide } from './helpers/wait-for.js';

/**
 * Feature: Read Receipts
 *
 * Two-user live test: User A sends DM -> User B opens it -> read receipt fires.
 */

test.describe.configure({ timeout: 180_000 });

const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';

const client = new E2EApiClient();

async function loginAndBootstrapSignal(
	page: Awaited<ReturnType<Browser['newPage']>>,
	email: string,
	password: string,
	installationId: string,
) {
	const keyUploads = [
		page.waitForResponse(
			(response) =>
				response.request().method() === 'POST' &&
				response.url().includes('/api/v2/keys/identity') &&
				response.ok(),
		),
		page.waitForResponse(
			(response) =>
				response.request().method() === 'POST' &&
				response.url().includes('/api/v2/keys/signed-prekey') &&
				response.ok(),
		),
		page.waitForResponse(
			(response) =>
				response.request().method() === 'POST' &&
				response.url().includes('/api/v2/keys/one-time-prekeys') &&
				response.ok(),
		),
	];

	await setInstallationId(page, installationId);
	await page.goto('/login');
	await page.fill('#email', email);
	await page.fill('#password', password);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 20_000 });
	await waitForAppReady(page);
	await Promise.all(keyUploads);
}

test.describe('Read receipts - DM', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL / E2E_EMAIL_2 credentials to run read receipt tests',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('User A sees a read receipt after User B opens the conversation', async ({ browser }: { browser: Browser }) => {
		const sessionA = await client.login(USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID, 'RR A');
		const sessionB = await client.login(USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID, 'RR B');

		const conversationId = await client.createDmConversation(sessionA, sessionB.userId);

		const ctxA = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const ctxB = await browser.newContext({ storageState: { cookies: [], origins: [] } });
		const pageA = await ctxA.newPage();
		const pageB = await ctxB.newPage();

		try {
			await loginAndBootstrapSignal(pageB, USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID);
			await loginAndBootstrapSignal(pageA, USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID);

			await Promise.all([
				waitForKeyBundles(sessionA.accessToken, sessionA.userId),
				waitForKeyBundles(sessionA.accessToken, sessionB.userId),
			]);

			await navigateClientSide(pageA, `/dm/${conversationId}`);
			const inputA = pageA.locator('textarea[aria-label="Message"]').first();
			await expect(inputA).toBeEnabled({ timeout: 60_000 });

			const testMsg = `RR Test ${Date.now()}`;
			await inputA.fill(testMsg);
			await inputA.press('Enter');
			await expect(pageA.getByTestId('dm-message-list')).toContainText(testMsg, { timeout: 10_000 });

			await navigateClientSide(pageB, `/dm/${conversationId}`);
			await expect(pageB.getByTestId('dm-message-list')).toContainText(testMsg, { timeout: 10_000 });

			await expect(pageA.getByTestId('read-receipt-indicator')).toBeVisible({ timeout: 8_000 });
		} finally {
			await ctxA.close();
			await ctxB.close();
		}
	});
});
