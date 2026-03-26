/**
 * Tauri Desktop — Keyboard Shortcuts
 *
 * Verifies that Ctrl+K (command palette), Ctrl+, (settings), and Escape
 * (close modal) work correctly in the Tauri desktop app.
 *
 * Guards itself with __TAURI_INTERNALS__ detection.
 *
 * @desktop @smoke
 */

import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

test.describe('Tauri keyboard shortcuts @desktop @smoke', () => {
	test.beforeEach(async ({ page }) => {
		const isTauri = await page.evaluate(() => '__TAURI_INTERNALS__' in window);
		test.skip(!isTauri, 'Tauri-only test — skip in browser projects');
	});

	async function setupMockedAuth(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
		const device = buildMockDevice({ installation_id: 'tauri-kb-install', platform: 'tauri' });
		const authData = buildMockAuthData({ device });
		await setInstallationId(page, 'tauri-kb-install');
		await mockAuthEndpoints(page, authData, { devices: [device] });
		await page.route('**/api/v2/servers', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await page.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await mockExploreEndpoints(page);
	}

	test('Ctrl+K opens command palette or search', async ({ page }) => {
		await setupMockedAuth(page);
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });

		await page.keyboard.press('Control+k');
		await page.waitForTimeout(300);

		// Command palette or quick-search should appear
		const palette = page.locator(
			'[data-testid="command-palette"], [aria-label*="command" i], [aria-label*="search" i], input[placeholder*="Search" i]',
		).first();
		await expect(palette).toBeVisible({ timeout: 5_000 });
	});

	test('Ctrl+, opens settings', async ({ page }) => {
		await setupMockedAuth(page);
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });

		await page.keyboard.press('Control+,');
		await page.waitForTimeout(300);

		// Settings panel should be visible
		await expect(page).toHaveURL(/\/settings/, { timeout: 5_000 });
	});

	test('Escape closes open modal/panel', async ({ page }) => {
		await setupMockedAuth(page);
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });

		// Open something first (command palette via Ctrl+K)
		await page.keyboard.press('Control+k');
		await page.waitForTimeout(300);

		// Escape should close it
		await page.keyboard.press('Escape');
		await page.waitForTimeout(300);

		const palette = page.locator('[data-testid="command-palette"]').first();
		await expect(palette).toHaveCount(0, { timeout: 3_000 });
	});
});
