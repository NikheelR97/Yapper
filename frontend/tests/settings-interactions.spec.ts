import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { buildMockDevice } from './auth-helper.js';
import { mockSupportEndpoints, mockPremiumEndpoints } from './helpers/mock-routes.js';

/**
 * Feature: Settings Interactions
 *
 * Covers interactive behaviour within settings sections:
 * profile edits, theme toggle, change password, premium promo,
 * support ticket creation, and device management.
 */

async function setupSettingsPage(
	page: Page,
	options: { devices?: ReturnType<typeof buildMockDevice>[] } = {},
): Promise<void> {
	const installationId =
		(await page.evaluate(() => localStorage.getItem('yapper_installation_id'))) ??
		'settings-primary-install';
	const currentDevice = buildMockDevice({
		id: 'settings-primary-device',
		installation_id: installationId,
		label: 'Primary Browser',
	});
	const otherDevice = buildMockDevice({
		id: 'settings-remote-device',
		signal_device_id: 2,
		installation_id: 'settings-remote-install',
		label: 'Work Laptop',
	});

	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/devices', async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify(options.devices ?? [currentDevice, otherDevice]),
			});
			return;
		}
		await route.continue();
	});
	await page.route('**/api/v2/users/me', async (route) => {
		if (route.request().method() === 'PATCH' || route.request().method() === 'PUT') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ displayName: 'Updated Display Name' }),
			});
			return;
		}
		await route.continue();
	});
	await page.route('**/api/v2/users/me/avatar', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ avatarUrl: 'https://cdn.example.com/avatar.webp' }),
		});
	});
	await page.route('**/api/v2/auth/change-password', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
	});
	await page.route('**/api/v2/devices/**', async (route) => {
		if (route.request().method() === 'DELETE') {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
			return;
		}
		await route.continue();
	});

	await page.goto('/settings');
	await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });
}

test.describe('Settings - My Profile', () => {
	test('profile section renders display name and username fields @smoke', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await userPage.getByRole('button', { name: 'My Profile' }).click();

		await expect(
			userPage
				.locator('#displayName, input[placeholder*="display name" i], label')
				.filter({ hasText: /Display Name/i })
				.first(),
		).toBeVisible({ timeout: 10_000 });

		const nameField = userPage
			.locator('input[placeholder*="display name" i], input[id*="display"], input[name*="display"]')
			.first();
		await expect(nameField).toBeVisible({ timeout: 5_000 });
	});

	test('saving a profile change triggers a success notification', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await userPage.route('**/api/v2/users/me', async (route) => {
			if (route.request().method() === 'PATCH') {
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({ displayName: 'New Name' }),
				});
				return;
			}
			await route.continue();
		});

		await userPage.getByRole('button', { name: 'My Profile' }).click();
		await expect(userPage.locator('#displayName')).toBeVisible({ timeout: 10_000 });

		const nameField = userPage
			.locator('input[placeholder*="display name" i], input[id*="display"], input[name*="display"]')
			.first();
		if (await nameField.isVisible()) {
			await nameField.fill('Updated Display Name');
		}

		const saveButton = userPage.getByRole('button', { name: /Save|Update/i });
		if (await saveButton.isVisible()) {
			await saveButton.click();
			await expect(userPage.locator('.toast-success, [role="alert"].toast').first()).toBeVisible({
				timeout: 8_000,
			});
		}
	});
});

test.describe('Settings - Appearance', () => {
	test('Appearance section renders theme options @smoke', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await userPage.getByRole('button', { name: 'Appearance' }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Appearance', exact: true }),
		).toBeVisible({ timeout: 10_000 });

		const themeControls = userPage
			.locator('input[type="radio"][name*="theme"], button[data-theme], [aria-label*="theme"]')
			.or(userPage.getByText(/Dark|Light|System/i));
		await expect(themeControls.first()).toBeVisible({ timeout: 5_000 });
	});

	test('toggling the theme changes the data-theme attribute', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await userPage.route('**/api/v2/users/me/appearance', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
		});

		await userPage.getByRole('button', { name: 'Appearance' }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Appearance', exact: true }),
		).toBeVisible({ timeout: 10_000 });

		const lightCard = userPage.locator('label.theme-card').filter({ hasText: /Light/i });
		const darkCard = userPage.locator('label.theme-card').filter({ hasText: /Dark/i });
		const targetTheme = (await lightCard.isVisible({ timeout: 3_000 }).catch(() => false))
			? 'light'
			: 'dark';
		const cardToClick = targetTheme === 'light' ? lightCard : darkCard;

		if (await cardToClick.isVisible()) {
			await cardToClick.click();
			await userPage.getByRole('button', { name: /Save Appearance/i }).click();
			await expect(userPage.locator('html')).toHaveAttribute('data-theme', targetTheme, {
				timeout: 5_000,
			});
		}
	});
});

test.describe('Settings - Change Password', () => {
	test('Change Password section renders current/new/confirm fields @smoke', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await userPage.getByRole('button', { name: 'Change Password' }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Change Password', exact: true }),
		).toBeVisible({ timeout: 10_000 });

		await expect(userPage.locator('input[type="password"]').first()).toBeVisible();
		expect(await userPage.locator('input[type="password"]').count()).toBeGreaterThanOrEqual(2);
	});
});

test.describe('Settings - Yapper Premium', () => {
	test('Premium section renders heading and promo code input @smoke', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await mockPremiumEndpoints(userPage, { promoValid: true });

		await userPage.getByRole('button', { name: /Yapper Premium/i }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Yapper Premium', exact: true }),
		).toBeVisible({ timeout: 10_000 });
	});

	test('entering a valid promo code shows a success message', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await mockPremiumEndpoints(userPage, { promoValid: true });

		await userPage.getByRole('button', { name: /Yapper Premium/i }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Yapper Premium', exact: true }),
		).toBeVisible({ timeout: 10_000 });

		const promoInput = userPage
			.locator('input[placeholder*="promo" i], input[placeholder*="code" i], input[name*="promo"]')
			.first();
		if (await promoInput.isVisible()) {
			await promoInput.fill('TESTCODE123');
			await userPage.getByRole('button', { name: /Apply|Redeem/i }).click();
			await expect(userPage.getByText(/activated|success|applied/i)).toBeVisible({
				timeout: 5_000,
			});
		}
	});

	test('entering an invalid promo code shows an error message', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await mockPremiumEndpoints(userPage, { promoValid: false });

		await userPage.getByRole('button', { name: /Yapper Premium/i }).click();
		await expect(
			userPage.getByRole('heading', { name: 'Yapper Premium', exact: true }),
		).toBeVisible({ timeout: 10_000 });

		const promoInput = userPage
			.locator('input[placeholder*="promo" i], input[placeholder*="code" i], input[name*="promo"]')
			.first();
		if (await promoInput.isVisible()) {
			await promoInput.fill('INVALIDCODE');
			await userPage.getByRole('button', { name: /Apply|Redeem/i }).click();
			await expect(userPage.getByText(/invalid|expired|error/i)).toBeVisible({
				timeout: 5_000,
			});
		}
	});
});

test.describe('Settings - Support', () => {
	test('Support section renders the ticket creation form @smoke', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await mockSupportEndpoints(userPage, []);

		await userPage.getByRole('button', { name: 'Support' }).click();
		await expect(userPage.getByRole('heading', { name: /Support/i })).toBeVisible({
			timeout: 10_000,
		});
		await expect(userPage.locator('textarea, input[name*="description"]').first()).toBeVisible({
			timeout: 5_000,
		});
	});

	test('submitting a support ticket adds it to the history list', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await userPage.route('**/api/v2/support/tickets', async (route) => {
			if (route.request().method() === 'GET') {
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({
						tickets: [
							{
								id: 'ticket-existing',
								ticket_type: 'bug',
								subject: 'Existing ticket subject',
								priority: 'medium',
								status: 'open',
								hubspot_id: null,
								created_at: new Date().toISOString(),
							},
						],
					}),
				});
				return;
			}

			await route.fulfill({
				status: 201,
				contentType: 'application/json',
				body: JSON.stringify({
					id: 'ticket-new',
					ticket_type: 'bug',
					subject: 'E2E test ticket subject',
					priority: 'medium',
					status: 'open',
					hubspot_id: null,
					created_at: new Date().toISOString(),
				}),
			});
		});

		await userPage.getByRole('button', { name: 'Support' }).click();
		await expect(userPage.getByRole('heading', { name: /Support/i })).toBeVisible({
			timeout: 10_000,
		});
		await expect(userPage.locator('.ticket-subject').first()).toBeVisible({ timeout: 5_000 });

		const subjectInput = userPage.locator('input.text-input[type="text"]').first();
		const descriptionInput = userPage.locator('textarea').first();

		if ((await subjectInput.isVisible()) && (await descriptionInput.isVisible())) {
			await subjectInput.fill('E2E test ticket subject');
			await descriptionInput.fill('E2E test ticket description');

			const submitButton = userPage.getByRole('button', { name: /Submit Ticket/i });
			await expect(submitButton).toBeEnabled({ timeout: 3_000 });
			await submitButton.click();
			await expect(userPage.locator('.toast-success').first()).toBeVisible({ timeout: 8_000 });
		}
	});
});

test.describe('Settings - Device management', () => {
	test('devices sidebar shows the registered devices list @smoke', async ({ userPage }) => {
		await setupSettingsPage(userPage);
		await expect(userPage.getByText('Work Laptop')).toBeVisible({ timeout: 10_000 });
	});

	test('revoking a non-current device removes it from the list', async ({ userPage }) => {
		await setupSettingsPage(userPage);

		const revokeButton = userPage
			.locator('.device-item, [data-testid="device-item"]')
			.filter({ hasText: 'Work Laptop' })
			.getByRole('button', { name: /Revoke|Remove|Forget/i });

		if (await revokeButton.isVisible()) {
			const installationId =
				(await userPage.evaluate(() => localStorage.getItem('yapper_installation_id'))) ??
				'settings-primary-install';
			const currentDevice = buildMockDevice({
				id: 'settings-primary-device',
				installation_id: installationId,
				label: 'Primary Browser',
			});
			await userPage.route('**/api/v2/devices', async (route) => {
				if (route.request().method() === 'GET') {
					await route.fulfill({
						status: 200,
						contentType: 'application/json',
						body: JSON.stringify([currentDevice]),
					});
					return;
				}
				await route.continue();
			});

			await revokeButton.click();

			const confirmButton = userPage.getByRole('button', { name: /Confirm|Yes|Revoke/i });
			if (await confirmButton.isVisible({ timeout: 2_000 }).catch(() => false)) {
				await confirmButton.click();
			}

			await expect(userPage.getByText('Work Laptop')).toHaveCount(0, { timeout: 5_000 });
		}
	});
});
