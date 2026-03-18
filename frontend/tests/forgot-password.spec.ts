import { test, expect } from '@playwright/test';

/**
 * Feature: Forgot / Reset Password
 *
 * Tests the forgot-password form flow.
 * These are UI-only tests — no live API calls required.
 */

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

test.describe('Forgot password page', () => {
	// (auth) layout redirects authenticated users to /explore.
	// Clear saved auth so we land on the forgot-password form.
	test.use({ storageState: { cookies: [], origins: [] } });

	test.beforeEach(async ({ page }) => {
		// Mock the reset endpoint so we don't need a live backend
		await page.route(`**/api/v1/auth/forgot-password`, async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ message: 'Reset email sent' }),
			});
		});
	});

	test('renders the heading and email input @smoke', async ({ page }) => {
		await page.goto('/forgot-password');

		await expect(page.locator('h1, h2').first()).toBeVisible();
		await expect(page.locator('input[type="email"]').first()).toBeVisible();
	});

	test('submit button is disabled when email is empty @smoke', async ({ page }) => {
		await page.goto('/forgot-password');

		const submitBtn = page.getByRole('button', { name: /Send|Reset|Submit/i });
		await expect(submitBtn).toBeDisabled();
	});

	test('submit button becomes enabled after typing a valid email', async ({ page }) => {
		await page.goto('/forgot-password');

		const emailInput = page.locator('input[type="email"]').first();
		const submitBtn = page.getByRole('button', { name: /Send|Reset|Submit/i });

		await emailInput.fill('test@example.com');
		await expect(submitBtn).toBeEnabled();
	});

	test('submitting a valid email shows the success confirmation', async ({ page }) => {
		await page.goto('/forgot-password');

		const emailInput = page.locator('input[type="email"]').first();
		const submitBtn = page.getByRole('button', { name: /Send|Reset|Submit/i });

		await emailInput.fill('test@example.com');
		await submitBtn.click();

		await expect(
			page.getByText(/Check your inbox|Email sent|Reset link/i),
		).toBeVisible({ timeout: 8_000 });
	});

	test('clicking the back-to-login link navigates to /login', async ({ page }) => {
		await page.goto('/forgot-password');

		const backLink = page
			.getByRole('link', { name: /Back to (login|sign in)|Sign in/i })
			.or(page.locator('a[href="/login"]'))
			.first();
		await expect(backLink).toBeVisible();
		await backLink.click();
		await expect(page).toHaveURL('/login');
	});
});
