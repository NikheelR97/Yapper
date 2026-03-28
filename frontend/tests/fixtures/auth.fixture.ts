import { test as base, expect, type Browser, type BrowserContext, type Page } from '@playwright/test';
import { existsSync, readFileSync } from 'fs';

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
	if (!existsSync(storageState)) {
		throw new Error(
			`Missing Playwright auth state: ${storageState}. Run \`npm run test:setup-auth\` first.`,
		);
	}
	if (!existsSync(dataFile)) {
		throw new Error(`Missing Playwright auth data: ${dataFile}. Run \`npm run test:setup-auth\` first.`);
	}
	const data = JSON.parse(readFileSync(dataFile, 'utf-8')) as {
		device?: { installation_id?: string | null };
	};
	const context = await browser.newContext({ storageState });
	const installationId = data.device?.installation_id;
	if (installationId) {
		await context.addInitScript(
			([key, value]) => {
				window.localStorage.setItem(key, value);
			},
			['yapper_installation_id', installationId],
		);
	}
	return {
		page: await context.newPage(),
		context,
	};
}

export const test = base.extend<AuthFixtures>({
	userPage: async ({ browser }, use) => {
		const { context, page } = await createAuthedPage(
			browser,
			'tests/auth-state/user-a.json',
			'tests/auth-state/user-a.data.json',
		);
		try {
			await use(page);
		} finally {
			await context.close();
		}
	},
	userAPage: async ({ browser }, use) => {
		const { context, page } = await createAuthedPage(
			browser,
			'tests/auth-state/user-a.json',
			'tests/auth-state/user-a.data.json',
		);
		try {
			await use(page);
		} finally {
			await context.close();
		}
	},
	userBPage: async ({ browser }, use) => {
		const { context, page } = await createAuthedPage(
			browser,
			'tests/auth-state/user-b.json',
			'tests/auth-state/user-b.data.json',
		);
		try {
			await use(page);
		} finally {
			await context.close();
		}
	},
});

export { expect };
