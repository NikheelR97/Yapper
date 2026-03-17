import { test, expect } from '@playwright/test';
import {
	buildMockAuthData,
	buildMockDevice,
	mockAuthEndpoints,
	setInstallationId,
} from './auth-helper.js';

test.setTimeout(120_000);

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

test.describe('Authenticated shell', () => {
	test('boots explore, covers settings sections, survives reload, and logs out', async ({ page }) => {
		const currentDevice = buildMockDevice({
			id: 'auth-shell-primary-device',
			installation_id: 'auth-shell-primary-installation',
			label: 'Primary Browser',
		});
		const deviceAwareAuthData = buildMockAuthData({ device: currentDevice });
		const otherDevice = buildMockDevice({
			id: 'remote-auth-shell-device',
			signal_device_id: 2,
			installation_id: 'remote-auth-shell-installation',
			label: 'Desk Laptop',
		});

		await setInstallationId(page, currentDevice.installation_id ?? 'auth-shell-primary-installation');
		await mockAuthEndpoints(page, deviceAwareAuthData, {
			devices: [currentDevice, otherDevice],
		});
		await page.route(`${API_URL}/api/v1/account/data-export`, async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/zip',
				body: 'e2e-export',
			});
		});
		await page.route(`${API_URL}/api/v2/auth/logout`, async (route) => {
			await route.fulfill({ status: 204, body: '' });
		});

		await page.goto('/explore');
		await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 30_000,
		});

		await page.getByRole('link', { name: 'Settings' }).click();
		await expect(page).toHaveURL(/\/settings/, { timeout: 20_000 });
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({
			timeout: 20_000,
		});
		await expect(page.getByRole('button', { name: 'Privacy & Safety' })).toBeVisible({
			timeout: 20_000,
		});
		await expect(page.getByRole('button', { name: 'Notifications' })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: /Export My Data/i }).click();
		await expect(page.getByText(/Data export downloaded\./i)).toBeVisible({ timeout: 10_000 });

		await page.getByRole('button', { name: 'Privacy & Safety' }).click();
		await expect(page.getByRole('heading', { name: 'Privacy & Safety', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: 'Notifications' }).click();
		await expect(page.getByRole('heading', { name: 'Notifications', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: 'Appearance' }).click();
		await expect(page.getByRole('heading', { name: 'Appearance', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: 'Voice & Video' }).click();
		await expect(page.getByRole('heading', { name: 'Voice & Video', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: /Yapper Premium/i }).click();
		await expect(page.getByRole('heading', { name: 'Yapper Premium', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: 'Connected Accounts' }).click();
		await expect(page.getByRole('heading', { name: 'Connected Accounts', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: 'For Developers' }).click();
		await expect(page.getByRole('heading', { name: 'Yapper for Developers', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: 'Change Password' }).click();
		await expect(page.getByRole('heading', { name: 'Change Password', exact: true })).toBeVisible({
			timeout: 20_000,
		});

		await page.getByRole('button', { name: /Delete Account/i }).click();
		await expect(page.getByText('This is permanent and cannot be undone.')).toBeVisible({
			timeout: 10_000,
		});
		await page.getByRole('button', { name: 'Cancel' }).click();
		await expect(page.getByRole('button', { name: /Delete Account/i })).toBeVisible({
			timeout: 10_000,
		});

		await page.getByRole('link', { name: 'Explore' }).click();
		await expect(page).toHaveURL(/\/explore/, { timeout: 20_000 });
		await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 30_000,
		});

		await page.reload();
		await expect(page).toHaveURL(/\/explore/, { timeout: 20_000 });
		await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 30_000,
		});

		await page.getByRole('link', { name: 'Settings' }).click();
		await expect(page).toHaveURL(/\/settings/, { timeout: 20_000 });
		await expect(page.getByRole('button', { name: /Log Out/i })).toBeVisible({
			timeout: 20_000,
		});
		await page.getByRole('button', { name: /Log Out/i }).click();

		await expect(page).toHaveURL(/\/login/, { timeout: 10_000 });
		await expect(page.getByRole('button', { name: /Sign In/i })).toBeVisible({ timeout: 10_000 });
	});
});
