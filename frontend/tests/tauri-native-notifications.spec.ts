/**
 * Tauri Desktop - Native Notification Permission Flow
 *
 * @desktop @regression
 */

import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

async function setupShellData(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);
}

test.describe('Tauri native notifications @desktop @regression', () => {
	test.beforeEach(async ({}, testInfo) => {
		if (!process.env.TAURI_BINARY) {
			testInfo.skip(true, 'TAURI_BINARY not set — skipping Tauri-specific test');
		}
	});

	test('app becomes ready without crashing when notifications are granted', async ({ userPage }) => {
		await userPage.addInitScript(() => {
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

		await setupShellData(userPage);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await expect(userPage.locator('body')).toBeVisible();
	});

	test('app becomes ready without crashing when notifications are denied', async ({ userPage }) => {
		await userPage.addInitScript(() => {
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

		await setupShellData(userPage);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await expect(userPage.locator('body')).toBeVisible();

		const errorDialog = userPage
			.locator('[role="alertdialog"], .error-boundary, text=/unexpected error/i')
			.first();
		await expect(errorDialog).toHaveCount(0);
	});
});
