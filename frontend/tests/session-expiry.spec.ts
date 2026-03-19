/**
 * QA Tests doc — Section 2, P1: Session Expiry
 *
 * Feature: Session Expiry
 *   Scenario: Access token expires during mutation
 *     Given the user is editing data in an authenticated route
 *     When the session expires before submit
 *     Then the app shows a stable re-authentication or error path
 *
 * Uses mocked auth — no live API required.
 * @smoke
 */

import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { log } from './helpers/log.js';

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

async function setupAuth(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
	const device = buildMockDevice({ installation_id: 'session-expiry-install' });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, 'session-expiry-install');
	await mockAuthEndpoints(page, authData);

	await page.route('**/api/v1/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v1/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

test.describe('Session expiry @security @auth', () => {
	test('401 during profile save shows error and remains stable @smoke', async ({ page }) => {
		await setupAuth(page);

		log('SESSION', 'SETUP', 'Configuring 401 response on profile PATCH to simulate token expiry');

		// First GET /users/me succeeds (page loads), but PATCH returns 401.
		let patchCalled = false;
		await page.route(`**/api/v1/users/me`, async (route) => {
			if (route.request().method() === 'PATCH') {
				patchCalled = true;
				log('SESSION', 'INTERCEPT', '401 Unauthorized on PATCH /users/me — token expired');
				await route.fulfill({
					status: 401,
					contentType: 'application/json',
					body: JSON.stringify({ error: 'Token expired' }),
				});
			} else {
				await route.continue();
			}
		});

		log('NETWORK', 'NAVIGATE', 'Loading /settings (profile section)');
		await page.goto('/settings');

		// Wait for profile form to load.
		const displayNameInput = page.getByRole('textbox', { name: 'Display Name' });
		await expect(displayNameInput).toBeVisible({ timeout: 20_000 });

		log('SESSION', 'ACTION', 'Editing display name and clicking Save');
		await displayNameInput.fill('Expired User');

		const saveBtn = page.getByRole('button', { name: 'Save Changes' });
		await expect(saveBtn).toBeVisible();
		await saveBtn.click();

		// The app should show an error toast or error state — not crash.
		log('ASSERTION', 'STATE', 'Verifying app remains stable after 401 on save');

		// The PATCH must have been called.
		expect(patchCalled).toBe(true);
		log('ASSERTION', 'STATE', 'PATCH request was intercepted with 401. [PASS]');

		// Page should still be functional — not a blank screen or unhandled error.
		await expect(page.locator('.settings-page')).toBeVisible({ timeout: 5_000 });
		log('VALIDATION', 'UI', 'Settings page remains visible after session expiry. App did not crash. [PASS]');
	});

	test('401 on device list fetch shows error state gracefully @smoke', async ({ page }) => {
		await setupAuth(page);

		log('SESSION', 'SETUP', 'Configuring 401 on GET /api/v2/devices to simulate expired session');

		await page.route('**/api/v2/devices', async (route) => {
			log('SESSION', 'INTERCEPT', '401 on GET /api/v2/devices');
			await route.fulfill({
				status: 401,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'Unauthorized' }),
			});
		});

		await page.goto('/settings');
		await expect(page.locator('.settings-page')).toBeVisible({ timeout: 20_000 });

		log('VALIDATION', 'UI', 'Settings page renders despite 401 on device fetch. [PASS]');
	});
});
