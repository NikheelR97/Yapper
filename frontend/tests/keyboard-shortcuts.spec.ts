import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';

/**
 * Feature: Keyboard Shortcuts
 *
 * Tests that Ctrl+/ opens the keyboard shortcuts modal
 * and pressing it again (or Escape) closes it.
 */

async function setupAndNavigate(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
	const device = buildMockDevice({ installation_id: 'shortcuts-test-install' });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, 'shortcuts-test-install');
	await mockAuthEndpoints(page, authData);
	await page.route(`**/api/v1/servers`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route(`**/api/v1/conversations`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route(`**/api/v1/explore/**`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});

	await page.goto('/explore');
	await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });
	// Ensure the page has focus so keyboard events are received
	await page.locator('body').click();
}

test.describe('Keyboard shortcuts modal', () => {
	// Use clean storage so mocked auth isn't polluted by the global live-auth state.
	test.use({ storageState: { cookies: [], origins: [] } });

	test('Ctrl+/ opens the keyboard shortcuts modal @smoke', async ({ page }) => {
		await setupAndNavigate(page);

		await page.keyboard.press('Control+/');

		// The modal should open
		const modal = page.locator('[role="dialog"]').filter({ hasText: /shortcut|keyboard/i });
		await expect(modal).toBeVisible({ timeout: 5_000 });
	});

	test('pressing Escape closes the keyboard shortcuts modal @smoke', async ({ page }) => {
		await setupAndNavigate(page);

		await page.keyboard.press('Control+/');
		const modal = page.locator('[role="dialog"]').filter({ hasText: /shortcut|keyboard/i });
		await expect(modal).toBeVisible({ timeout: 5_000 });

		await page.keyboard.press('Escape');
		await expect(modal).toHaveCount(0, { timeout: 5_000 });
	});

	test('pressing Ctrl+/ a second time closes the modal', async ({ page }) => {
		await setupAndNavigate(page);

		await page.keyboard.press('Control+/');
		const modal = page.locator('[role="dialog"]').filter({ hasText: /shortcut|keyboard/i });
		await expect(modal).toBeVisible({ timeout: 5_000 });

		await page.keyboard.press('Control+/');
		await expect(modal).toHaveCount(0, { timeout: 5_000 });
	});

	test('modal lists at least some keyboard shortcuts', async ({ page }) => {
		await setupAndNavigate(page);

		await page.keyboard.press('Control+/');
		const modal = page.locator('[role="dialog"]').filter({ hasText: /shortcut|keyboard/i });
		await expect(modal).toBeVisible({ timeout: 5_000 });

		// Modal should contain at least one shortcut listed (kbd elements or shortcut rows)
		const shortcuts = modal.locator('kbd, .shortcut, [data-testid="shortcut"]');
		const count = await shortcuts.count();
		expect(count).toBeGreaterThan(0);
	});
});
