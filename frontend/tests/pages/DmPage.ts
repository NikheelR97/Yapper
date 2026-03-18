import { type Page, type Locator, expect } from '@playwright/test';

export class DmPage {
	readonly page: Page;
	readonly conversationList: Locator;
	readonly messageInput: Locator;
	readonly sendBtn: Locator;
	readonly yapBtn: Locator;
	readonly clipBtn: Locator;
	readonly emojiPickerBtn: Locator;
	readonly safetyNumbersBtn: Locator;
	readonly typingIndicator: Locator;

	constructor(page: Page) {
		this.page = page;
		this.conversationList = page.locator('.conv-list, [data-testid="conversation-list"]');
		this.messageInput = page.locator('textarea[aria-label="Message"]').first();
		this.sendBtn = page.getByRole('button', { name: /Send/i }).or(page.locator('[aria-label="Send message"]'));
		this.yapBtn = page.locator('[aria-label*="Yap"], [aria-label*="Record a Yap"], [data-testid="yap-btn"]');
		this.clipBtn = page.locator('[aria-label*="Clip"], [aria-label*="Record a Clip"], [data-testid="clip-btn"]');
		this.emojiPickerBtn = page.locator('[aria-label*="Emoji"], [aria-label*="emoji"], [data-testid="emoji-btn"]');
		this.safetyNumbersBtn = page.locator('[aria-label*="safety"], [aria-label*="Security"], [data-testid="safety-numbers-btn"]');
		this.typingIndicator = page.locator('.typing-indicator, [data-testid="typing-indicator"]');
	}

	async goto(conversationId?: string): Promise<void> {
		if (conversationId) {
			await this.page.goto(`/dm/${conversationId}`);
		} else {
			await this.page.goto('/dm');
		}
	}

	async selectConversation(label: string): Promise<void> {
		await this.page
			.locator('.conv-btn, [data-testid="conversation-item"]')
			.filter({ hasText: label })
			.first()
			.click();
	}

	async sendMessage(text: string): Promise<void> {
		await this.messageInput.fill(text);
		await this.messageInput.press('Enter');
	}

	async isMessageVisible(text: string, timeout = 10_000): Promise<void> {
		await expect(this.page.getByText(text)).toBeVisible({ timeout });
	}

	async getMessages(): Promise<Locator> {
		return this.page.locator('.bubble, [data-testid="message-bubble"]');
	}
}
