/**
 * Extended Playwright test fixtures.
 *
 * Import `test` and `expect` from this file in specs that use Page Objects.
 * Specs that only need the standard Playwright API can still import from
 * '@playwright/test' directly.
 */

import { test as base, expect } from '@playwright/test';
import { LoginPage } from './pages/LoginPage.js';
import { RegisterPage } from './pages/RegisterPage.js';
import { ForgotPasswordPage } from './pages/ForgotPasswordPage.js';
import { OnboardingPage } from './pages/OnboardingPage.js';
import { ExplorePage } from './pages/ExplorePage.js';
import { DmPage } from './pages/DmPage.js';
import { ChannelPage } from './pages/ChannelPage.js';
import { SettingsPage } from './pages/SettingsPage.js';
import { ProfilePage } from './pages/ProfilePage.js';
import { ParentDashboard } from './pages/ParentDashboard.js';
import { AppShell } from './pages/AppShell.js';
import {
	mockAuthEndpoints,
	buildMockAuthData,
	setInstallationId,
} from './auth-helper.js';

type YapperFixtures = {
	loginPage: LoginPage;
	registerPage: RegisterPage;
	forgotPasswordPage: ForgotPasswordPage;
	onboardingPage: OnboardingPage;
	explorePage: ExplorePage;
	dmPage: DmPage;
	channelPage: ChannelPage;
	settingsPage: SettingsPage;
	profilePage: ProfilePage;
	parentDashboard: ParentDashboard;
	appShell: AppShell;
	/** A page that is already authenticated via mocked endpoints. */
	authedPage: ReturnType<typeof base.extend> extends { page: infer P } ? P : never;
};

export const test = base.extend<YapperFixtures>({
	loginPage: async ({ page }, use) => {
		await use(new LoginPage(page));
	},
	registerPage: async ({ page }, use) => {
		await use(new RegisterPage(page));
	},
	forgotPasswordPage: async ({ page }, use) => {
		await use(new ForgotPasswordPage(page));
	},
	onboardingPage: async ({ page }, use) => {
		await use(new OnboardingPage(page));
	},
	explorePage: async ({ page }, use) => {
		await use(new ExplorePage(page));
	},
	dmPage: async ({ page }, use) => {
		await use(new DmPage(page));
	},
	channelPage: async ({ page }, use) => {
		await use(new ChannelPage(page));
	},
	settingsPage: async ({ page }, use) => {
		await use(new SettingsPage(page));
	},
	profilePage: async ({ page }, use) => {
		await use(new ProfilePage(page));
	},
	parentDashboard: async ({ page }, use) => {
		await use(new ParentDashboard(page));
	},
	appShell: async ({ page }, use) => {
		await use(new AppShell(page));
	},

	// authedPage: sets up mocked auth endpoints + installation ID before the
	// test body runs, giving a pre-authenticated page without real API calls.
	authedPage: async ({ page }, use) => {
		const authData = buildMockAuthData();
		await setInstallationId(page, 'fixture-installation-id');
		await mockAuthEndpoints(page, authData);
		// @ts-expect-error — fixture typing trick; the actual type is Page
		await use(page);
	},
});

export { expect };
