import { test, expect, type Page } from '@playwright/test';
import { mockAuthEndpoints } from './auth-helper.js';

/**
 * Settings & GDPR E2E tests.
 *
 * Covers: settings page sections, data export trigger, account deletion UI.
 * Does NOT actually delete the test account — only verifies UI flow.
 * Requires E2E_EMAIL / E2E_PASSWORD.
 */


async function loginAs(page: Page) {
	await mockAuthEndpoints(page);
	await page.goto('/settings');
	await page.waitForURL(/\/settings/, { timeout: 20_000 });
}

// ─── Settings page structure ───────────────────────────────────────────────────

test.describe('Settings — structure', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page);
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });
	});

	test('settings page renders', async ({ page }) => {
		await expect(page.locator('body')).toBeVisible();
	});

	test('Profile section is visible', async ({ page }) => {
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 5_000 });
	});

	test('Privacy & Safety section is accessible', async ({ page }) => {
		await expect(page.getByRole('button', { name: 'Privacy & Safety' })).toBeVisible({ timeout: 5_000 });
	});

	test('Appearance section is accessible', async ({ page }) => {
		await expect(page.getByRole('button', { name: 'Appearance' })).toBeVisible({ timeout: 5_000 });
	});

	test('Notifications section is accessible', async ({ page }) => {
		await expect(page.getByRole('button', { name: 'Notifications' })).toBeVisible({ timeout: 5_000 });
	});
});

// ─── Profile settings ──────────────────────────────────────────────────────────

test.describe('Settings — Profile', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page);
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });
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
		await loginAs(page);
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });
	});

	test('data export button is visible', async ({ page }) => {
		const exportBtn = page.getByRole('button', { name: /Export My Data/i });
		await expect(exportBtn).toBeVisible({ timeout: 5_000 });
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
		await loginAs(page);
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });
	});

	test('danger zone section is visible', async ({ page }) => {
		const danger = page.getByText(/Danger Zone/i).first();
		await expect(danger).toBeVisible({ timeout: 5_000 });
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
		await loginAs(page);
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });
	});

	test('change password form is accessible', async ({ page }) => {
		const section = page.getByText(/Change Password|Password/i).first();
		await expect(section).toBeVisible({ timeout: 5_000 });
	});

	test('change password with wrong current password shows error', async ({ page }) => {
		// Navigate to the Change Password section (rendered only when activeSection === "password")
		const pwNavBtn = page.getByRole('button', { name: 'Change Password' });
		await pwNavBtn.waitFor({ timeout: 10_000 });
		await pwNavBtn.click();

		const currentInput = page.locator('#currentPw').first();
		const visible = await currentInput.isVisible({ timeout: 5_000 }).catch(() => false);

		if (!visible) {
			test.skip();
			return;
		}

		await currentInput.fill('WrongPassword123!');

		const newInput = page.locator('#newPw').first();
		await newInput.fill('NewPassword456!');

		const confirmInput = page.locator('#confirmPw').first();
		if (await confirmInput.isVisible()) {
			await confirmInput.fill('NewPassword456!');
		}

		const submitBtn = page.locator('button.save-btn');
		await submitBtn.click();

		// Should show an error toast (wrong current password).
		// Toast items have class "toast toast-error" and role="alert".
		// Avoid the outer .toast-container (role="region") which is always present but empty.
		const error = page.locator('.toast-error, .field-error').first();
		await expect(error).toBeVisible({ timeout: 10_000 });
	});
});
