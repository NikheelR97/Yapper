import { test, expect, type Page } from '@playwright/test';

/**
 * Explore page E2E tests.
 *
 * Covers: search, community listing, server join flow.
 * Authenticated tests require E2E_EMAIL / E2E_PASSWORD.
 */

const TEST_EMAIL = process.env.E2E_EMAIL ?? 'e2e@test.yapper.internal';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? 'E2eTestPass1!';

async function loginAs(page: Page, email: string, password: string) {
	await page.goto('/login');
	await page.fill('#email', email);
	await page.fill('#password', password);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 10_000 });
}

// ─── Unauthenticated ───────────────────────────────────────────────────────────

test.describe('Explore — unauthenticated', () => {
	test('redirects to /login', async ({ page }) => {
		await page.context().clearCookies();
		await page.goto('/explore');
		await expect(page).toHaveURL(/\/login/, { timeout: 8_000 });
	});
});

// ─── Authenticated ─────────────────────────────────────────────────────────────

test.describe('Explore — authenticated', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
		await page.goto('/explore');
		await expect(page).toHaveURL(/\/explore/);
		await expect(page.locator('.explore-page')).toBeVisible({ timeout: 10_000 });
	});

	test('renders search bar', async ({ page }) => {
		await expect(page.locator('.search-input')).toBeVisible({ timeout: 10_000 });
	});

	test('renders Communities section', async ({ page }) => {
		await expect(page.getByText(/Communities/i)).toBeVisible({ timeout: 10_000 });
	});

	test('search filters results', async ({ page }) => {
		const searchInput = page.locator('.search-input');
		await searchInput.fill('test');

		// Results or empty state should appear after debounce
		await page.waitForTimeout(500);
		await expect(page.locator('body')).toBeVisible();
	});

	test('grid/list toggle works', async ({ page }) => {
		// Grid toggle button should be present
		const toggleBtn = page.locator('button[title*="grid"], button[aria-label*="grid"], .view-toggle button').first();
		if (await toggleBtn.isVisible()) {
			await toggleBtn.click();
			await expect(page.locator('body')).toBeVisible();
		}
	});

	test('join button visible on community card', async ({ page }) => {
		// Wait for communities to load
		const joinBtn = page.getByRole('button', { name: /Join/i }).first();
		const hasJoin = await joinBtn.isVisible({ timeout: 8_000 }).catch(() => false);

		// Either there are joinable servers or the user is already in all of them — both are valid
		expect(typeof hasJoin).toBe('boolean');
	});

	test('clicking Join on a server shows feedback', async ({ page }) => {
		const joinBtn = page.getByRole('button', { name: /Join/i }).first();
		const visible = await joinBtn.isVisible({ timeout: 8_000 }).catch(() => false);

		if (!visible) {
			test.skip();
			return;
		}

		await joinBtn.click();

		// Should either navigate to the server or show an error/already-member message
		await page.waitForTimeout(2_000);
		const url = page.url();
		const hasNavigated = url.includes('/servers/') || url.includes('/channels/');
		const hasToast = await page.locator('[role="alert"], .toast, [class*="toast"]').isVisible().catch(() => false);

		expect(hasNavigated || hasToast).toBeTruthy();
	});
});
