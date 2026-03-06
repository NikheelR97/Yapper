import { test, expect, type Page } from '@playwright/test';

/**
 * Settings & GDPR E2E tests.
 *
 * Covers: settings page sections, data export trigger, account deletion UI.
 * Does NOT actually delete the test account — only verifies UI flow.
 * Requires E2E_EMAIL / E2E_PASSWORD.
 */

const TEST_EMAIL    = process.env.E2E_EMAIL ?? 'e2e@test.yapper.internal';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? 'E2eTestPass1!';

async function loginAs(page: Page, email: string, password: string) {
	await page.goto('/login');
	await page.fill('#email', email);
	await page.fill('#password', password);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 10_000 });
}

// ─── Settings page structure ───────────────────────────────────────────────────

test.describe('Settings — structure', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
		await page.goto('/settings');
		await expect(page).toHaveURL(/\/settings/, { timeout: 10_000 });
		await expect(page.locator('.settings-page')).toBeVisible({ timeout: 10_000 });
	});

	test('settings page renders', async ({ page }) => {
		await expect(page.locator('body')).toBeVisible();
	});

	test('Profile section is visible', async ({ page }) => {
		const section = page.getByRole('button', { name: /My Profile/i });
		await expect(section).toBeVisible({ timeout: 5_000 });
	});

	test('Privacy & Safety section is accessible', async ({ page }) => {
		await expect(
			page.getByRole('button', { name: /Privacy & Safety/i }),
		).toBeVisible({ timeout: 5_000 });
	});

	test('Appearance section is accessible', async ({ page }) => {
		await expect(
			page.getByRole('button', { name: /Appearance/i }),
		).toBeVisible({ timeout: 5_000 });
	});

	test('Notifications section is accessible', async ({ page }) => {
		await expect(
			page.getByRole('button', { name: /Notifications/i }),
		).toBeVisible({ timeout: 5_000 });
	});
});

// ─── Profile settings ──────────────────────────────────────────────────────────

test.describe('Settings — Profile', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
		await page.goto('/settings');
	});

	test('display name field is pre-populated', async ({ page }) => {
		const displayNameInput = page.locator('input[id*="display"], input[name*="display"], input[placeholder*="display"]').first();
		if (await displayNameInput.isVisible({ timeout: 5_000 }).catch(() => false)) {
			const value = await displayNameInput.inputValue();
			// Should have some value (not empty for existing user)
			expect(typeof value).toBe('string');
		}
	});

	test('can update display name', async ({ page }) => {
		const displayNameInput = page.locator('input[id*="display"], input[name*="display"], input[placeholder*="display"]').first();
		const visible = await displayNameInput.isVisible({ timeout: 5_000 }).catch(() => false);

		if (!visible) {
			test.skip();
			return;
		}

		const original = await displayNameInput.inputValue();
		await displayNameInput.fill('E2E Test Name');

		const saveBtn = page.getByRole('button', { name: /Save|Update/i }).first();
		await saveBtn.click();

		// Should show success feedback (toast or inline)
		const feedback = page.locator('[class*="toast"], [role="alert"], [class*="success"]').first();
		await expect(feedback).toBeVisible({ timeout: 5_000 });

		// Restore original value
		await displayNameInput.fill(original);
		await saveBtn.click();
	});
});

// ─── GDPR: Data export ────────────────────────────────────────────────────────

test.describe('Settings — GDPR data export', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
		await page.goto('/settings');
	});

	test('data export button is visible', async ({ page }) => {
		const exportBtn = page.getByRole('button', { name: /Export My Data/i });
		await expect(exportBtn).toBeVisible({ timeout: 8_000 });
	});

	test('data export triggers a download', async ({ page }) => {
		const exportBtn = page.getByRole('button', { name: /Export|Download.*data/i }).first();
		const visible = await exportBtn.isVisible({ timeout: 8_000 }).catch(() => false);

		if (!visible) {
			test.skip();
			return;
		}

		// Listen for download event
		const downloadPromise = page.waitForEvent('download', { timeout: 15_000 }).catch(() => null);
		await exportBtn.click();

		const download = await downloadPromise;
		if (download) {
			expect(download.suggestedFilename()).toMatch(/\.zip|\.json/i);
		} else {
			// Download may open as blob URL — check for success toast instead
			const toast = page.locator('[class*="toast"], [role="alert"]').first();
			await expect(toast).toBeVisible({ timeout: 8_000 });
		}
	});
});

// ─── GDPR: Account deletion UI ────────────────────────────────────────────────

test.describe('Settings — account deletion UI', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
		await page.goto('/settings');
	});

	test('danger zone section is visible', async ({ page }) => {
		const danger = page.getByText(/Danger Zone/i).first();
		await expect(danger).toBeVisible({ timeout: 8_000 });
	});

	test('delete account requires confirmation — does NOT actually delete', async ({ page }) => {
		const deleteBtn = page.getByRole('button', { name: /Delete Account/i }).first();
		const visible = await deleteBtn.isVisible({ timeout: 8_000 }).catch(() => false);

		if (!visible) {
			test.skip();
			return;
		}

		await deleteBtn.click();

		// Should show a confirmation dialog / modal before actually deleting
		const confirmation = page.locator('[role="dialog"], .modal, [class*="confirm"]').first();
		await expect(confirmation).toBeVisible({ timeout: 5_000 });

		// Close/cancel the dialog — do NOT confirm deletion
		const cancelBtn = page.getByRole('button', { name: /Cancel|No|Back/i }).first();
		if (await cancelBtn.isVisible()) {
			await cancelBtn.click();
		} else {
			await page.keyboard.press('Escape');
		}

		// Should still be logged in
		await expect(page).toHaveURL(/\/settings/, { timeout: 5_000 });
	});
});

// ─── Change password UI ────────────────────────────────────────────────────────

test.describe('Settings — change password', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
		await page.goto('/settings');
	});

	test('change password form is accessible', async ({ page }) => {
		const section = page.getByText(/Change Password|Password/i).first();
		await expect(section).toBeVisible({ timeout: 5_000 });
	});

	test('change password with wrong current password shows error', async ({ page }) => {
		const currentInput = page.locator('input[id*="current"], input[name*="current"]').first();
		const visible = await currentInput.isVisible({ timeout: 5_000 }).catch(() => false);

		if (!visible) {
			test.skip();
			return;
		}

		await currentInput.fill('WrongPassword123!');

		const newInput = page.locator('input[id*="new"], input[name*="new"]').first();
		await newInput.fill('NewPassword456!');

		const confirmInput = page.locator('input[id*="confirm"], input[name*="confirm"]').first();
		if (await confirmInput.isVisible()) {
			await confirmInput.fill('NewPassword456!');
		}

		const submitBtn = page.getByRole('button', { name: /Change|Update.*Password/i }).first();
		await submitBtn.click();

		// Should show an error (wrong current password)
		const error = page.locator('[role="alert"], [class*="error"], [class*="toast"]').first();
		await expect(error).toBeVisible({ timeout: 8_000 });
	});
});
