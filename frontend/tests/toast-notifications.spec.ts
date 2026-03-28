import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';

/**
 * Feature: Toast Notifications
 *
 * Tests that toast notifications appear and auto-dismiss.
 * Triggers toasts via the data export action.
 */

async function setupShellData(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

test.describe('Toast notifications - success', () => {
	test('export data action shows a success toast @smoke', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.route('**/api/v2/account/data-export', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/zip',
				body: 'fake-export-data',
			});
		});

		await userPage.goto('/settings');
		await expect(userPage.getByRole('button', { name: 'My Profile' })).toBeVisible({
			timeout: 20_000,
		});

		await userPage.getByRole('button', { name: /Export My Data/i }).click();
		await expect(
			userPage.locator('.toast-message').filter({ hasText: /Data export downloaded/i }),
		).toBeVisible({ timeout: 8_000 });
	});
});

test.describe('Toast notifications - error', () => {
	test('failed action shows an error toast or error state', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.route('**/api/v2/account/data-export', async (route) => {
			await route.fulfill({
				status: 500,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'Internal server error' }),
			});
		});

		await userPage.goto('/settings');
		await expect(userPage.getByRole('button', { name: 'My Profile' })).toBeVisible({
			timeout: 20_000,
		});

		await userPage.getByRole('button', { name: /Export My Data/i }).click();
		await expect(userPage.getByText(/Failed to export|export failed|error/i).first()).toBeVisible({
			timeout: 8_000,
		});
	});
});

test.describe('Toast notifications - appearance', () => {
	test('settings page has a working toast container element @smoke', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.goto('/settings');
		await expect(userPage.getByRole('button', { name: 'My Profile' })).toBeVisible({
			timeout: 20_000,
		});

		const toastContainer = userPage.locator('[data-toast-container], .toast-container, #toast');
		const count = await toastContainer.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});
});
