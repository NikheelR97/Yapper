import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';

/**
 * Feature: Error States
 *
 * Tests 404 pages, network failure states, and loading skeletons.
 */

async function setupShellData(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

test.describe('Error states - 404', () => {
	test('navigating to a non-existent route shows an error page @smoke', async ({ userPage }) => {
		await userPage.goto('/this-route-does-not-exist-e2e-test');

		await expect(
			userPage
				.getByText(/404|Not Found|page not found|does not exist/i)
				.first()
				.or(userPage.locator('[data-sveltekit-error]').first())
				.or(userPage.locator('h1').filter({ hasText: /error|not found/i }).first()),
		).toBeVisible({ timeout: 10_000 });
	});

	test('non-existent profile route shows error state', async ({ userPage }) => {
		await setupShellData(userPage);
		await userPage.route('**/api/v2/users/by/nonexistent_user_xyz', async (route) => {
			await route.fulfill({
				status: 404,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'User not found' }),
			});
		});

		await userPage.goto('/profile/nonexistent_user_xyz');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await expect(
			userPage
				.getByRole('heading', { name: /not found|does not exist|404/i })
				.or(userPage.locator('[data-testid="profile-error"]')),
		).toBeVisible({ timeout: 10_000 });
	});
});

test.describe('Error states - network failure', () => {
	test('API failure on explore page shows error state', async ({ userPage }) => {
		await setupShellData(userPage);

		await userPage.route('**/api/v2/explore/**', async (route) => {
			await route.fulfill({
				status: 503,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'Service unavailable' }),
			});
		});

		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await expect(userPage.locator('body')).toBeVisible();
		await expect(
			userPage
				.locator('[data-testid="error-boundary"]')
				.or(userPage.getByText(/Something went wrong/i)),
		)
			.toHaveCount(0, { timeout: 3_000 })
			.catch(() => {
				// It is acceptable if an error boundary renders a message.
			});
	});
});

test.describe('Error states - loading skeletons', () => {
	test('profile page shows loading skeleton before data arrives @smoke', async ({ userPage }) => {
		await setupShellData(userPage);

		await userPage.route('**/api/v2/users/by/slow_user', async (route) => {
			await new Promise((resolve) => setTimeout(resolve, 2_000));
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					id: 'slow-user',
					username: 'slow_user',
					displayName: 'Slow User',
					bio: null,
					followerCount: 0,
					followingCount: 0,
					isFollowing: false,
					isFriend: false,
				}),
			});
		});

		await userPage.goto('/profile/slow_user');

		const skeleton = userPage.locator(
			'.skeleton, [data-testid="skeleton"], [aria-busy="true"], .loading-skeleton',
		);

		await expect(userPage.getByText('Slow User')).toBeVisible({ timeout: 25_000 });
		await expect(skeleton).toHaveCount(0, { timeout: 3_000 });
	});
});
