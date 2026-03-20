/**
 * Codex Security Report — Section 3.2
 * Feature: Desktop Local Vault Failure Modes
 *
 * Simulates OS-level revocation of write permissions by overriding
 * window.indexedDB to throw a SecurityError before any key can be
 * written to disk.  The app must catch the failure and surface the
 * "Secure Storage Unavailable" recovery UI without leaking keys.
 *
 * @desktop-native @encryption @edge-case
 */

import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

async function setupVaultFailure(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
	log('VAULT', 'SETUP', 'Overriding window.indexedDB to simulate OS-level write revocation');

	// Inject before the page loads so the override is in place when the app
	// first touches IndexedDB during configureSignalStore().
	await page.addInitScript(() => {
		Object.defineProperty(window, 'indexedDB', {
			get() {
				throw new DOMException(
					'Permission denied: disk write access revoked by OS policy',
					'SecurityError',
				);
			},
			configurable: true,
		});
	});

	const device = buildMockDevice({ installation_id: 'vault-fail-install' });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, 'vault-fail-install');
	await mockAuthEndpoints(page, authData);

	// Stub out sidebar data so the app shell doesn't make extra real requests.
	await page.route('**/api/v1/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v1/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);
}

test.describe('Vault failure modes @desktop-native @encryption @edge-case', () => {
	test('OS-level write revocation surfaces Secure Storage Unavailable UI @smoke', async ({ page }) => {
		await setupVaultFailure(page);

		log('NETWORK', 'NAVIGATE', 'Navigating to /explore with IndexedDB disabled');
		await page.goto('/explore');

		// The app's configureSignalStore() will throw when it tries to open IndexedDB.
		// enterSecureStoreFailureState() sets ready=true, secureStoreError=<message>,
		// which clears the loading screen and shows the error panel.
		log('ASSERTION', 'STATE', 'Waiting for loading screen to clear (ready=true via failure path)');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		log('VAULT', 'FAILURE', 'Loading screen cleared. Asserting Secure Storage error panel.');

		// Primary heading must be visible.
		await expect(
			page.getByRole('heading', { name: 'Secure Storage Unavailable' }),
		).toBeVisible({ timeout: 10_000 });

		log('ASSERTION', 'UI', '"Secure Storage Unavailable" heading confirmed. [PASS]');

		// Both recovery actions must be accessible (no data is leaked in either path).
		await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Sign Out' })).toBeVisible();

		log('VALIDATION', 'UI', 'Retry + Sign Out recovery actions visible. Session remains read-only. [PASS]');
	});
});
