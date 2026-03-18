import { type Page, type Locator, expect } from '@playwright/test';

export class ChannelPage {
	readonly page: Page;
	readonly messageInput: Locator;
	readonly sendBtn: Locator;
	readonly messageList: Locator;
	readonly typingIndicator: Locator;
	readonly canvasToggle: Locator;

	constructor(page: Page) {
		this.page = page;
		this.messageInput = page.locator('textarea[aria-label="Message"]').first();
		this.sendBtn = page.getByRole('button', { name: /Send/i }).or(page.locator('[aria-label="Send message"]'));
		this.messageList = page.locator('.message-list, [role="log"]');
		this.typingIndicator = page.locator('.typing-indicator, [data-testid="typing-indicator"]');
		this.canvasToggle = page.locator('[aria-label*="canvas"], [data-testid="canvas-toggle"]');
	}

	async goto(serverId: string, channelId: string): Promise<void> {
		await this.page.goto(`/servers/${serverId}/channels/${channelId}`);
	}

	async waitForInput(timeout = 60_000): Promise<void> {
		await expect(this.messageInput).toBeEnabled({ timeout });
	}

	async sendMessage(text: string): Promise<void> {
		await this.messageInput.fill(text);
		await this.messageInput.press('Enter');
	}

	async isMessageVisible(text: string, timeout = 10_000): Promise<void> {
		await expect(this.page.getByText(text)).toBeVisible({ timeout });
	}

	async openCanvas(): Promise<void> {
		await this.canvasToggle.click();
	}
}
