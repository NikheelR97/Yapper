/**
 * Codex Security Report - Section 3.1
 * Feature: Brute-Force Form Resiliency and Rate Limiting
 *
 * @security @auth @mobile-layout
 */

import { test, expect, type Page } from '@playwright/test';
import { log } from './helpers/log.js';

test.use({ viewport: { width: 375, height: 667 } }); // iPhone SE

const bruteForceEmail = 'attacker@evil.com';
const bruteForcePassword = 'wrongpassword';

async function readyLoginForm(page: Page) {
	const emailInput = page.locator('#email');
	const passwordInput = page.locator('#password');
	const submitBtn = page.getByRole('button', { name: /Sign In/i });

	await expect(emailInput).toBeEditable({ timeout: 10_000 });
	await expect(passwordInput).toBeEditable({ timeout: 10_000 });
	await emailInput.clear();
	await emailInput.pressSequentially(bruteForceEmail);
	await passwordInput.clear();
	await passwordInput.pressSequentially(bruteForcePassword);
	await expect(emailInput).toHaveValue(bruteForceEmail);
	await expect(passwordInput).toHaveValue(bruteForcePassword);
	await expect(submitBtn).toBeEnabled({ timeout: 10_000 });

	return submitBtn;
}

test.describe('Brute-force auth @security @auth @mobile-layout', () => {
	test('successive authentication failures trigger rate-limit lockdown @smoke', async ({ page }) => {
		let loginAttempts = 0;

		log('SECURITY', 'SETUP', 'Mounting login endpoint interceptor (mobile viewport 375x667)');

		// Mock /api/v2/auth/login: 401 for first 4 attempts, 429 on the 5th.
		await page.route('**/api/v2/auth/login', async (route) => {
			loginAttempts++;
			log('AUTH', 'INTERCEPT', `Login attempt #${loginAttempts} - method: ${route.request().method()}`);

			if (loginAttempts >= 5) {
				log('NETWORK', 'RATE_LIMIT', 'Returning 429 - rate limit threshold reached');
				await route.fulfill({
					status: 429,
					contentType: 'application/json',
					body: JSON.stringify({ error: 'Too many attempts. Please try again later.' }),
				});
			} else {
				await route.fulfill({
					status: 401,
					contentType: 'application/json',
					body: JSON.stringify({ error: 'Invalid credentials' }),
				});
			}
		});

		log('NETWORK', 'NAVIGATE', 'Loading /login on mobile viewport');
		await page.goto('/login');
		await page.waitForLoadState('networkidle');
		await expect(page.locator('#email')).toBeVisible({ timeout: 10_000 });

		log('SECURITY', 'BRUTE_FORCE', 'Submitting 4 invalid attempts (expecting 401 each)');

		for (let i = 1; i <= 4; i++) {
			const submitBtn = await readyLoginForm(page);

			log('AUTH', 'SUBMIT', `Submitting attempt ${i} of 5`);
			await Promise.all([
				page.waitForResponse((response) =>
					response.url().includes('/api/v2/auth/login') && response.status() === 401
				),
				submitBtn.click(),
			]);

			await expect(page.locator('.error-banner')).toContainText('Invalid email or password.', {
				timeout: 8_000,
			});
			await expect(submitBtn).toBeEnabled({ timeout: 10_000 });
			log('ASSERTION', 'STATE', `Attempt ${i}: 401 confirmed - "Invalid email or password."`);
		}

		const submitBtn = await readyLoginForm(page);

		log('SECURITY', 'BRUTE_FORCE', 'Submitting 5th attempt - expecting 429 rate-limit response');
		await Promise.all([
			page.waitForResponse((response) =>
				response.url().includes('/api/v2/auth/login') && response.status() === 429
			),
			submitBtn.click(),
		]);

		const errorBanner = page.locator('.error-banner');
		await expect(errorBanner).toBeVisible({ timeout: 10_000 });
		await expect(errorBanner).toContainText('Too many failed attempts. Try again in 15 minutes.');

		log('VALIDATION', 'UI', '"Too many failed attempts" message confirmed on mobile viewport. [PASS]');

		expect(loginAttempts).toBe(5);
		log('ASSERTION', 'STATE', `Total intercepted login requests: ${loginAttempts}. [PASS]`);
	});
});
