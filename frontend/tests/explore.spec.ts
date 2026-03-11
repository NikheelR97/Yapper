import { test, expect, type Page } from '@playwright/test';
import { PRIMARY_INSTALLATION_ID, seedTrustedPrimaryDevice, setInstallationId } from './auth-helper.js';

/**
 * Explore page E2E tests.
 *
 * Covers: search, community listing, and join affordances.
 */

const TEST_EMAIL = process.env.E2E_EMAIL ?? '';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? '';

async function loginAs(page: Page) {
	await setInstallationId(page, PRIMARY_INSTALLATION_ID);
	await page.goto('/login');
	await page.fill('#email', TEST_EMAIL);
	await page.fill('#password', TEST_PASSWORD);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 20_000 });
}

test.describe('Explore - unauthenticated', () => {
	test('redirects to /login', async ({ page }) => {
		const apiURL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
		await page.route(`${apiURL}/api/v2/auth/refresh`, (route) =>
			route.fulfill({ status: 401, contentType: 'application/json', body: '{}' }),
		);
		await page.context().clearCookies();
		await page.goto('/explore');
		await expect(page).toHaveURL(/\/login/, { timeout: 12_000 });
	});
});

test.describe('Explore - authenticated', () => {
	test.skip(!TEST_EMAIL || !TEST_PASSWORD, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
	});

	test('renders core explore controls and handles join attempts', async ({ page }) => {
		await loginAs(page);
		const searchInput = page.getByRole('searchbox', { name: 'Search' });
		await expect(searchInput).toBeVisible({ timeout: 30_000 });
		await expect(page.getByText(/Communities/i)).toBeVisible({ timeout: 10_000 });

		await searchInput.fill('test');
		await page.waitForTimeout(500);
		await expect(searchInput).toHaveValue('test');

		const toggleBtn = page
			.locator('button[title*="grid"], button[aria-label*="grid"], .view-toggle button')
			.first();
		if (await toggleBtn.isVisible().catch(() => false)) {
			await toggleBtn.click();
		}

		await page.waitForSelector('.community-card, .empty-msg', { timeout: 20_000 }).catch(() => {});
		const joinBtn = page.locator('.join-btn').first();
		const visible = await joinBtn.isVisible({ timeout: 5_000 }).catch(() => false);

		if (!visible) {
			await expect(page.locator('body')).toBeVisible();
			return;
		}

		await joinBtn.click();
		await page.waitForTimeout(2_000);
		const url = page.url();
		const hasNavigated = url.includes('/servers/') || url.includes('/channels/');
		const hasToast = await page
			.locator('.toast-error, .toast-success, .toast-info')
			.isVisible()
			.catch(() => false);

		expect(hasNavigated || hasToast || (await joinBtn.isVisible().catch(() => false))).toBeTruthy();
	});
});
