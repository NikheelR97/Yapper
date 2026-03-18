import { type Page, type Locator, expect } from '@playwright/test';

export class ForgotPasswordPage {
	readonly page: Page;
	readonly emailInput: Locator;
	readonly submitBtn: Locator;
	readonly successMessage: Locator;
	readonly backToLoginLink: Locator;

	constructor(page: Page) {
		this.page = page;
		this.emailInput = page.locator('input[type="email"]').first();
		this.submitBtn = page.getByRole('button', { name: /Send|Reset|Submit/i });
		this.successMessage = page.getByText(/Check your inbox|Email sent|Reset link/i);
		this.backToLoginLink = page.getByRole('link', { name: /Back to (login|sign in)/i });
	}

	async goto(): Promise<void> {
		await this.page.goto('/forgot-password');
	}

	async submitEmail(email: string): Promise<void> {
		await this.emailInput.fill(email);
		await this.submitBtn.click();
	}

	async expectSuccess(): Promise<void> {
		await expect(this.successMessage).toBeVisible({ timeout: 8_000 });
	}
}
