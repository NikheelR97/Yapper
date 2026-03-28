import { test as base, expect } from '@playwright/test';
import { test as authedTest } from './fixtures/auth.fixture';

/**
 * Navigation and protected-route E2E tests.
 *
 * These tests cover page-level rendering and redirect behaviour.
 */

base.describe('Unauthenticated redirects', () => {
	for (const route of ['/dm', '/servers', '/explore', '/settings']) {
		base(`${route} redirects to /login when not authenticated`, async ({ page }) => {
			await page.context().clearCookies();
			await page.context().clearPermissions();

			await page.goto(route);

			await expect(page).toHaveURL(/\/login/, { timeout: 8_000 });
		});
	}
});

base.describe('Public pages', () => {
	base('root page loads', async ({ page }) => {
		await page.goto('/');
		await expect(page.locator('body')).toBeVisible();
	});

	base('/login page title contains Yapper', async ({ page }) => {
		await page.goto('/login');
		await expect(page).toHaveTitle(/Yapper/i);
	});

	base('/register page title contains Yapper', async ({ page }) => {
		await page.goto('/register');
		await expect(page).toHaveTitle(/Yapper/i);
	});

	base('/forgot-password page loads', async ({ page }) => {
		await page.goto('/forgot-password');
		await expect(page.locator('body')).toBeVisible();
		await expect(page).not.toHaveURL('/login');
	});
});

authedTest.describe('Authenticated navigation', () => {
	authedTest('DM page renders', async ({ userPage }) => {
		await userPage.goto('/dm');
		await expect(userPage).toHaveURL('/dm');
		await expect(userPage.locator('body')).toBeVisible();
	});

	authedTest('Servers page renders', async ({ userPage }) => {
		await userPage.goto('/servers');
		await expect(userPage).toHaveURL('/servers');
		await expect(userPage.locator('body')).toBeVisible();
	});

	authedTest('Explore page renders', async ({ userPage }) => {
		await userPage.goto('/explore');
		await expect(userPage).toHaveURL('/explore');
		await expect(userPage.locator('body')).toBeVisible();
	});

	authedTest('Settings page renders', async ({ userPage }) => {
		await userPage.goto('/settings');
		await expect(userPage).toHaveURL('/settings');
		await expect(userPage.locator('body')).toBeVisible();
	});
});
