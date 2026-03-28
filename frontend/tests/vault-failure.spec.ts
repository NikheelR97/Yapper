/**
 * Codex Security Report - Section 3.2
 * Feature: Desktop Local Vault Failure Modes
 *
 * @desktop-native @encryption @edge-case
 */

import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { mockExploreEndpoints } from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

async function setupVaultFailure(page: Page): Promise<void> {
	log('VAULT', 'SETUP', 'Overriding window.indexedDB to simulate OS-level write revocation');

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

	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);
}

test.describe('Vault failure modes @desktop-native @encryption @edge-case', () => {
	test.beforeEach(async ({}, testInfo) => {
		if (!process.env.TAURI_BINARY) {
			testInfo.skip(true, 'TAURI_BINARY not set — skipping Tauri-specific test');
		}
	});

	test('OS-level write revocation surfaces Secure Storage Unavailable UI @smoke', async ({ userPage }) => {
		await setupVaultFailure(userPage);
		await userPage.goto('/explore');

		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await expect(
			userPage.getByRole('heading', { name: 'Secure Storage Unavailable' }),
		).toBeVisible({ timeout: 10_000 });
		await expect(userPage.getByRole('button', { name: 'Retry' })).toBeVisible();
		await expect(userPage.getByRole('button', { name: 'Sign Out' })).toBeVisible();
	});
});
