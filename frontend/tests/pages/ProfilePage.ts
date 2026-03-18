import { type Page, type Locator, expect } from '@playwright/test';

export class ProfilePage {
	readonly page: Page;
	readonly displayName: Locator;
	readonly username: Locator;
	readonly bio: Locator;
	readonly followBtn: Locator;
	readonly followerCount: Locator;
	readonly hypeMoments: Locator;
	readonly topCommunities: Locator;
	readonly mutualConnections: Locator;

	constructor(page: Page) {
		this.page = page;
		this.displayName = page.locator(
			'h1, .display-name, [data-testid="display-name"]',
		).first();
		this.username = page.locator(
			'.username, [data-testid="username"], [class*="username"]',
		).first();
		this.bio = page.locator('.bio, [data-testid="bio"]').first();
		this.followBtn = page
			.getByRole('button', { name: /Follow|Following|Unfollow/i })
			.first();
		this.followerCount = page.locator(
			'.follower-count, [data-testid="follower-count"]',
		).first();
		this.hypeMoments = page.locator(
			'.hype-moments, [data-testid="hype-moments"]',
		).first();
		this.topCommunities = page.locator(
			'.top-communities, [data-testid="top-communities"]',
		).first();
		this.mutualConnections = page.locator(
			'.mutual-connections, [data-testid="mutual-connections"]',
		).first();
	}

	async goto(username: string): Promise<void> {
		await this.page.goto(`/profile/${username}`);
	}

	async clickFollow(): Promise<void> {
		await this.followBtn.click();
	}

	async getFollowerCountText(): Promise<string> {
		return (await this.followerCount.textContent()) ?? '0';
	}

	async isFollowingState(): Promise<boolean> {
		const text = (await this.followBtn.textContent()) ?? '';
		return /Following|Unfollow/i.test(text);
	}

	async expectDisplayName(name: string): Promise<void> {
		await expect(this.displayName).toContainText(name, { timeout: 10_000 });
	}

	async expectUsernameContains(username: string): Promise<void> {
		await expect(
			this.page.locator('body'),
		).toContainText(username, { timeout: 10_000 });
	}
}
