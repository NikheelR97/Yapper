import { test as base, expect, type Browser, type BrowserContext, type Page } from '@playwright/test';
import { readFileSync } from 'fs';
import { mockAuthEndpoints, type AuthData } from '../auth-helper.js';

type AuthFixtures = {
	userPage: Page;
	userAPage: Page;
	userBPage: Page;
};

async function createAuthedPage(
	browser: Browser,
	storageState: string,
	dataFile: string,
): Promise<{ context: BrowserContext; page: Page }> {
	let data: Partial<AuthData> = {};
	let hasStorageState = false;

	try {
		data = JSON.parse(readFileSync(dataFile, 'utf-8')) as Partial<AuthData>;
	} catch {
		// No data file — proceed with empty defaults
	}

	try {
		await browser.newContext({ storageState }); // probe existence
		hasStorageState = true;
	} catch {
		hasStorageState = false;
	}

	const context = hasStorageState
		? await browser.newContext({ storageState })
		: await browser.newContext();

	const installationId = data.device?.installation_id ?? 'e2e-fixture-install';

	// Set installation ID via addInitScript (context-level so it runs before
	// page JS and avoids SecurityError from evaluating on about:blank).
	await context.addInitScript(
		([key, value]) => {
			window.localStorage.setItem(key, value);
		},
		['yapper_installation_id', installationId],
	);

	const page = await context.newPage();

	if (data.accessToken || data.user || data.device) {
		await mockAuthEndpoints(page, {
			accessToken: data.accessToken ?? 'e2e-access-token',
			csrfToken: data.csrfToken ?? 'e2e-csrf-token',
			user: data.user ?? { id: 'e2e-user', username: 'e2e_user' },
			device: data.device,
		});
	}

	return { page, context };
}

async function useAuthedPage(
	browser: Browser,
	storageState: string,
	dataFile: string,
	use: (page: Page) => Promise<void>,
): Promise<void> {
	const { context, page } = await createAuthedPage(browser, storageState, dataFile);
	try {
		await use(page);
	} finally {
		await context.close();
	}
}

export const test = base.extend<AuthFixtures>({
	userPage: async ({ browser }, use) => {
		await useAuthedPage(browser, 'tests/auth-state/user-a.json', 'tests/auth-state/user-a.data.json', use);
	},
	userAPage: async ({ browser }, use) => {
		await useAuthedPage(browser, 'tests/auth-state/user-a.json', 'tests/auth-state/user-a.data.json', use);
	},
	userBPage: async ({ browser }, use) => {
		await useAuthedPage(browser, 'tests/auth-state/user-b.json', 'tests/auth-state/user-b.data.json', use);
	},
});

export { expect };
