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
 * Feature: Offline Message Delivery
 *
 * User A sends a DM while User B is not connected.
 * When User B opens the conversation, the message is visible.
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

		const bLogs: string[] = [];
		pageB.on('console', msg => bLogs.push(`[B][${msg.type()}] ${msg.text()}`));
		pageB.on('pageerror', err => bLogs.push(`[B][pageerror] ${err.message}`));
		const aLogs: string[] = [];
		pageA.on('console', msg => aLogs.push(`[A][${msg.type()}] ${msg.text()}`));
		pageA.on('pageerror', err => aLogs.push(`[A][pageerror] ${err.message}`));
		pageA.on('response', resp => {
			if (resp.url().includes('/api/v2/keys') || resp.status() >= 400) {
				const msg = `[A][response] ${resp.status()} ${resp.url()}`;
				aLogs.push(msg);
				console.log(msg);  // immediate output for timing analysis
			}
		});

		try {
			// Step 1: User B logs in and uploads prekeys, then idles on /explore.
			await loginAndBootstrapSignal(pageB, USER_B_EMAIL, USER_B_PASS, B_PRIMARY_INSTALLATION_ID);
			await ctxB.setOffline(true);
			// B stays on /explore — not in the DM conversation (simulates "offline from DM")

			// Step 2: User A logs in.
			await loginAndBootstrapSignal(pageA, USER_A_EMAIL, USER_A_PASS, PRIMARY_INSTALLATION_ID);

			// Wait for BOTH users' key bundles to land on the server before A
			// sends. initializeSignalKeys() runs as a background promise after the
			// loading screen hides, so encryptDm() can race with the key upload.
			await Promise.all([
				waitForKeyBundles(sessionA.accessToken, sessionA.userId),
				waitForKeyBundles(sessionA.accessToken, sessionB.userId),
			]);

			await navigateClientSide(pageA, `/dm/${conversationId}`);
			const inputA = pageA.locator('textarea[aria-label="Message"]').first();
			await expect(inputA).toBeEnabled({ timeout: 60_000 });
			await inputA.fill(testMsg);
			await inputA.press('Enter');
			for (const log of aLogs.splice(0)) console.log(log);
			await expect(pageA.getByTestId('dm-message-list')).toContainText(testMsg, { timeout: 10_000 });

			// Step 3: User B opens the conversation in the same context (same IndexedDB →
			// same private key → can decrypt A's message).
			await ctxB.setOffline(false);
			for (const log of aLogs.splice(0)) console.log(log);
			console.log('[test] Navigating B to DM');
			await navigateClientSide(pageB, `/dm/${conversationId}`);
			for (const log of bLogs.splice(0)) console.log(log);
			await expect(pageB.getByTestId('dm-message-list')).toContainText(testMsg, { timeout: 15_000 });
			for (const log of bLogs.splice(0)) console.log(log);
		} finally {
			await ctxA.close();
			await ctxB.close();
		}
	});
});
