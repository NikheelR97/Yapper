import { expect, test, type Page } from '@playwright/test';

/**
 * Auth flow E2E tests.
 *
 * These tests run against a live backend. Set E2E_EMAIL / E2E_PASSWORD
 * to the credentials of a pre-existing test account.
 */

const TEST_EMAIL = process.env.E2E_EMAIL ?? 'e2e@test.yapper.internal';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? 'E2eTestPass1!';

async function gotoLogin(page: Page) {
	await page.goto('/login');
	await page.waitForLoadState('networkidle');
	await expect(page.locator('#email')).toBeEditable();
	await expect(page.locator('#password')).toBeEditable();
}

async function fillLogin(page: Page, email: string, password: string) {
	const emailInput = page.locator('#email');
	const passwordInput = page.locator('#password');

	await emailInput.clear();
	await emailInput.pressSequentially(email);
	await passwordInput.clear();
	await passwordInput.pressSequentially(password);
	await expect(emailInput).toHaveValue(email);
	await expect(passwordInput).toHaveValue(password);
}

test.describe('Login page', () => {
	test('renders form elements @smoke', async ({ page }) => {
		await gotoLogin(page);

		await expect(page.locator('h1')).toContainText('Enter the Void');
		await expect(page.locator('#email')).toBeVisible();
		await expect(page.locator('#password')).toBeVisible();
		await expect(page.getByRole('button', { name: /Sign In/i })).toBeVisible();
	});

	test('submit button disabled when fields are empty @smoke', async ({ page }) => {
		await gotoLogin(page);

		const submitBtn = page.getByRole('button', { name: /Sign In/i });
		await expect(submitBtn).toBeDisabled();
	});

	test('shows error banner on wrong credentials @smoke', async ({ page }) => {
		await gotoLogin(page);

		await fillLogin(page, 'nobody@nowhere.invalid', 'wrongpassword');
		const submitBtn = page.getByRole('button', { name: /Sign In/i });
		await expect(submitBtn).toBeEnabled({ timeout: 10_000 });
		await submitBtn.click();

		await expect(page.locator('[role="alert"]')).toBeVisible({ timeout: 8_000 });
	});

	test('navigates to /register via link @smoke', async ({ page }) => {
		await gotoLogin(page);

		await Promise.all([
			page.waitForURL('**/register'),
			page.getByRole('link', { name: /Join the Hype/i }).click(),
		]);
	});

	test('navigates to /forgot-password via link @smoke', async ({ page }) => {
		await gotoLogin(page);

		await Promise.all([
			page.waitForURL('**/forgot-password'),
			page.getByRole('link', { name: /Forgot password/i }).click(),
		]);
	});

	test('OAuth buttons are present @smoke', async ({ page }) => {
		await gotoLogin(page);

		await expect(page.getByRole('button', { name: /Discord/i })).toBeVisible();
		await expect(page.getByRole('button', { name: /Google/i })).toBeVisible();
	});
});

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
		await expect(page.locator('.strength-bar')).toBeVisible();
		await expect(page.locator('.strength-label')).toBeVisible();
	});

	test('navigates to /login via sign-in link', async ({ page }) => {
		await page.goto('/register');

		await page.getByRole('link', { name: /Sign in/i }).click();
		await expect(page).toHaveURL('/login');
	});
});

test.describe('Authenticated navigation', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run auth tests');

	test('successful login redirects to /explore', async ({ page }) => {
		await gotoLogin(page);

		await fillLogin(page, TEST_EMAIL, TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();

		await expect(page).toHaveURL(/\/explore/, { timeout: 20_000 });
	});
});
