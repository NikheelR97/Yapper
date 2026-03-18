import { type Page, type Locator, expect } from '@playwright/test';

export type SettingsSection =
	| 'My Profile'
	| 'Privacy & Safety'
	| 'Change Password'
	| 'Appearance'
	| 'Voice & Video'
	| 'Notifications'
	| 'Yapper Premium'
	| 'Connected Accounts'
	| 'Family Controls'
	| 'For Developers'
	| 'Support'
	| 'About';

export class SettingsPage {
	readonly page: Page;
	readonly exportBtn: Locator;
	readonly logOutBtn: Locator;
	readonly deleteAccountBtn: Locator;

	constructor(page: Page) {
		this.page = page;
		this.exportBtn = page.getByRole('button', { name: /Export My Data/i });
		this.logOutBtn = page.getByRole('button', { name: /Log Out/i });
		this.deleteAccountBtn = page.getByRole('button', { name: /Delete Account/i });
	}

	async goto(): Promise<void> {
		await this.page.goto('/settings');
	}

	async navigateToSection(section: SettingsSection): Promise<void> {
		await this.page.getByRole('button', { name: section }).click();
	}

	async getSectionHeading(heading: string): Promise<Locator> {
		return this.page.getByRole('heading', { name: heading, exact: true });
	}

	async revokeDevice(label: string): Promise<void> {
		const deviceRow = this.page
			.locator('.device-item, [data-testid="device-item"]')
			.filter({ hasText: label });
		await deviceRow.getByRole('button', { name: /Revoke|Remove/i }).click();
	}

	async getDeviceRows(): Promise<Locator> {
		return this.page.locator('.device-item, [data-testid="device-item"]');
	}

	async exportData(): Promise<void> {
		await this.exportBtn.click();
	}

	async logout(): Promise<void> {
		await this.logOutBtn.click();
	}

	async clickDeleteAccount(): Promise<void> {
		await this.deleteAccountBtn.click();
	}

	async cancelDeleteAccount(): Promise<void> {
		await this.page.getByRole('button', { name: 'Cancel' }).click();
	}
}
