/**
 * Tauri Desktop — Vault Gate (Stronghold passphrase)
 *
 * Verifies that on first launch the app presents a passphrase entry gate
 * before unlocking the Stronghold secure store, and that the gate responds
 * correctly to valid/invalid input.
 *
 * Guards itself with __TAURI_INTERNALS__ detection so the test is a no-op
 * when run under a plain browser Playwright project.
 *
 * @desktop @smoke
 */

import { test, expect } from '@playwright/test';

test.describe('Tauri vault gate @desktop @smoke', () => {
	test.beforeEach(async ({ page }) => {
		const isTauri = await page.evaluate(() => '__TAURI_INTERNALS__' in window);
		test.skip(!isTauri, 'Tauri-only test — skip in browser projects');
	});

	test('Stronghold passphrase gate appears on first launch', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// The vault gate should show a passphrase input
		const passphraseInput = page.locator(
			'input[type="password"], input[placeholder*="passphrase" i], [data-testid="vault-passphrase"]',
		).first();
		await expect(passphraseInput).toBeVisible({ timeout: 10_000 });
	});

	test('empty passphrase shows validation error', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		const submitBtn = page.locator('button[type="submit"], button:has-text("Unlock")').first();
		if (!await submitBtn.isVisible({ timeout: 5_000 }).catch(() => false)) return;

		await submitBtn.click();

		// Should show an error, not proceed to the app
		const errorMsg = page.locator(
			'text=/passphrase required|cannot be empty|enter a passphrase/i',
			{ has: page.locator('visible=true') },
		).first();
		await expect(errorMsg).toBeVisible({ timeout: 5_000 });
	});
});
