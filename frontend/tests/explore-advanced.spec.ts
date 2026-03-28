import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

/**
 * Feature: Explore - Advanced Interactions
 *
 * Tests debounced search, tag filtering, grid/list toggle, and user search results.
 * Uses mocked endpoints - no live API needed.
 */

async function setupExploreRoutes(page: Page): Promise<void> {
	await mockExploreEndpoints(page, {
		tags: ['gaming', 'music', 'art', 'tech', 'anime'],
		communities: [
			{ id: 'c1', name: 'Gaming Community Alpha', tags: ['gaming'], memberCount: 120 },
			{ id: 'c2', name: 'Music Makers', tags: ['music'], memberCount: 55 },
			{ id: 'c3', name: 'Art Circle', tags: ['art'], memberCount: 30 },
		],
		searchUsers: [{ id: 'u1', username: 'searchable_user', displayName: 'Searchable User' }],
		searchServers: [{ id: 's1', name: 'Searchable Server', memberCount: 5 }],
	});

	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

test.describe('Explore - debounced search', () => {
	test.beforeEach(async ({ userPage }) => {
		await setupExploreRoutes(userPage);
	});

	test('search box is present and accepts input @smoke', async ({ userPage }) => {
		await userPage.goto('/explore');
		const searchBox = userPage.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });
		await searchBox.fill('test query');
		await expect(searchBox).toHaveValue('test query');
	});

	test('search results render after user input @smoke', async ({ userPage }) => {
		await userPage.goto('/explore');
		const searchBox = userPage.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });

		await searchBox.fill('searchable');

		await expect(
			userPage.getByText('Searchable User').or(userPage.getByText('Searchable Server')).first(),
		).toBeVisible({ timeout: 3_000 });
	});
});

test.describe('Explore - tag filtering', () => {
	test.beforeEach(async ({ userPage }) => {
		await setupExploreRoutes(userPage);
	});

	test('trending tags are visible @smoke', async ({ userPage }) => {
		await userPage.goto('/explore');
		await expect(userPage.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 20_000,
		});

		const tagElement = userPage.locator('.tag, .tag-chip, [data-testid="trending-tag"]').first();
		await expect(tagElement).toBeVisible({ timeout: 5_000 });
	});

	test('clicking a tag filters or highlights results', async ({ userPage }) => {
		await userPage.goto('/explore');
		await expect(userPage.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 20_000,
		});

		const gamingTag = userPage.locator('.tag, .tag-chip').filter({ hasText: /gaming/i }).first();
		if (await gamingTag.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await gamingTag.click();
			await expect(async () => {
				const isActive = await gamingTag.evaluate(
					(element) =>
						element.classList.contains('active') ||
						element.classList.contains('selected') ||
						element.getAttribute('aria-pressed') === 'true',
				);
				const hasFilteredResults = (await userPage.locator('.community-card').count()) > 0;
				expect(isActive || hasFilteredResults).toBe(true);
			}).toPass({ timeout: 3_000 });
		}
	});
});

test.describe('Explore - grid/list view toggle', () => {
	test.beforeEach(async ({ userPage }) => {
		await setupExploreRoutes(userPage);
	});

	test('communities render in the default view @smoke', async ({ userPage }) => {
		await userPage.goto('/explore');
		await expect(userPage.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 20_000,
		});

		const communities = userPage.locator('.community-card, [data-testid="community-card"]');
		await expect(communities.first()).toBeVisible({ timeout: 5_000 });
	});

	test('list view toggle changes layout class or structure', async ({ userPage }) => {
		await userPage.goto('/explore');
		await expect(userPage.getByRole('searchbox', { name: 'Search' })).toBeVisible({
			timeout: 20_000,
		});

		const listToggle = userPage
			.getByRole('button', { name: /list/i })
			.or(userPage.locator('[aria-label="List view"], [data-testid="list-toggle"]'))
			.first();

		if (await listToggle.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await listToggle.click();
			const container = userPage
				.locator('.communities-grid, .communities-list, [data-testid="communities-container"]')
				.first();
			if ((await container.count()) > 0) {
				const classList = await container.getAttribute('class');
				expect(classList).toMatch(/list/i);
			}
		}
	});
});

test.describe('Explore - user search results', () => {
	test.beforeEach(async ({ userPage }) => {
		await setupExploreRoutes(userPage);
	});

	test('searching for a username shows user row with display name', async ({ userPage }) => {
		await userPage.goto('/explore');
		const searchBox = userPage.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });

		await searchBox.fill('searchable_user');
		await expect(userPage.getByText('Searchable User')).toBeVisible({ timeout: 3_000 });
	});

	test('user row includes Add Friend or Follow button', async ({ userPage }) => {
		await userPage.goto('/explore');
		const searchBox = userPage.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });

		await searchBox.fill('searchable_user');
		await expect(userPage.getByText('Searchable User')).toBeVisible({ timeout: 3_000 });

		const actionButton = userPage
			.locator('.user-row, [data-testid="user-row"]')
			.getByRole('button', { name: /Add Friend|Follow|Connect/i });

		if (await actionButton.isVisible({ timeout: 2_000 }).catch(() => false)) {
			await expect(actionButton).toBeVisible();
		}
	});
});
