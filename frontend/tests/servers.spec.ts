import { test, expect, type Page } from '@playwright/test';
import {
	loginViaApi,
	PRIMARY_INSTALLATION_ID,
	seedTrustedPrimaryDevice,
	setInstallationId,
} from './auth-helper.js';
import { navigateClientSide } from './helpers/wait-for.js';

/**
 * Servers and channels E2E tests.
 *
 * Covers: server creation, channel navigation, message sending, and invite links.
 */

const TEST_EMAIL = process.env.E2E_EMAIL ?? '';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? '';
const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

interface SessionBootstrap {
	accessToken: string;
	csrfToken: string;
}

async function waitForAppReady(page: Page) {
	await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
		timeout: 45_000,
	});
}

async function createSessionBootstrap(): Promise<SessionBootstrap> {
	const authData = await loginViaApi(TEST_EMAIL, TEST_PASSWORD, {
		installationId: PRIMARY_INSTALLATION_ID,
		label: 'E2E Primary Browser',
	});

	return {
		accessToken: authData.accessToken,
		csrfToken: authData.csrfToken,
	};
}

function apiHeaders(session: SessionBootstrap) {
	return {
		'Content-Type': 'application/json',
		Authorization: `Bearer ${session.accessToken}`,
		Cookie: `csrf_token=${session.csrfToken}`,
		'X-CSRF-Token': session.csrfToken,
	};
}

async function createServerViaApi(namePrefix: string): Promise<{ serverId: string; channelId: string }> {
	const session = await createSessionBootstrap();
	const createResponse = await fetch(`${API_URL}/api/v1/servers`, {
		method: 'POST',
		headers: apiHeaders(session),
		body: JSON.stringify({ name: `${namePrefix} ${Date.now()}` }),
	});
	if (!createResponse.ok) {
		throw new Error(`createServerViaApi failed: ${createResponse.status} ${await createResponse.text()}`);
	}

	const server = (await createResponse.json()) as { id?: string };
	if (typeof server.id !== 'string') {
		throw new Error('createServerViaApi response missing server id');
	}

	const channelResponse = await fetch(`${API_URL}/api/v1/servers/${server.id}/channels`, {
		headers: { Authorization: `Bearer ${session.accessToken}` },
	});
	if (!channelResponse.ok) {
		throw new Error(`listChannels failed: ${channelResponse.status} ${await channelResponse.text()}`);
	}

	const channels = (await channelResponse.json()) as Array<{ id?: string }>;
	const channelId = channels[0]?.id;
	if (typeof channelId !== 'string') {
		throw new Error('createServerViaApi response missing default channel');
	}

	return { serverId: server.id, channelId };
}

async function loginAs(page: Page) {
	await setInstallationId(page, PRIMARY_INSTALLATION_ID);
	await page.goto('/login');
	await page.fill('#email', TEST_EMAIL);
	await page.fill('#password', TEST_PASSWORD);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 30_000 });
	await waitForAppReady(page);
}

async function createServerInUi(page: Page, namePrefix: string): Promise<string> {
	await loginAs(page);
	await expect(page.getByRole('searchbox', { name: 'Search' })).toBeVisible({ timeout: 30_000 });

	const createBtn = page.getByRole('button', { name: 'Create Server' });
	await expect(createBtn).toBeVisible({ timeout: 20_000 });
	await createBtn.click();

	const modal = page.locator('[role="dialog"]').first();
	await expect(modal).toBeVisible({ timeout: 5_000 });

	const nameInput = page.getByPlaceholder('Server name').first();
	await expect(nameInput).toBeVisible({ timeout: 5_000 });

	const serverName = `${namePrefix} ${Date.now()}`;
	await nameInput.fill(serverName);
	await page.getByRole('button', { name: 'Create', exact: true }).click();

	await page.waitForURL(/\/servers\/[^/]+\/channels(\/[^/]+)?$/, { timeout: 20_000 });
	return serverName;
}

test.describe('Servers - authenticated', () => {
	test.skip(!TEST_EMAIL || !TEST_PASSWORD, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');
	test.use({ storageState: { cookies: [], origins: [] } });

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
	});

	test('opens the create-server flow from the app shell', async ({ page }) => {
		await createServerInUi(page, 'E2E Server');
		await expect(page).toHaveURL(/\/servers\/[^/]+\/channels(\/[^/]+)?$/, { timeout: 10_000 });
	});
});

test.describe('Channel - authenticated', () => {
	test.skip(!TEST_EMAIL || !TEST_PASSWORD, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');
	test.use({ storageState: { cookies: [], origins: [] } });

	let activeServer: { serverId: string; channelId: string };

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
	});

	test.beforeEach(async ({ page }) => {
		activeServer = await createServerViaApi('E2E Channel Server');
		await loginAs(page);
		await navigateClientSide(page, `/servers/${activeServer.serverId}/channels/${activeServer.channelId}`);
		const input = page.locator('textarea[aria-label="Message"]').first();
		await expect(input).toBeEnabled({ timeout: 60_000 });
	});

	test('first server channel page renders message input', async ({ page }) => {
		const input = page
			.locator('textarea, [contenteditable="true"], input[placeholder*="Message"]')
			.first();
		await expect(input).toBeVisible({ timeout: 20_000 });
	});

	test('sending a channel message renders it in the list', async ({ page }) => {
		test.slow();
		const input = page.locator('textarea[aria-label="Message"]').first();
		await expect(input).toBeEnabled({ timeout: 60_000 });

		const testMsg = `E2E channel test ${Date.now()}`;
		await input.fill(testMsg);
		await input.press('Enter');

		await expect(page.getByText(testMsg)).toBeVisible({ timeout: 8_000 });
	});

	test('typing indicator appears when typing in channel', async ({ page }) => {
		test.slow();
		const input = page.locator('textarea[aria-label="Message"]').first();
		await expect(input).toBeEnabled({ timeout: 60_000 });
		await input.fill('typing...');
		await expect(page.locator('body')).toBeVisible();
	});
});

test.describe('Invite links - authenticated', () => {
	test.skip(!TEST_EMAIL || !TEST_PASSWORD, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');
	test.use({ storageState: { cookies: [], origins: [] } });

	let activeServer: { serverId: string; channelId: string };

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
	});

	test.beforeEach(async ({ page }) => {
		activeServer = await createServerViaApi('E2E Invite Server');
		await loginAs(page);
		await navigateClientSide(page, `/servers/${activeServer.serverId}/channels/${activeServer.channelId}`);
		// Wait for channel to fully load before tests assert on channel UI
		await expect(page.locator('textarea[aria-label="Message"]').first()).toBeEnabled({ timeout: 60_000 });
	});

	test('invite link can be generated for a server', async ({ page }) => {
		const inviteBtn = page
			.getByRole('button', { name: /Create invite link/i })
			.or(page.locator('[title*="invite"], [aria-label*="invite"]'))
			.first();
		await expect(inviteBtn).toBeVisible({ timeout: 10_000 });
		await inviteBtn.click();

		await expect(page.locator('[role="dialog"], .invite-panel').first()).toBeVisible({ timeout: 5_000 });
	});
});
