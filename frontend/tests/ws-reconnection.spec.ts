import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

/**
 * Feature: WebSocket Reconnection Banner
 */

async function setupShellData(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);
}

test.describe('WebSocket reconnection banner', () => {
	test('reconnecting banner appears when WebSocket connection is lost @smoke', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await expect(userPage.locator('.reconnecting-banner')).toBeVisible({ timeout: 15_000 });
	});

	test('app shell renders normally on initial load @smoke', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await expect(userPage.locator('body')).toBeVisible();
	});
});
