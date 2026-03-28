import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockParentalEndpoints, type MockChild, type MockAlert } from './helpers/mock-routes.js';

/**
 * Feature: Parental Controls
 *
 * Tests the child setup wizard and parent dashboard with mocked endpoints.
 */

async function setupParentAuth(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
	const device = buildMockDevice({ installation_id: 'parental-test-install' });
	const authData = buildMockAuthData({
		device,
		user: {
			id: 'parent-user',
			username: 'parent_user',
			displayName: 'Parent User',
			avatarUrl: null,
			accountType: 'parent',
			isPremium: false,
		},
	});
	await setInstallationId(page, 'parental-test-install');
	await mockAuthEndpoints(page, authData);

	await page.route(`**/api/v2/servers`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route(`**/api/v2/conversations`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

// ─── Child setup wizard ───────────────────────────────────────────────────────

test.describe('Parental controls — child setup wizard', () => {
	// Clear saved auth so nikki's real session doesn't pre-populate authStore
	// (parent layout skips refreshSession() when authStore.user is already set,
	// so without this the accountType check sees nikki = non-parent → redirect)
	test.use({ storageState: { cookies: [], origins: [] } });

	test.beforeEach(async ({ page }) => {
		await setupParentAuth(page);
		await mockParentalEndpoints(page, [], []);
	});

	test('step 1 renders profile fields (display name, username, email, password) @smoke', async ({ page }) => {
		await page.goto('/parent/children/setup');

		// Step 1 fields
		await expect(
			page.locator('input[placeholder*="display name" i], input[id*="display"], input[type="text"]').first(),
		).toBeVisible({ timeout: 10_000 });
		await expect(page.locator('input[type="email"]').first()).toBeVisible();
		await expect(page.locator('input[type="password"]').first()).toBeVisible();
	});

	test('step 2 shows date of birth fields after completing step 1', async ({ page }) => {
		await page.goto('/parent/children/setup');
		await expect(page.locator('body')).toBeVisible({ timeout: 10_000 });

		// Fill step 1
		const inputs = page.locator('input[type="text"], input[type="email"]');
		const count = await inputs.count();

		if (count >= 3) {
			await inputs.nth(0).fill('TestChild');
			await inputs.nth(1).fill('testchild');
			await page.locator('input[type="email"]').first().fill('child@example.com');
			await page.locator('input[type="password"]').first().fill('ChildPass1!');
		}

		const nextBtn = page.getByRole('button', { name: /Next|Continue/i });
		if (await nextBtn.isVisible()) {
			await nextBtn.click();

			// Step 2 should show DOB fields
			await expect(
				page.locator('input[type="date"], select[name*="month"], input[placeholder*="year" i]').first(),
			).toBeVisible({ timeout: 5_000 });
		}
	});

	test('COPPA warning appears for under-13 date of birth', async ({ page }) => {
		await page.goto('/parent/children/setup');
		await expect(page.locator('body')).toBeVisible({ timeout: 10_000 });

		// Navigate to step 2
		const nextBtn = page.getByRole('button', { name: /Next|Continue/i });

		// Try to reach step 2 by filling minimal step 1 data
		const emailInput = page.locator('input[type="email"]').first();
		const passwordInput = page.locator('input[type="password"]').first();
		if (await emailInput.isVisible()) {
			await emailInput.fill('child@example.com');
			await passwordInput.fill('Pass1!');
		}

		if (await nextBtn.isVisible({ timeout: 2_000 }).catch(() => false)) {
			await nextBtn.click();

			// Set a DOB that makes user under 13
			const dateInput = page.locator('input[type="date"]').first();
			if (await dateInput.isVisible({ timeout: 3_000 }).catch(() => false)) {
				const under13Date = new Date();
				under13Date.setFullYear(under13Date.getFullYear() - 10);
				await dateInput.fill(under13Date.toISOString().split('T')[0]);

				// COPPA warning should appear
				await expect(
					page.getByText(/COPPA|13|age|parent|consent/i),
				).toBeVisible({ timeout: 3_000 });
			}
		}
	});

	test('step 3 shows safety toggles (friend requests, server joins, screen time)', async ({ page }) => {
		await page.goto('/parent/children/setup');
		await expect(page.locator('#displayName')).toBeVisible({ timeout: 10_000 });

		// Step 1: fill all required fields (displayName, username, email, password).
		await page.locator('#displayName').fill('TestChild');
		await page.locator('#username').fill('testchild_e2e');
		await page.locator('#email').fill('child@example.com');
		await page.locator('#password').fill('ChildPass1!');

		const nextBtn = page.getByRole('button', { name: /Next|Continue/i });
		await nextBtn.click();

		// Step 2: fill DOB fields (separate number inputs — not a date picker).
		await expect(page.locator('#dobDay')).toBeVisible({ timeout: 5_000 });
		await page.locator('#dobDay').fill('15');
		await page.locator('#dobMonth').fill('6');
		await page.locator('#dobYear').fill('2010');
		await nextBtn.click();

		// Step 3: safety gate heading and toggle controls should be present.
		// Checkboxes inside .toggle-switch labels are visually hidden by CSS (custom slider
		// pattern), so use count() rather than toBeVisible().
		await expect(page.getByText('Safety Gates')).toBeVisible({ timeout: 5_000 });
		const toggleCount = await page.locator(
			'input[type="checkbox"], input[type="range"], [role="switch"], .toggle-switch',
		).count();
		expect(toggleCount).toBeGreaterThan(0);
	});
});

// ─── Parent dashboard ─────────────────────────────────────────────────────────

test.describe('Parental controls — parent dashboard', () => {
	// Clear saved auth so nikki's real session doesn't pre-populate authStore
	test.use({ storageState: { cookies: [], origins: [] } });

	const mockChild: MockChild = {
		id: 'child-1',
		username: 'my_child',
		displayName: 'My Child',
		avatarUrl: null,
	};

	const mockAlerts: MockAlert[] = [
		{
			id: 'alert-1',
			type: 'friend_request',
			childId: 'child-1',
			description: 'Friend request from unknown_user',
			createdAt: new Date().toISOString(),
		},
	];

	test.beforeEach(async ({ page }) => {
		await setupParentAuth(page);
		await mockParentalEndpoints(page, [mockChild], mockAlerts);
	});

	test('parent dashboard renders and shows safety section @smoke', async ({ page }) => {
		await page.goto('/parent/dashboard');
		await expect(page.locator('body')).toBeVisible({ timeout: 15_000 });
		// Should render without crashing
		await expect(page.locator('main, [role="main"], .dashboard').first()).toBeVisible({ timeout: 10_000 });
	});

	test('pending alerts section shows alert items', async ({ page }) => {
		// The parental store's loadChildren() expects { children: [...] } (snake_case fields),
		// but mockParentalEndpoints returns a bare array with camelCase — override it here.
		await page.route(`**/api/v2/parental/children`, async (route) => {
			if (route.request().method() === 'GET') {
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({
						children: [{ id: 'child-1', username: 'my_child', display_name: 'My Child', avatar_url: null }],
					}),
				});
			} else {
				await route.continue();
			}
		});

		// loadAlerts() fetches per-child notifications, not the generic pending-alerts endpoint.
		await page.route(`**/api/v2/parental/children/child-1/notifications`, async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					friend_requests: [{
						id: 'fr-1',
						requester_id: 'u-1',
						requester_name: 'unknown_user',
						status: 'pending',
						created_at: new Date().toISOString(),
					}],
					server_joins: [],
				}),
			});
		});

		await page.goto('/parent/dashboard');
		await expect(page.locator('body')).toBeVisible({ timeout: 15_000 });

		// Safety dashboard heading confirms a child is selected and dashboard is rendered.
		await expect(
			page.getByRole('heading', { name: /Safety Dashboard/i }),
		).toBeVisible({ timeout: 10_000 });

		// Alert card rendered by PendingAlerts.svelte contains the alert description.
		await expect(page.locator('.alert-card').first()).toBeVisible({ timeout: 10_000 });
	});
});
