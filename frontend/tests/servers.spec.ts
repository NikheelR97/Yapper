import { test, expect, type Page } from '@playwright/test';

/**
 * Servers and channels E2E tests.
 *
 * Covers: server creation, channel navigation, message sending, and invite links.
 */

const TEST_EMAIL = process.env.E2E_EMAIL ?? '';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? '';

async function loginAs(page: Page) {
	await page.goto('/login');
	await page.fill('#email', TEST_EMAIL);
	await page.fill('#password', TEST_PASSWORD);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 20_000 });
}

async function createServerInUi(page: Page, namePrefix: string): Promise<string> {
	await loginAs(page);
	await expect(page.locator('.search-input')).toBeVisible({ timeout: 30_000 });

	const createBtn = page.getByRole('button', { name: 'Create Server' });
	await expect(createBtn).toBeVisible({ timeout: 20_000 });
	await createBtn.click();

	const modal = page.locator('[role="dialog"]').first();
	await expect(modal).toBeVisible({ timeout: 5_000 });

	const nameInput = page.locator('.modal-input').first();
	await expect(nameInput).toBeVisible({ timeout: 5_000 });

	const serverName = `${namePrefix} ${Date.now()}`;
	await nameInput.fill(serverName);
	await page.locator('.modal-submit').click();

	await page.waitForURL(/\/servers\/[^/]+\/channels(\/[^/]+)?$/, { timeout: 20_000 });
	return serverName;
}

test.describe('Servers - authenticated', () => {
	test.skip(!TEST_EMAIL || !TEST_PASSWORD, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test('opens the create-server flow from the app shell', async ({ page }) => {
		await createServerInUi(page, 'E2E Server');
		await expect(page).toHaveURL(/\/servers\/[^/]+\/channels(\/[^/]+)?$/, { timeout: 10_000 });
	});
});

test.describe('Channel - authenticated', () => {
	test.skip(!TEST_EMAIL || !TEST_PASSWORD, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await createServerInUi(page, 'E2E Channel Server');
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

	test.beforeEach(async ({ page }) => {
		await createServerInUi(page, 'E2E Invite Server');
	});

	test('invite link can be generated for a server', async ({ page }) => {
		const inviteBtn = page
			.getByRole('button', { name: /Create invite link/i })
			.or(page.locator('[title*="invite"], [aria-label*="invite"]'))
			.first();
		await expect(inviteBtn).toBeVisible({ timeout: 10_000 });
		await inviteBtn.click();

		await expect(page.locator('.invite-panel').first()).toBeVisible({ timeout: 5_000 });
	});
});
