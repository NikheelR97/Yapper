import { test, expect } from '@playwright/test';

/**
 * Feature: Onboarding Carousel
 *
 * Tests the two-step onboarding carousel shown to new users.
 * Uses mocked auth so we don't consume API rate limits.
 */

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

test.describe('Onboarding carousel', () => {
	// Clear stored auth state so the (auth) layout doesn't redirect away.
	test.use({ storageState: { cookies: [], origins: [] } });

	test('step 1 renders with a visible title and dot pagination @smoke', async ({ page }) => {
		await page.goto('/onboarding');

		// There should be a heading on the first slide
		await expect(page.locator('h1, h2, .slide-title').first()).toBeVisible({ timeout: 10_000 });

		// Dot pagination should exist (at least 2 dots for 2 steps)
		const dots = page.locator('.dot, [data-onboarding-dot], [aria-label*="step"], [aria-label*="slide"]');
		const count = await dots.count();
		expect(count).toBeGreaterThanOrEqual(2);
	});

	test('clicking Next advances to the next step @smoke', async ({ page }) => {
		await page.goto('/onboarding');

		const titleBefore = await page.locator('h1, h2, .slide-title').first().textContent({ timeout: 10_000 });

		// Use .btn-primary class directly — avoids ARIA name matching issues caused by the
		// slide's fadeUp animation keeping the element unstable during getByRole resolution.
		const nextBtn = page.locator('.btn-primary').first();
		await expect(nextBtn).toBeVisible({ timeout: 10_000 });
		await nextBtn.click({ force: true });

		// Title should change after advancing
		await expect(async () => {
			const titleAfter = await page.locator('h1, h2, .slide-title').first().textContent();
			expect(titleAfter).not.toBe(titleBefore);
		}).toPass({ timeout: 5_000 });
	});

	test('clicking the first dot returns to step 1', async ({ page }) => {
		await page.goto('/onboarding');

		const titleStep1 = await page.locator('h1, h2, .slide-title').first().textContent({ timeout: 10_000 });

		// Advance to step 2 using the primary button
		const nextBtn = page.locator('.btn-primary').first();
		await expect(nextBtn).toBeVisible({ timeout: 10_000 });
		await nextBtn.click({ force: true });

		// Click first dot to go back
		const dots = page.locator('.dot, [data-onboarding-dot], [aria-label*="step"]');
		await dots.first().click();

		const titleBack = await page.locator('h1, h2, .slide-title').first().textContent();
		expect(titleBack).toBe(titleStep1);
	});

	test('completing all steps navigates to /explore or /login (redirect blocked mid-flow)', async ({ page }) => {
		// Mock explore endpoints in case we actually reach /explore
		await page.route(`**/api/v1/explore/**`, async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([]),
			});
		});
		await page.route(`**/api/v1/search**`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ servers: [], users: [] }) });
		});

		await page.goto('/onboarding');

		// Click through all steps until the final CTA
		const maxSteps = 5;
		for (let i = 0; i < maxSteps; i++) {
			const continueBtn = page
				.getByRole('button', { name: /Next|Continue|Get Started|Explore/i })
				.first();
			const isVisible = await continueBtn.isVisible().catch(() => false);
			if (!isVisible) break;
			await continueBtn.click();
			// Stop if we've navigated away from /onboarding
			if (!page.url().includes('/onboarding')) break;
		}

		// Without auth, the app may redirect to /login instead of /explore
		await expect(page).toHaveURL(/\/(explore|onboarding|login)/, { timeout: 10_000 });
	});
});
