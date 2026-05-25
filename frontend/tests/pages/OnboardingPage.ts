import { type Page, type Locator } from '@playwright/test';

export class OnboardingPage {
	readonly page: Page;
	readonly dots: Locator;
	readonly nextBtn: Locator;
	readonly slideTitle: Locator;

	constructor(page: Page) {
		this.page = page;
		this.dots = page.locator('.dot, [data-onboarding-dot], [aria-label*="step"]');
		this.nextBtn = page.getByRole('button', { name: /Next|Continue|Get Started/i });
		this.slideTitle = page.locator('h1, h2, .slide-title, [data-testid="slide-title"]').first();
	}

	async goto(): Promise<void> {
		await this.page.goto('/onboarding');
	}

	async clickNext(): Promise<void> {
		await this.nextBtn.click();
	}

	async goToStep(n: number): Promise<void> {
		const dot = this.dots.nth(n - 1);
		await dot.click();
	}

	async getSlideTitle(): Promise<string> {
		return (await this.slideTitle.textContent()) ?? '';
	}

	async getDotCount(): Promise<number> {
		return this.dots.count();
	}
}
