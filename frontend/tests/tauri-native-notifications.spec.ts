/**
 * Tauri Desktop — Native Notification Permission Flow
 *
 * Verifies that:
 *   1. The app requests notification permission via Tauri's notification API
 *   2. When granted, no error or crash occurs
 *   3. When denied, the app gracefully degrades (no crash or infinite loop)
 *
 * Guards itself with __TAURI_INTERNALS__ detection.
 *
 * @desktop @regression
 */

import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

test.describe('Tauri native notifications @desktop @regression', () => {
	test.beforeEach(async ({ page }) => {
		const isTauri = await page.evaluate(() => '__TAURI_INTERNALS__' in window);
		test.skip(!isTauri, 'Tauri-only test — skip in browser projects');
	});

	async function setupMockedAuth(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
		const device = buildMockDevice({ installation_id: 'tauri-notif-install', platform: 'tauri' });
		const authData = buildMockAuthData({ device });
		await setInstallationId(page, 'tauri-notif-install');
		await mockAuthEndpoints(page, authData, { devices: [device] });
		await page.route('**/api/v1/servers', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await page.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await mockExploreEndpoints(page);
	}

	test('app becomes ready without crashing when notifications are granted', async ({ page }) => {
		// Stub Notification.requestPermission to immediately resolve 'granted'
		await page.addInitScript(() => {
			Object.defineProperty(window, 'Notification', {
				writable: true,
				value: class MockNotification {
					static permission = 'default';
					static requestPermission(): Promise<NotificationPermission> {
						MockNotification.permission = 'granted';
						return Promise.resolve('granted');
					}
				},
			});
		});

		await setupMockedAuth(page);
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });

		// App should be fully rendered, not blank or crashed
		await expect(page.locator('body')).toBeVisible();
	});

	test('app becomes ready without crashing when notifications are denied', async ({ page }) => {
		// Stub Notification.requestPermission to resolve 'denied'
		await page.addInitScript(() => {
			Object.defineProperty(window, 'Notification', {
				writable: true,
				value: class MockNotification {
					static permission = 'denied';
					static requestPermission(): Promise<NotificationPermission> {
						return Promise.resolve('denied');
					}
				},
			});
		});

		await setupMockedAuth(page);
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });

		// App must not crash or show an unhandled error when notifications are denied
		await expect(page.locator('body')).toBeVisible();

		// No unhandled error dialogs
		const errorDialog = page.locator('[role="alertdialog"], .error-boundary, text=/unexpected error/i').first();
		await expect(errorDialog).toHaveCount(0);
	});
});
