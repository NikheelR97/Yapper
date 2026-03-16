import { test, expect } from '@playwright/test';

/**
 * Auth flow E2E tests.
 *
 * These tests run against a live backend. Set E2E_EMAIL / E2E_PASSWORD
 * to the credentials of a pre-existing test account.
 */

const TEST_EMAIL = process.env.E2E_EMAIL ?? 'e2e@test.yapper.internal';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? 'E2eTestPass1!';

// ─── Login page ────────────────────────────────────────────────────────────────

test.describe('Login page', () => {
	test('renders form elements', async ({ page }) => {
		await page.goto('/login');

		await expect(page.locator('h1')).toContainText('Enter the Void');
		await expect(page.locator('#email')).toBeVisible();
		await expect(page.locator('#password')).toBeVisible();
		await expect(page.getByRole('button', { name: /Sign In/i })).toBeVisible();
	});

	test('submit button disabled when fields are empty', async ({ page }) => {
		await page.goto('/login');

		const submitBtn = page.getByRole('button', { name: /Sign In/i });
		await expect(submitBtn).toBeDisabled();
	});

	test('shows error banner on wrong credentials', async ({ page }) => {
		await page.goto('/login');

		await page.fill('#email', 'nobody@nowhere.invalid');
		await page.fill('#password', 'wrongpassword');
		const submitBtn = page.getByRole('button', { name: /Sign In/i });
		await expect(submitBtn).toBeEnabled();
		await submitBtn.click();

		await expect(page.locator('[role="alert"]')).toBeVisible({ timeout: 8_000 });
	});

	test('navigates to /register via link', async ({ page }) => {
		await page.goto('/login');

		await page.getByRole('link', { name: /Join the Hype/i }).click();
		await expect(page).toHaveURL('/register');
	});

	test('navigates to /forgot-password via link', async ({ page }) => {
		await page.goto('/login');

		await page.getByRole('link', { name: /Forgot password/i }).click();
		await expect(page).toHaveURL('/forgot-password');
	});

	test('OAuth buttons are present', async ({ page }) => {
		await page.goto('/login');

		await expect(page.getByRole('button', { name: /Discord/i })).toBeVisible();
		await expect(page.getByRole('button', { name: /Google/i })).toBeVisible();
	});
});

// ─── Register page ─────────────────────────────────────────────────────────────

test.describe('Register page', () => {
	test('renders all form fields', async ({ page }) => {
		await page.goto('/register');

		await expect(page.locator('h1')).toContainText('Join the Hype');
		await expect(page.locator('#username')).toBeVisible();
		await expect(page.locator('#email')).toBeVisible();
		await expect(page.locator('#password')).toBeVisible();
	});

	test('submit button disabled until required fields are filled', async ({ page }) => {
		await page.goto('/register');

		const submitBtn = page.getByRole('button', { name: /Create Account/i });
		await expect(submitBtn).toBeDisabled();

		await page.fill('#username', 'testuser');
		await page.fill('#email', 'test@example.com');
		await page.fill('#password', 'ValidPass1!');

		await expect(submitBtn).toBeEnabled();
	});

	test('password strength indicator appears after typing', async ({ page }) => {
		await page.goto('/register');

		await page.fill('#password', 'weak');
		// The strength bar container and label are always visible once password is non-empty
		await expect(page.locator('.strength-bar')).toBeVisible();
		await expect(page.locator('.strength-label')).toBeVisible();
	});

	test('navigates to /login via sign-in link', async ({ page }) => {
		await page.goto('/register');

		await page.getByRole('link', { name: /Sign in/i }).click();
		await expect(page).toHaveURL('/login');
	});
});

// ─── Login → app navigation ────────────────────────────────────────────────────

test.describe('Authenticated navigation', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run auth tests');

	test('successful login redirects to /explore', async ({ page }) => {
		await page.goto('/login');

		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();

		await expect(page).toHaveURL(/\/explore/, { timeout: 20_000 });
	});
});
