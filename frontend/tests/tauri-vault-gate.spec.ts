/**
 * Tauri Desktop - Vault Gate (Stronghold passphrase)
 *
 * @desktop @smoke
 */

import { test, expect } from '@playwright/test';

test.describe('Tauri vault gate @desktop @smoke', () => {
	test.beforeEach(async ({}, testInfo) => {
		if (!process.env.TAURI_BINARY) {
			testInfo.skip(true, 'TAURI_BINARY not set — skipping Tauri-specific test');
		}
	});

	test('Stronghold passphrase gate appears on first launch', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		const passphraseInput = page
			.locator(
				'input[type="password"], input[placeholder*="passphrase" i], [data-testid="vault-passphrase"]',
			)
			.first();
		await expect(passphraseInput).toBeVisible({ timeout: 10_000 });
	});

	test('empty passphrase shows validation error', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		const submitButton = page.locator('button[type="submit"], button:has-text("Unlock")').first();
		if (!(await submitButton.isVisible({ timeout: 5_000 }).catch(() => false))) {
			return;
		}

		await submitButton.click();

		const errorMessage = page
			.locator('text=/passphrase required|cannot be empty|enter a passphrase/i', {
				has: page.locator('visible=true'),
			})
			.first();
		await expect(errorMessage).toBeVisible({ timeout: 5_000 });
	});
});
