import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { buildMockDevice, PRIMARY_INSTALLATION_ID } from './auth-helper.js';

test.setTimeout(120_000);

async function setupShellRoutes(page: Page): Promise<void> {
	const installationId = PRIMARY_INSTALLATION_ID;
	const currentDevice = buildMockDevice({
		id: 'auth-shell-primary-device',
		installation_id: installationId,
		label: 'Primary Browser',
	});
	const otherDevice = buildMockDevice({
		id: 'remote-auth-shell-device',
		signal_device_id: 2,
		installation_id: 'remote-auth-shell-installation',
		label: 'Desk Laptop',
	});

	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/devices', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([currentDevice, otherDevice]),
		});
	});
}

test.describe('Authenticated shell', () => {
	test('boots explore, covers settings sections, survives reload, and logs out @smoke', async ({ userPage }) => {
		await setupShellRoutes(userPage);
		await userPage.route('**/api/v2/account/data-export', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/zip',
				body: 'e2e-export',
			});
		});
		await userPage.route('**/api/v2/auth/logout', async (route) => {
			await route.fulfill({ status: 204, body: '' });
		});

		await userPage.goto('/explore');
		await expect(userPage.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 30_000,
		});

		await userPage.getByRole('link', { name: 'Settings' }).click();
		await expect(userPage).toHaveURL(/\/settings/, { timeout: 20_000 });
		await expect(userPage.getByRole('button', { name: 'My Profile' })).toBeVisible({
			timeout: 20_000,
		});
		await expect(userPage.getByRole('button', { name: 'Privacy & Safety' })).toBeVisible({
			timeout: 20_000,
		});
		await expect(userPage.getByRole('button', { name: 'Notifications' })).toBeVisible({
			timeout: 20_000,
		});

		await userPage.getByRole('button', { name: /Export My Data/i }).click();
		await expect(userPage.getByText(/Data export downloaded\./i)).toBeVisible({
			timeout: 10_000,
		});

		await userPage.getByRole('button', { name: 'Privacy & Safety' }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Privacy & Safety', exact: true }),
		).toBeVisible({ timeout: 20_000 });

		await userPage.getByRole('button', { name: 'Notifications' }).click();
		await expect(userPage.getByRole('heading', { name: 'Notifications', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await userPage.getByRole('button', { name: 'Appearance' }).click();
		await expect(userPage.getByRole('heading', { name: 'Appearance', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await userPage.getByRole('button', { name: 'Voice & Video' }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Voice & Video', exact: true }),
		).toBeVisible({ timeout: 20_000 });

		await userPage.getByRole('button', { name: /Yapper Premium/i }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Yapper Premium', exact: true }),
		).toBeVisible({ timeout: 20_000 });

		await userPage.getByRole('button', { name: 'Connected Accounts' }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Connected Accounts', exact: true }),
		).toBeVisible({ timeout: 20_000 });

		await userPage.getByRole('button', { name: 'For Developers' }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Yapper for Developers', exact: true }),
		).toBeVisible({ timeout: 20_000 });

		await userPage.getByRole('button', { name: 'Change Password' }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Change Password', exact: true }),
		).toBeVisible({ timeout: 20_000 });

		await userPage.getByRole('button', { name: /Delete Account/i }).click();
		await expect(userPage.getByText('This is permanent and cannot be undone.')).toBeVisible({
			timeout: 10_000,
		});
		await userPage.getByRole('button', { name: 'Cancel' }).click();
		await expect(userPage.getByRole('button', { name: /Delete Account/i })).toBeVisible({
			timeout: 10_000,
		});

		await userPage.getByRole('link', { name: 'Explore' }).click();
		await expect(userPage).toHaveURL(/\/explore/, { timeout: 20_000 });
		await expect(userPage.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 30_000,
		});

		await userPage.reload();
		await expect(userPage).toHaveURL(/\/explore/, { timeout: 20_000 });
		await expect(userPage.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 30_000,
		});

		await userPage.getByRole('link', { name: 'Settings' }).click();
		await expect(userPage).toHaveURL(/\/settings/, { timeout: 20_000 });
		await expect(userPage.getByRole('button', { name: /Log Out/i })).toBeVisible({
			timeout: 20_000,
		});
		await userPage.getByRole('button', { name: /Log Out/i }).click();

		await expect(userPage).toHaveURL(/\/login/, { timeout: 10_000 });
		await expect(userPage.getByRole('button', { name: /Sign In/i })).toBeVisible({
			timeout: 10_000,
		});
	});
});
