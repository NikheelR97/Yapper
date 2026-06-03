import { type Page, type Locator, expect } from '@playwright/test';

export class AppShell {
	readonly page: Page;
	readonly loadingOverlay: Locator;
	readonly reconnectingBanner: Locator;
	readonly toasts: Locator;
	readonly keyboardShortcutsModal: Locator;
	readonly exploreLink: Locator;
	readonly dmLink: Locator;
	readonly settingsLink: Locator;
	readonly serversNav: Locator;

	constructor(page: Page) {
		this.page = page;
		this.loadingOverlay = page.locator('[aria-label="Loading Yapper"]');
		this.reconnectingBanner = page.locator('.reconnecting-banner, [data-reconnecting]');
		this.toasts = page.locator('[role="status"], .toast, [data-toast]');
		this.keyboardShortcutsModal = page.locator(
			'[role="dialog"]',
		).filter({ hasText: /Keyboard Shortcut/i });
		const mainNav = page.getByRole('navigation', { name: 'Main navigation' });
		this.exploreLink = mainNav.getByRole('link', { name: 'Explore' });
		this.dmLink = mainNav.getByRole('link', { name: /Direct/i });
		this.settingsLink = page.getByRole('link', { name: 'Settings' });
		this.serversNav = page.locator('.server-strip, [data-testid="server-strip"]');
	}

	async waitForReady(timeout = 45_000): Promise<void> {
		await expect(this.loadingOverlay).toHaveCount(0, { timeout });
	}

	async getToastWithText(text: string | RegExp): Promise<Locator> {
		return this.toasts.filter({ hasText: text });
	}

	async openKeyboardShortcuts(): Promise<void> {
		await this.page.keyboard.press('Control+/');
	}

	async closeKeyboardShortcuts(): Promise<void> {
		await this.page.keyboard.press('Control+/');
	}

	async navigateTo(section: 'explore' | 'dm' | 'settings'): Promise<void> {
		const linkMap = {
			explore: this.exploreLink,
			dm: this.dmLink,
			settings: this.settingsLink,
		};
		await linkMap[section].click();
	}

	async expectReconnectingBannerVisible(): Promise<void> {
		await expect(this.reconnectingBanner).toBeVisible({ timeout: 10_000 });
	}

	async expectReconnectingBannerHidden(): Promise<void> {
		await expect(this.reconnectingBanner).toHaveCount(0, { timeout: 10_000 });
	}
}
