import { type Page, type Locator, expect } from '@playwright/test';

export class LoginPage {
	readonly page: Page;
	readonly email: Locator;
	readonly password: Locator;
	readonly submitBtn: Locator;
	readonly errorBanner: Locator;
	readonly discordBtn: Locator;
	readonly googleBtn: Locator;
	readonly forgotLink: Locator;
	readonly registerLink: Locator;

	constructor(page: Page) {
		this.page = page;
		this.email = page.locator('#email');
		this.password = page.locator('#password');
		this.submitBtn = page.getByRole('button', { name: /Sign In/i });
		this.errorBanner = page.locator('[role="alert"]');
		this.discordBtn = page.getByRole('button', { name: /Discord/i });
		this.googleBtn = page.getByRole('button', { name: /Google/i });
		this.forgotLink = page.getByRole('link', { name: /Forgot password/i });
		this.registerLink = page.getByRole('link', { name: /Join the Hype/i });
	}

	async goto(): Promise<void> {
		await this.page.goto('/login');
	}

	async fillCredentials(email: string, pw: string): Promise<void> {
		await this.email.fill(email);
		await this.password.fill(pw);
	}

	async submit(): Promise<void> {
		await this.submitBtn.click();
	}

	async login(email: string, pw: string): Promise<void> {
		await this.fillCredentials(email, pw);
		await this.submit();
	}

	async expectErrorBanner(): Promise<void> {
		await expect(this.errorBanner).toBeVisible({ timeout: 8_000 });
	}
}
