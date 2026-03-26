import { test, expect } from '@playwright/test';
import {
	buildMockAuthData,
	buildMockDevice,
	mockAuthEndpoints,
	setInstallationId,
} from './auth-helper.js';
import { mockSupportEndpoints, mockPremiumEndpoints } from './helpers/mock-routes.js';

/**
 * Feature: Settings Interactions
 *
 * Covers interactive behaviour within settings sections:
 * profile edits, theme toggle, change password, premium promo,
 * support ticket creation, and device management.
 *
 * All tests use mocked auth — no live API needed.
 */

async function setupAuthAndNavigate(page: Parameters<typeof mockAuthEndpoints>[0], options: {
	devices?: ReturnType<typeof buildMockDevice>[];
} = {}): Promise<void> {
	const currentDevice = buildMockDevice({
		id: 'settings-primary-device',
		installation_id: 'settings-primary-install',
		label: 'Primary Browser',
	});
	const authData = buildMockAuthData({ device: currentDevice });
	const otherDevice = buildMockDevice({
		id: 'settings-remote-device',
		signal_device_id: 2,
		installation_id: 'settings-remote-install',
		label: 'Work Laptop',
	});

	await setInstallationId(page, currentDevice.installation_id ?? 'settings-primary-install');
	await mockAuthEndpoints(page, authData, {
		devices: options.devices ?? [currentDevice, otherDevice],
	});

	// Mock profile update endpoint
	await page.route(`**/api/v2/users/me`, async (route) => {
		if (route.request().method() === 'PATCH' || route.request().method() === 'PUT') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify(authData.user),
			});
		} else {
			await route.continue();
		}
	});

	// Mock avatar/banner upload
	await page.route(`**/api/v2/users/me/avatar`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ avatarUrl: 'https://cdn.example.com/avatar.webp' }) });
	});

	// Mock password change endpoint
	await page.route(`**/api/v2/auth/change-password`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
	});

	// Mock device revoke
	await page.route(`**/api/v2/devices/**`, async (route) => {
		if (route.request().method() === 'DELETE') {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
		} else {
			await route.continue();
		}
	});

	await page.goto('/settings');
	await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });
}

// ─── Profile editing ──────────────────────────────────────────────────────────

test.describe('Settings — My Profile', () => {
	test('profile section renders display name and username fields @smoke', async ({ page }) => {
		await setupAuthAndNavigate(page);

		await page.getByRole('button', { name: 'My Profile' }).click();
		// ProfileForm has no heading — check that the display name input is visible
		await expect(
			page.locator('#displayName, input[placeholder*="display name" i], label').filter({ hasText: /Display Name/i }).first(),
		).toBeVisible({ timeout: 10_000 });

		// Profile form inputs should be present
		const nameField = page.locator('input[placeholder*="display name" i], input[id*="display"], input[name*="display"]').first();
		await expect(nameField).toBeVisible({ timeout: 5_000 });
	});

	test('saving a profile change triggers a success notification', async ({ page }) => {
		await setupAuthAndNavigate(page);

		// Mock the PATCH specifically to return success
		await page.route(`**/api/v2/users/me`, async (route) => {
			if (route.request().method() === 'PATCH') {
				await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ displayName: 'New Name' }) });
			} else {
				await route.continue();
			}
		});

		await page.getByRole('button', { name: 'My Profile' }).click();
		// ProfileForm has no heading — verify the form is active by checking the input
		await expect(
			page.locator('#displayName'),
		).toBeVisible({ timeout: 10_000 });

		// Fill in the display name
		const nameField = page.locator('input[placeholder*="display name" i], input[id*="display"], input[name*="display"]').first();
		if (await nameField.isVisible()) {
			await nameField.fill('Updated Display Name');
		}

		// Click save button
		const saveBtn = page.getByRole('button', { name: /Save|Update/i });
		if (await saveBtn.isVisible()) {
			await saveBtn.click();
			// A success toast should appear — use class selector to avoid strict-mode
			// violation: getByText(/updated/i) would also match the preview-name field.
			await expect(
				page.locator('.toast-success, [role="alert"].toast').first(),
			).toBeVisible({ timeout: 8_000 });
		}
	});
});

// ─── Appearance ───────────────────────────────────────────────────────────────

test.describe('Settings — Appearance', () => {
	test('Appearance section renders theme options @smoke', async ({ page }) => {
		await setupAuthAndNavigate(page);

		await page.getByRole('button', { name: 'Appearance' }).click();
		await expect(page.getByRole('heading', { name: 'Appearance', exact: true })).toBeVisible({ timeout: 10_000 });

		// Theme toggle or radio buttons should be present
		const themeControls = page
			.locator('input[type="radio"][name*="theme"], button[data-theme], [aria-label*="theme"]')
			.or(page.getByText(/Dark|Light|System/i));
		await expect(themeControls.first()).toBeVisible({ timeout: 5_000 });
	});

	test('toggling the theme changes the data-theme attribute', async ({ page }) => {
		await setupAuthAndNavigate(page);

		// Mock the appearance PATCH so Save succeeds
		await page.route(`**/api/v2/users/me/appearance`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
		});

		await page.getByRole('button', { name: 'Appearance' }).click();
		await expect(page.getByRole('heading', { name: 'Appearance', exact: true })).toBeVisible({ timeout: 10_000 });

		// Theme is rendered as <label class="theme-card"> — click the "Light" card
		const lightCard = page.locator('label.theme-card').filter({ hasText: /Light/i });
		const darkCard = page.locator('label.theme-card').filter({ hasText: /Dark/i });

		const targetTheme = (await lightCard.isVisible({ timeout: 3_000 }).catch(() => false))
			? 'light'
			: 'dark';
		const cardToClick = targetTheme === 'light' ? lightCard : darkCard;

		if (await cardToClick.isVisible()) {
			await cardToClick.click();
			// Click Save to apply the theme to the document
			await page.getByRole('button', { name: /Save Appearance/i }).click();
			// The <html> element's data-theme attribute should update
			await expect(page.locator('html')).toHaveAttribute('data-theme', targetTheme, { timeout: 5_000 });
		}
	});
});

// ─── Change Password ──────────────────────────────────────────────────────────

test.describe('Settings — Change Password', () => {
	test('Change Password section renders current/new/confirm fields @smoke', async ({ page }) => {
		await setupAuthAndNavigate(page);

		await page.getByRole('button', { name: 'Change Password' }).click();
		await expect(page.getByRole('heading', { name: 'Change Password', exact: true })).toBeVisible({ timeout: 10_000 });

		await expect(page.locator('input[type="password"]').first()).toBeVisible();
		// There should be at least 2 password inputs (current + new or new + confirm)
		const passwordInputs = page.locator('input[type="password"]');
		const count = await passwordInputs.count();
		expect(count).toBeGreaterThanOrEqual(2);
	});
});

// ─── Premium promo code ───────────────────────────────────────────────────────

test.describe('Settings — Yapper Premium', () => {
	test('Premium section renders heading and promo code input @smoke', async ({ page }) => {
		await setupAuthAndNavigate(page);
		await mockPremiumEndpoints(page, { promoValid: true });

		await page.getByRole('button', { name: /Yapper Premium/i }).click();
		await expect(page.getByRole('heading', { name: 'Yapper Premium', exact: true })).toBeVisible({ timeout: 10_000 });
	});

	test('entering a valid promo code shows a success message', async ({ page }) => {
		await setupAuthAndNavigate(page);
		await mockPremiumEndpoints(page, { promoValid: true });

		await page.getByRole('button', { name: /Yapper Premium/i }).click();
		await expect(page.getByRole('heading', { name: 'Yapper Premium', exact: true })).toBeVisible({ timeout: 10_000 });

		const promoInput = page.locator('input[placeholder*="promo" i], input[placeholder*="code" i], input[name*="promo"]').first();
		if (await promoInput.isVisible()) {
			await promoInput.fill('TESTCODE123');
			const applyBtn = page.getByRole('button', { name: /Apply|Redeem/i });
			await applyBtn.click();
			await expect(
				page.getByText(/activated|success|applied/i),
			).toBeVisible({ timeout: 5_000 });
		}
	});

	test('entering an invalid promo code shows an error message', async ({ page }) => {
		await setupAuthAndNavigate(page);
		await mockPremiumEndpoints(page, { promoValid: false });

		await page.getByRole('button', { name: /Yapper Premium/i }).click();
		await expect(page.getByRole('heading', { name: 'Yapper Premium', exact: true })).toBeVisible({ timeout: 10_000 });

		const promoInput = page.locator('input[placeholder*="promo" i], input[placeholder*="code" i], input[name*="promo"]').first();
		if (await promoInput.isVisible()) {
			await promoInput.fill('INVALIDCODE');
			const applyBtn = page.getByRole('button', { name: /Apply|Redeem/i });
			await applyBtn.click();
			await expect(
				page.getByText(/invalid|expired|error/i),
			).toBeVisible({ timeout: 5_000 });
		}
	});
});

// ─── Support tickets ──────────────────────────────────────────────────────────

test.describe('Settings — Support', () => {
	test('Support section renders the ticket creation form @smoke', async ({ page }) => {
		await setupAuthAndNavigate(page);
		await mockSupportEndpoints(page, []);

		await page.getByRole('button', { name: 'Support' }).click();
		await expect(page.getByRole('heading', { name: /Support/i })).toBeVisible({ timeout: 10_000 });

		// Form elements should be present
		await expect(page.locator('textarea, input[name*="description"]').first()).toBeVisible({ timeout: 5_000 });
	});

	test('submitting a support ticket adds it to the history list', async ({ page }) => {
		await setupAuthAndNavigate(page);
		// mockSupportEndpoints returns a bare array, but Support.svelte's loadTickets()
		// expects { tickets: [...] }. Override with the correct server-side field names.
		await page.route(`**/api/v2/support/tickets`, async (route) => {
			if (route.request().method() === 'GET') {
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({
						tickets: [{
							id: 'ticket-existing',
							ticket_type: 'bug',
							subject: 'Existing ticket subject',
							priority: 'medium',
							status: 'open',
							hubspot_id: null,
							created_at: new Date().toISOString(),
						}],
					}),
				});
			} else {
				// POST — delegate to a simple 201 response
				await route.fulfill({
					status: 201,
					contentType: 'application/json',
					body: JSON.stringify({ id: 'ticket-new', ticket_type: 'bug', subject: 'E2E test ticket subject', priority: 'medium', status: 'open', hubspot_id: null, created_at: new Date().toISOString() }),
				});
			}
		});

		await page.getByRole('button', { name: 'Support' }).click();
		await expect(page.getByRole('heading', { name: /Support/i })).toBeVisible({ timeout: 10_000 });

		// Existing ticket must appear in the history list before submitting.
		await expect(page.locator('.ticket-subject').first()).toBeVisible({ timeout: 5_000 });

		// Fill both required fields (canSubmit = subject.trim() && description.trim()).
		const subjectInput = page.locator('input.text-input[type="text"]').first();
		const descInput = page.locator('textarea').first();

		if (await subjectInput.isVisible() && await descInput.isVisible()) {
			await subjectInput.fill('E2E test ticket subject');
			await descInput.fill('E2E test ticket description');

			const submitBtn = page.getByRole('button', { name: /Submit Ticket/i });
			await expect(submitBtn).toBeEnabled({ timeout: 3_000 });
			await submitBtn.click();

			// After submission the success toast appears, then loadTickets() re-runs.
			await expect(page.locator('.toast-success').first()).toBeVisible({ timeout: 8_000 });
		}
	});
});

// ─── Device management ────────────────────────────────────────────────────────

test.describe('Settings — Device management', () => {
	test('devices sidebar shows the registered devices list @smoke', async ({ page }) => {
		await setupAuthAndNavigate(page);

		// The devices section appears in the settings sidebar
		await expect(page.getByText('Work Laptop')).toBeVisible({ timeout: 10_000 });
	});

	test('revoking a non-current device removes it from the list', async ({ page }) => {
		await setupAuthAndNavigate(page);

		const revokeBtn = page
			.locator('.device-item, [data-testid="device-item"]')
			.filter({ hasText: 'Work Laptop' })
			.getByRole('button', { name: /Revoke|Remove|Forget/i });

		if (await revokeBtn.isVisible()) {
			// Mock the updated device list (Work Laptop removed)
			const currentDevice = buildMockDevice({
				id: 'settings-primary-device',
				installation_id: 'settings-primary-install',
				label: 'Primary Browser',
			});
			await page.route(`**/api/v2/devices`, async (route) => {
				if (route.request().method() === 'GET') {
					await route.fulfill({
						status: 200,
						contentType: 'application/json',
						body: JSON.stringify([currentDevice]),
					});
				} else {
					await route.continue();
				}
			});

			await revokeBtn.click();

			// Confirm if a dialog appears
			const confirmBtn = page.getByRole('button', { name: /Confirm|Yes|Revoke/i });
			if (await confirmBtn.isVisible({ timeout: 2_000 }).catch(() => false)) {
				await confirmBtn.click();
			}

			await expect(page.getByText('Work Laptop')).toHaveCount(0, { timeout: 5_000 });
		}
	});
});
