import { test, expect, type Page } from '@playwright/test';

/**
 * Navigation & protected-route E2E tests.
 *
 * These tests cover page-level rendering and redirect behaviour.
 * Tests that require a real session are gated behind E2E_EMAIL env var.
 */

const TEST_EMAIL = process.env.E2E_EMAIL ?? 'e2e@test.yapper.internal';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? 'E2eTestPass1!';

// ─── Helpers ───────────────────────────────────────────────────────────────────

async function loginAs(page: Page, email: string, password: string) {
	await page.goto('/login');
	await page.fill('#email', email);
	await page.fill('#password', password);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 10_000 });
}

// ─── Unauthenticated redirects ─────────────────────────────────────────────────

test.describe('Unauthenticated redirects', () => {
	// These tests check that protected routes send unauthenticated visitors to /login.
	// The SvelteKit (app) layout guards these routes.

	for (const route of ['/dm', '/servers', '/explore', '/settings']) {
		test(`${route} redirects to /login when not authenticated`, async ({ page }) => {
			// Clear any stored auth state
			await page.context().clearCookies();
			await page.context().clearPermissions();

			await page.goto(route);

			// Should land on /login (possibly after a redirect chain)
			await expect(page).toHaveURL(/\/login/, { timeout: 8_000 });
		});
	}
});

// ─── Public pages ──────────────────────────────────────────────────────────────

test.describe('Public pages', () => {
	test('root page loads', async ({ page }) => {
		await page.goto('/');
		// Root either redirects to /login or renders a landing page — just confirm no crash
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

// ─── Authenticated navigation ──────────────────────────────────────────────────

test.describe('Authenticated navigation', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
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
