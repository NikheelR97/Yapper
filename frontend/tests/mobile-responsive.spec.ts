import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

/**
 * Feature: Mobile Responsive Layout
 *
 * All tests run at 375×667 (iPhone SE) viewport via the 'mobile-chrome' project.
 * Uses mocked auth — no live API required.
 */

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

async function setupMobileAuth(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
	const device = buildMockDevice({ installation_id: 'mobile-test-install', label: 'Mobile Browser' });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, 'mobile-test-install');
	await mockAuthEndpoints(page, authData);
}

// ─── Login page ───────────────────────────────────────────────────────────────

test.describe('Mobile — Login page', () => {
	// (auth) layout redirects authenticated users to /explore — clear saved auth.
	test.use({ storageState: { cookies: [], origins: [] } });

	test('login form is fully visible without horizontal scroll @smoke', async ({ page }) => {
		await page.goto('/login');

		// No horizontal scrollbar
		const scrollWidth = await page.evaluate(() => document.body.scrollWidth);
		const clientWidth = await page.evaluate(() => document.body.clientWidth);
		expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1); // 1px tolerance

		await expect(page.locator('#email')).toBeVisible();
		await expect(page.locator('#password')).toBeVisible();
		await expect(page.getByRole('button', { name: /Sign In/i })).toBeVisible();
	});

	test('brand panel (left side) is hidden on mobile @smoke', async ({ page }) => {
		await page.goto('/login');

		// The brand panel has display:none at mobile breakpoints
		const brandPanel = page.locator('.brand-panel');
		const isHidden = await brandPanel.evaluate((el) => {
			return window.getComputedStyle(el).display === 'none';
		}).catch(() => true);
		expect(isHidden).toBe(true);
	});
});

// ─── Register page ────────────────────────────────────────────────────────────

test.describe('Mobile — Register page', () => {
	// (auth) layout redirects authenticated users to /explore — clear saved auth.
	test.use({ storageState: { cookies: [], origins: [] } });

	test('register form is fully visible without horizontal scroll @smoke', async ({ page }) => {
		await page.goto('/register');

		const scrollWidth = await page.evaluate(() => document.body.scrollWidth);
		const clientWidth = await page.evaluate(() => document.body.clientWidth);
		expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);

		await expect(page.locator('#username')).toBeVisible();
		await expect(page.locator('#email')).toBeVisible();
		await expect(page.locator('#password')).toBeVisible();
	});
});

// ─── Explore page ─────────────────────────────────────────────────────────────

test.describe('Mobile — Explore page', () => {
	test.beforeEach(async ({ page }) => {
		await setupMobileAuth(page);
		await mockExploreEndpoints(page);
		// Mock servers endpoint for app load
		await page.route(`**/api/v1/servers`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await page.route(`**/api/v1/conversations`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
	});

	test('search bar fills available width @smoke', async ({ page }) => {
		await page.goto('/explore');
		const searchBox = page.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });

		const box = await searchBox.boundingBox();
		expect(box).toBeTruthy();
		// Search bar width should be close to the viewport width (allowing for padding)
		expect(box!.width).toBeGreaterThan(300);
	});

	test('page renders without horizontal overflow @smoke', async ({ page }) => {
		await page.goto('/explore');
		await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({ timeout: 20_000 });

		const scrollWidth = await page.evaluate(() => document.body.scrollWidth);
		const clientWidth = await page.evaluate(() => document.body.clientWidth);
		expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
	});
});

// ─── Settings page ────────────────────────────────────────────────────────────

test.describe('Mobile — Settings page', () => {
	test.beforeEach(async ({ page }) => {
		await setupMobileAuth(page);
		await page.route(`**/api/v1/servers`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await page.route(`**/api/v1/conversations`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
	});

	test('settings page loads and renders navigation @smoke', async ({ page }) => {
		await page.goto('/settings');
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });
	});

	test('settings right sidebar is hidden or collapsed on mobile', async ({ page }) => {
		await page.goto('/settings');
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });

		// At 375px, the right sidebar (.settings-sidebar-right) should be hidden
		const rightSidebar = page.locator('.settings-right, .sidebar-right, [data-testid="settings-right"]');
		if (await rightSidebar.count() > 0) {
			const isHidden = await rightSidebar.first().evaluate((el) => {
				const style = window.getComputedStyle(el);
				return style.display === 'none' || style.visibility === 'hidden' || parseFloat(style.width) === 0;
			});
			expect(isHidden).toBe(true);
		}
	});

	test('page renders without horizontal overflow', async ({ page }) => {
		await page.goto('/settings');
		await expect(page.getByRole('button', { name: 'My Profile' })).toBeVisible({ timeout: 20_000 });

		const scrollWidth = await page.evaluate(() => document.body.scrollWidth);
		const clientWidth = await page.evaluate(() => document.body.clientWidth);
		expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
	});
});

// ─── DM page ──────────────────────────────────────────────────────────────────

test.describe('Mobile — DM page', () => {
	test.beforeEach(async ({ page }) => {
		await setupMobileAuth(page);
		await page.route(`**/api/v1/conversations`, async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([]),
			});
		});
		await page.route(`**/api/v1/servers`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
	});

	test('DM inbox page loads without horizontal overflow @smoke', async ({ page }) => {
		await page.goto('/dm');
		// Wait for page to load
		await expect(page.locator('body')).toBeVisible({ timeout: 20_000 });

		const scrollWidth = await page.evaluate(() => document.body.scrollWidth);
		const clientWidth = await page.evaluate(() => document.body.clientWidth);
		expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
	});
});
