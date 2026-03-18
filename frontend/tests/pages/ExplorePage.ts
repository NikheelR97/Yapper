import { type Page, type Locator, expect } from '@playwright/test';

export class ExplorePage {
	readonly page: Page;
	readonly searchBox: Locator;
	readonly communityCards: Locator;
	readonly liveServerCards: Locator;
	readonly trendingTags: Locator;
	readonly gridToggle: Locator;
	readonly listToggle: Locator;
	readonly userRows: Locator;

	constructor(page: Page) {
		this.page = page;
		this.searchBox = page.getByRole('searchbox', { name: 'Search' });
		this.communityCards = page.locator('.community-card, [data-testid="community-card"]');
		this.liveServerCards = page.locator('.live-server-card, [data-testid="live-server-card"]');
		this.trendingTags = page.locator('.tag, .tag-chip, [data-testid="trending-tag"]');
		this.gridToggle = page.getByRole('button', { name: /grid/i }).or(page.locator('[aria-label="Grid view"]'));
		this.listToggle = page.getByRole('button', { name: /list/i }).or(page.locator('[aria-label="List view"]'));
		this.userRows = page.locator('.user-row, [data-testid="user-row"]');
	}

	async goto(): Promise<void> {
		await this.page.goto('/explore');
	}

	async search(query: string): Promise<void> {
		await this.searchBox.fill(query);
	}

	async clearSearch(): Promise<void> {
		await this.searchBox.clear();
	}

	async clickTag(tagName: string): Promise<void> {
		await this.trendingTags.filter({ hasText: tagName }).first().click();
	}

	async toggleListView(): Promise<void> {
		await this.listToggle.click();
	}

	async toggleGridView(): Promise<void> {
		await this.gridToggle.click();
	}

	async getJoinButtonForServer(serverName: string): Promise<Locator> {
		return this.page
			.locator('.community-card, .live-server-card')
			.filter({ hasText: serverName })
			.getByRole('button', { name: /Join/i });
	}

	async getAddFriendButtonForUser(username: string): Promise<Locator> {
		return this.page
			.locator('.user-row')
			.filter({ hasText: username })
			.getByRole('button', { name: /Add Friend|Follow/i });
	}

	async waitForResults(timeout = 5_000): Promise<void> {
		await expect(
			this.communityCards.first().or(this.userRows.first()),
		).toBeVisible({ timeout });
	}
}
