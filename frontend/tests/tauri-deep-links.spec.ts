/**
 * Tauri Desktop — Deep Links
 *
 * Verifies that `yapper://invite/:code` deep links open the join-server modal.
 *
 * Guards itself with __TAURI_INTERNALS__ detection.
 *
 * @desktop @regression
 */

import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

test.describe('Tauri deep links @desktop @regression', () => {
	test.beforeEach(async ({ page }) => {
		const isTauri = await page.evaluate(() => '__TAURI_INTERNALS__' in window);
		test.skip(!isTauri, 'Tauri-only test — skip in browser projects');
	});

	test('yapper://invite/:code opens the join server modal', async ({ page }) => {
		const device = buildMockDevice({ installation_id: 'tauri-deeplink-install', platform: 'tauri' });
		const authData = buildMockAuthData({ device });
		await setInstallationId(page, 'tauri-deeplink-install');
		await mockAuthEndpoints(page, authData, { devices: [device] });
		await page.route('**/api/v1/servers', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await page.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await mockExploreEndpoints(page);

		// Stub the invite-code lookup
		await page.route('**/api/v1/servers/join/**', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ id: 'test-server-id', name: 'Invite Server', member_count: 5 }),
			});
		});

		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });

		// Simulate deep-link arrival via Tauri event system
		await page.evaluate(() => {
			// Tauri v2 emits deep-link events via the event bus
			window.dispatchEvent(
				new CustomEvent('tauri://deep-link', {
					detail: { url: 'yapper://invite/TESTCODE123' },
				}),
			);
		});

		await page.waitForTimeout(500);

		// Join modal should appear
		const joinModal = page.locator(
			'[data-testid="join-modal"], [aria-label*="join" i], text=/join server|TESTCODE123/i',
		).first();
		await expect(joinModal).toBeVisible({ timeout: 10_000 });
	});
});
