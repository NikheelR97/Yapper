import { test, expect, type Page } from '@playwright/test';
import { buildMockAuthData, mockAuthEndpoints } from './auth-helper.js';

/**
 * Navigation and protected-route E2E tests.
 *
 * These tests cover page-level rendering and redirect behaviour.
 */

async function loginAs(page: Page) {
	await mockAuthEndpoints(page, buildMockAuthData());
	await page.goto('/explore');
	await page.waitForURL(/\/explore/, { timeout: 20_000 });
}

test.describe('Unauthenticated redirects', () => {
	for (const route of ['/dm', '/servers', '/explore', '/settings']) {
		test(`${route} redirects to /login when not authenticated`, async ({ page }) => {
			await page.context().clearCookies();
			await page.context().clearPermissions();

			await page.goto(route);

			await expect(page).toHaveURL(/\/login/, { timeout: 8_000 });
		});
	}
});

test.describe('Public pages', () => {
	test('root page loads', async ({ page }) => {
		await page.goto('/');
		await expect(page.locator('body')).toBeVisible();
	});

	test('/login page title contains Yapper', async ({ page }) => {
		await page.goto('/login');
		await expect(page).toHaveTitle(/Yapper/i);
	});

	test('/register page title contains Yapper', async ({ page }) => {
		await page.goto('/register');
		await expect(page).toHaveTitle(/Yapper/i);
	});

	test('/forgot-password page loads', async ({ page }) => {
		await page.goto('/forgot-password');
		await expect(page.locator('body')).toBeVisible();
		await expect(page).not.toHaveURL('/login');
	});
});

test.describe('Authenticated navigation', () => {
	test.beforeEach(async ({ page }) => {
		await loginAs(page);
	});

	test('DM page renders', async ({ page }) => {
		await page.goto('/dm');
		await expect(page).toHaveURL('/dm');
		await expect(page.locator('body')).toBeVisible();
	});

	test('Servers page renders', async ({ page }) => {
		await page.goto('/servers');
		await expect(page).toHaveURL('/servers');
		await expect(page.locator('body')).toBeVisible();
	});

	test('Explore page renders', async ({ page }) => {
		await page.goto('/explore');
		await expect(page).toHaveURL('/explore');
		await expect(page.locator('body')).toBeVisible();
	});

	test('Settings page renders', async ({ page }) => {
		await page.goto('/settings');
		await expect(page).toHaveURL('/settings');
		await expect(page.locator('body')).toBeVisible();
	});
});
