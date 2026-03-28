/**
 * Tauri Desktop - Keyboard Shortcuts
 *
 * @desktop @smoke
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

test.describe('Tauri keyboard shortcuts @desktop @smoke', () => {
	test.beforeEach(async ({}, testInfo) => {
		if (!process.env.TAURI_BINARY) {
			testInfo.skip(true, 'TAURI_BINARY not set — skipping Tauri-specific test');
		}
	});

	test('Ctrl+K opens command palette or search', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await userPage.keyboard.press('Control+k');

		const palette = userPage
			.locator('[data-testid="command-palette"], [aria-label*="command" i], [aria-label*="search" i]')
			.first();
		await expect(palette).toBeVisible({ timeout: 5_000 });
	});

	test('Ctrl+, opens settings', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await userPage.keyboard.press('Control+,');
		await expect(userPage).toHaveURL(/\/settings/, { timeout: 5_000 });
	});

	test('Escape closes open modal/panel', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await userPage.keyboard.press('Control+k');
		const palette = userPage.locator('[data-testid="command-palette"]').first();
		await expect(palette).toBeVisible({ timeout: 5_000 });

		await userPage.keyboard.press('Escape');
		await expect(palette).toHaveCount(0, { timeout: 3_000 });
	});
});
