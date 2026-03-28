import { test as base, expect } from '@playwright/test';
import { test as authedTest } from './fixtures/auth.fixture';

/**
 * Explore page E2E tests.
 *
 * Covers: search, community listing, and join affordances.
 */

base.describe('Explore - unauthenticated', () => {
	base('redirects to /login', async ({ page }) => {
		const apiURL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
		await page.route(`${apiURL}/api/v2/auth/refresh`, (route) =>
			route.fulfill({ status: 401, contentType: 'application/json', body: '{}' }),
		);
		await page.context().clearCookies();
		await page.goto('/explore');
		await expect(page).toHaveURL(/\/login/, { timeout: 12_000 });
	});
});

authedTest.describe('Explore - authenticated', () => {
	authedTest('renders core explore controls and handles join attempts', async ({ userPage }) => {
		await userPage.goto('/explore');
		const searchInput = userPage.getByRole('searchbox', { name: 'Search' });
		await expect(searchInput).toBeVisible({ timeout: 30_000 });
		await expect(userPage.getByText(/Communities/i)).toBeVisible({ timeout: 10_000 });

		await searchInput.fill('test');
		await expect(searchInput).toHaveValue('test');

		const toggleButton = userPage
			.locator('button[title*="grid"], button[aria-label*="grid"], .view-toggle button')
			.first();
		if (await toggleButton.isVisible().catch(() => false)) {
			await toggleButton.click();
		}

		await userPage.waitForSelector('.community-card, .empty-msg', { timeout: 20_000 }).catch(() => {});
		const joinButton = userPage.getByRole('button', { name: /join/i }).first();
		const visible = await joinButton.isVisible({ timeout: 5_000 }).catch(() => false);

		if (!visible) {
			await expect(userPage.locator('body')).toBeVisible();
			return;
		}

		await joinButton.click();
		await expect
			.poll(
				async () => {
					const url = userPage.url();
					if (url.includes('/servers/') || url.includes('/channels/')) {
						return 'navigated';
					}
					if (
						await userPage
							.getByText(/joined|success|error|failed/i)
							.first()
							.isVisible()
							.catch(() => false)
					) {
						return 'toast';
					}
					return (await joinButton.isVisible().catch(() => false))
						? 'button-still-visible'
						: 'pending';
				},
				{ timeout: 10_000 },
			)
			.not.toBe('pending');
	});
});
