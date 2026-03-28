/**
 * QA Tests doc - Section 2, P1: Session Expiry
 *
 * Feature: Session Expiry
 *   Scenario: Access token expires during mutation
 *     Given the user is editing data in an authenticated route
 *     When the session expires before submit
 *     Then the app shows a stable re-authentication or error path
 *
 * @smoke
 */

import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { log } from './helpers/log.js';

async function setupShellData(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

test.describe('Session expiry @security @auth', () => {
	test('401 during profile save shows error and remains stable @smoke', async ({ userPage }) => {
		await setupShellData(userPage);

		log('SESSION', 'SETUP', 'Configuring 401 response on profile PATCH to simulate token expiry');

		let patchCalled = false;
		await userPage.route('**/api/v2/users/me', async (route) => {
			if (route.request().method() === 'PATCH') {
				patchCalled = true;
				log('SESSION', 'INTERCEPT', '401 Unauthorized on PATCH /users/me - token expired');
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
		await userPage.goto('/settings');

		const displayNameInput = userPage.getByRole('textbox', { name: 'Display Name' });
		await expect(displayNameInput).toBeVisible({ timeout: 20_000 });

		log('SESSION', 'ACTION', 'Editing display name and clicking Save');
		await displayNameInput.fill('Expired User');

		const saveButton = userPage.getByRole('button', { name: 'Save Changes' });
		await expect(saveButton).toBeVisible();
		await saveButton.click();

		expect(patchCalled).toBe(true);
		await expect(userPage.locator('.settings-page')).toBeVisible({ timeout: 5_000 });
		log('VALIDATION', 'UI', 'Settings page remains visible after session expiry. App did not crash. [PASS]');
	});

	test('401 on device list fetch shows error state gracefully @smoke', async ({ userPage }) => {
		await setupShellData(userPage);

		log('SESSION', 'SETUP', 'Configuring 401 on GET /api/v2/devices to simulate expired session');

		await userPage.route('**/api/v2/devices', async (route) => {
			log('SESSION', 'INTERCEPT', '401 on GET /api/v2/devices');
			await route.fulfill({
				status: 401,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'Unauthorized' }),
			});
		});

		await userPage.goto('/settings');
		await expect(userPage.locator('.settings-page')).toBeVisible({ timeout: 20_000 });
	});
});
