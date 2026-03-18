import { type Page, type Locator, expect } from '@playwright/test';

export class RegisterPage {
	readonly page: Page;
	readonly username: Locator;
	readonly email: Locator;
	readonly password: Locator;
	readonly displayName: Locator;
	readonly submitBtn: Locator;
	readonly strengthBar: Locator;
	readonly strengthLabel: Locator;
	readonly loginLink: Locator;

	constructor(page: Page) {
		this.page = page;
		this.username = page.locator('#username');
		this.email = page.locator('#email');
		this.password = page.locator('#password');
		this.displayName = page.locator('#displayName').or(page.locator('#display_name'));
		this.submitBtn = page.getByRole('button', { name: /Create Account/i });
		this.strengthBar = page.locator('.strength-bar');
		this.strengthLabel = page.locator('.strength-label');
		this.loginLink = page.getByRole('link', { name: /Sign in/i });
	}

	async goto(): Promise<void> {
		await this.page.goto('/register');
	}

	async fillForm(data: { username: string; email: string; password: string; displayName?: string }): Promise<void> {
		await this.username.fill(data.username);
		await this.email.fill(data.email);
		await this.password.fill(data.password);
		if (data.displayName) {
			await this.displayName.fill(data.displayName);
		}
	}

	async submit(): Promise<void> {
		await this.submitBtn.click();
	}

	async expectStrengthBarVisible(): Promise<void> {
		await expect(this.strengthBar).toBeVisible();
		await expect(this.strengthLabel).toBeVisible();
	}
}
