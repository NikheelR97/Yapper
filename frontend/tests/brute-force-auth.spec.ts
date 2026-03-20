/**
 * Codex Security Report — Section 3.1
 * Feature: Brute-Force Form Resiliency and Rate Limiting
 *
 * @security @auth @mobile-layout
 */

import { test, expect } from '@playwright/test';
import { log } from './helpers/log.js';

test.use({ viewport: { width: 375, height: 667 } }); // iPhone SE

test.describe('Brute-force auth @security @auth @mobile-layout', () => {
	test('successive authentication failures trigger rate-limit lockdown @smoke', async ({ page }) => {
		let loginAttempts = 0;

		log('SECURITY', 'SETUP', 'Mounting login endpoint interceptor (mobile viewport 375×667)');

		// Mock /api/v2/auth/login: 401 for first 4 attempts, 429 on the 5th.
		await page.route('**/api/v2/auth/login', async (route) => {
			loginAttempts++;
			log('AUTH', 'INTERCEPT', `Login attempt #${loginAttempts} — method: ${route.request().method()}`);

			if (loginAttempts >= 5) {
				log('NETWORK', 'RATE_LIMIT', 'Returning 429 — rate limit threshold reached');
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
		await expect(page.locator('#email')).toBeVisible({ timeout: 10_000 });

		// Fill credentials once — inputs retain their values across failed attempts.
		await page.locator('#email').fill('attacker@evil.com');
		await page.locator('#password').fill('wrongpassword');

		log('SECURITY', 'BRUTE_FORCE', 'Submitting 4 invalid attempts (expecting 401 each)');

		// Attempts 1–4: expect "Invalid email or password." after each click.
		for (let i = 1; i <= 4; i++) {
			const submitBtn = page.locator('button[type="submit"]');
			await expect(submitBtn).toBeEnabled({ timeout: 5_000 });

			log('AUTH', 'SUBMIT', `Submitting attempt ${i} of 5`);
			await submitBtn.click();

			await expect(page.locator('.error-banner')).toContainText('Invalid email or password.', {
				timeout: 8_000,
			});
			log('ASSERTION', 'STATE', `Attempt ${i}: 401 confirmed — "Invalid email or password."`);
		}

		// Attempt 5: expect the 429 rate-limit message.
		const submitBtn = page.locator('button[type="submit"]');
		await expect(submitBtn).toBeEnabled({ timeout: 5_000 });

		log('SECURITY', 'BRUTE_FORCE', 'Submitting 5th attempt — expecting 429 rate-limit response');
		await submitBtn.click();

		const errorBanner = page.locator('.error-banner');
		await expect(errorBanner).toBeVisible({ timeout: 10_000 });
		await expect(errorBanner).toContainText('Too many failed attempts. Try again in 15 minutes.');

		log('VALIDATION', 'UI', '"Too many failed attempts" message confirmed on mobile viewport. [PASS]');

		// Confirm all 5 requests were intercepted.
		expect(loginAttempts).toBe(5);
		log('ASSERTION', 'STATE', `Total intercepted login requests: ${loginAttempts}. [PASS]`);
	});
});
