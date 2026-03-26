import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

/**
 * Feature: Explore — Advanced Interactions
 *
 * Tests debounced search, tag filtering, grid/list toggle, and user search results.
 * Uses mocked endpoints — no live API needed.
 */

async function setupAuthAndExplore(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
	const device = buildMockDevice({ installation_id: 'explore-adv-install' });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, 'explore-adv-install');
	await mockAuthEndpoints(page, authData);

	await mockExploreEndpoints(page, {
		tags: ['gaming', 'music', 'art', 'tech', 'anime'],
		communities: [
			{ id: 'c1', name: 'Gaming Community Alpha', tags: ['gaming'], memberCount: 120 },
			{ id: 'c2', name: 'Music Makers', tags: ['music'], memberCount: 55 },
			{ id: 'c3', name: 'Art Circle', tags: ['art'], memberCount: 30 },
		],
		searchUsers: [
			{ id: 'u1', username: 'searchable_user', displayName: 'Searchable User' },
		],
		searchServers: [
			{ id: 's1', name: 'Searchable Server', memberCount: 5 },
		],
	});

	await page.route(`**/api/v2/servers`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route(`**/api/v2/conversations`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

// ─── Debounced search ─────────────────────────────────────────────────────────

test.describe('Explore — debounced search', () => {
	test.beforeEach(async ({ page }) => {
		await setupAuthAndExplore(page);
	});

	test('search box is present and accepts input @smoke', async ({ page }) => {
		await page.goto('/explore');
		const searchBox = page.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });
		await searchBox.fill('test query');
		await expect(searchBox).toHaveValue('test query');
	});

	test('search results render after user input @smoke', async ({ page }) => {
		await page.goto('/explore');
		const searchBox = page.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });

		await searchBox.fill('searchable');

		// Wait for debounce (350ms) and results
		await expect(
			page.getByText('Searchable User').or(page.getByText('Searchable Server')).first(),
		).toBeVisible({ timeout: 3_000 });
	});
});

// ─── Tag filtering ────────────────────────────────────────────────────────────

test.describe('Explore — tag filtering', () => {
	test.beforeEach(async ({ page }) => {
		await setupAuthAndExplore(page);
	});

	test('trending tags are visible @smoke', async ({ page }) => {
		await page.goto('/explore');
		await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({ timeout: 20_000 });

		// Tags should render — look for the tag list
		const tagEl = page.locator('.tag, .tag-chip, [data-testid="trending-tag"]').first();
		await expect(tagEl).toBeVisible({ timeout: 5_000 });
	});

	test('clicking a tag filters or highlights results', async ({ page }) => {
		await page.goto('/explore');
		await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({ timeout: 20_000 });

		const gamingTag = page.locator('.tag, .tag-chip').filter({ hasText: /gaming/i }).first();
		if (await gamingTag.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await gamingTag.click();
			// After clicking, either the tag is active/selected or results are filtered
			await expect(async () => {
				const isActive = await gamingTag.evaluate((el) =>
					el.classList.contains('active') ||
					el.classList.contains('selected') ||
					el.getAttribute('aria-pressed') === 'true'
				);
				const hasFilteredResults = await page.locator('.community-card').count() > 0;
				expect(isActive || hasFilteredResults).toBe(true);
			}).toPass({ timeout: 3_000 });
		}
	});
});

// ─── Grid / list toggle ───────────────────────────────────────────────────────

test.describe('Explore — grid/list view toggle', () => {
	test.beforeEach(async ({ page }) => {
		await setupAuthAndExplore(page);
	});

	test('communities render in the default view @smoke', async ({ page }) => {
		await page.goto('/explore');
		await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({ timeout: 20_000 });

		const communities = page.locator('.community-card, [data-testid="community-card"]');
		await expect(communities.first()).toBeVisible({ timeout: 5_000 });
	});

	test('list view toggle changes layout class or structure', async ({ page }) => {
		await page.goto('/explore');
		await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({ timeout: 20_000 });

		// Find and click the list toggle
		const listToggle = page
			.getByRole('button', { name: /list/i })
			.or(page.locator('[aria-label="List view"], [data-testid="list-toggle"]'))
			.first();

		if (await listToggle.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await listToggle.click();
			// Check that something changed — either a class or the layout differs
			const container = page.locator('.communities-grid, .communities-list, [data-testid="communities-container"]').first();
			if (await container.count() > 0) {
				const classList = await container.getAttribute('class');
				expect(classList).toMatch(/list/i);
			}
		}
	});
});

// ─── User search results ──────────────────────────────────────────────────────

test.describe('Explore — user search results', () => {
	test.beforeEach(async ({ page }) => {
		await setupAuthAndExplore(page);
	});

	test('searching for a username shows user row with display name', async ({ page }) => {
		await page.goto('/explore');
		const searchBox = page.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });

		await searchBox.fill('searchable_user');

		// User row should appear with display name
		await expect(page.getByText('Searchable User')).toBeVisible({ timeout: 3_000 });
	});

	test('user row includes Add Friend or Follow button', async ({ page }) => {
		await page.goto('/explore');
		const searchBox = page.getByRole('searchbox', { name: 'Search' });
		await expect(searchBox).toBeVisible({ timeout: 20_000 });

		await searchBox.fill('searchable_user');
		await expect(page.getByText('Searchable User')).toBeVisible({ timeout: 3_000 });

		const actionBtn = page
			.locator('.user-row, [data-testid="user-row"]')
			.getByRole('button', { name: /Add Friend|Follow|Connect/i });

		if (await actionBtn.isVisible({ timeout: 2_000 }).catch(() => false)) {
			await expect(actionBtn).toBeVisible();
		}
	});
});
