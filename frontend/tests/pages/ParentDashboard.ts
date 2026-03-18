import { type Page, type Locator, expect } from '@playwright/test';

export class ParentDashboard {
	readonly page: Page;
	readonly childSelector: Locator;
	readonly alertsPanel: Locator;
	readonly activitySnapshot: Locator;
	readonly safetyFeed: Locator;
	readonly pendingAlerts: Locator;

	constructor(page: Page) {
		this.page = page;
		this.childSelector = page.locator('.child-selector, [data-testid="child-selector"]').first();
		this.alertsPanel = page.locator('.alerts-panel, [data-testid="alerts-panel"], .pending-alerts').first();
		this.activitySnapshot = page.locator('.activity-snapshot, [data-testid="activity-snapshot"]').first();
		this.safetyFeed = page.locator('.safety-feed, [data-testid="safety-feed"]').first();
		this.pendingAlerts = page.locator('.alert-item, [data-testid="alert-item"]');
	}

	async goto(): Promise<void> {
		await this.page.goto('/parent/dashboard');
	}

	async selectChild(name: string): Promise<void> {
		await this.childSelector.locator(`option, button, [role="option"]`).filter({ hasText: name }).click();
	}

	async getAlertCount(): Promise<number> {
		return this.pendingAlerts.count();
	}

	async approveAlert(index = 0): Promise<void> {
		await this.pendingAlerts.nth(index).getByRole('button', { name: /Approve/i }).click();
	}

	async denyAlert(index = 0): Promise<void> {
		await this.pendingAlerts.nth(index).getByRole('button', { name: /Deny|Reject/i }).click();
	}
}
