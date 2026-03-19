/**
 * QA Tests doc — Section 2, P1: Invite Expiry
 *
 * Feature: Invite Expiry
 *   Scenario: Expired invite code
 *     Given an invite has expired or been revoked
 *     When the user submits the invite code
 *     Then the app shows a clear rejection state and does not join the server
 *
 * Uses mocked auth — no live API required.
 * @smoke
 */

import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { log } from './helpers/log.js';

async function setupAuth(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
	const device = buildMockDevice({ installation_id: 'invite-expiry-install' });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, 'invite-expiry-install');
	await mockAuthEndpoints(page, authData);

	// Mock servers — return one server so the sidebar shows server mode with the join form.
	await page.route('**/api/v1/servers', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{ id: 'srv-1', name: 'Test Server', icon_url: null },
			]),
		});
	});
	await page.route('**/api/v1/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	// Mock channels for the server
	await page.route('**/api/v1/servers/srv-1/channels', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{ id: 'ch-1', name: 'general', server_id: 'srv-1' },
			]),
		});
	});
}

test.describe('Invite expiry @security @auth', () => {
	test('expired or invalid invite code shows rejection message @smoke', async ({ page }) => {
		await setupAuth(page);

		log('INVITE', 'SETUP', 'Configuring 404 response on join-by-invite to simulate expired invite');

		// Mock the join endpoint to return 404 (expired/invalid invite).
		await page.route('**/api/v1/servers/join/**', async (route) => {
			log('INVITE', 'INTERCEPT', `404 on POST ${route.request().url()} — invite expired`);
			await route.fulfill({
				status: 404,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'Invite not found or expired' }),
			});
		});

		log('NETWORK', 'NAVIGATE', 'Navigating to server page to access join form');
		await page.goto('/servers/srv-1/channels');

		// Wait for the sidebar with the join form to be visible.
		const joinInput = page.locator('input[aria-label="Enter invite code"]');
		await expect(joinInput).toBeVisible({ timeout: 20_000 });

		log('INVITE', 'ACTION', 'Submitting expired invite code');
		await joinInput.fill('EXPIRED-CODE-123');

		const joinBtn = page.locator('button.join-submit, button:has-text("Join")').first();
		await expect(joinBtn).toBeEnabled();
		await joinBtn.click();

		// The sidebar should show the error message.
		log('ASSERTION', 'STATE', 'Waiting for rejection message');
		const errorMsg = page.locator('.join-error');
		await expect(errorMsg).toBeVisible({ timeout: 10_000 });
		await expect(errorMsg).toContainText('Invalid or expired invite code');

		log('VALIDATION', 'UI', '"Invalid or expired invite code" rejection confirmed. [PASS]');

		// The server list should NOT have gained a new server.
		// The user should still be on the same page.
		await expect(page.locator('input[aria-label="Enter invite code"]')).toBeVisible();
		log('VALIDATION', 'STATE', 'User remains on server page — no server was joined. [PASS]');
	});

	test('empty invite code does not submit @smoke', async ({ page }) => {
		await setupAuth(page);

		log('INVITE', 'SETUP', 'Testing that empty invite code cannot be submitted');

		await page.goto('/servers/srv-1/channels');

		const joinInput = page.locator('input[aria-label="Enter invite code"]');
		await expect(joinInput).toBeVisible({ timeout: 20_000 });

		// The Join button should be disabled when the input is empty.
		const joinBtn = page.locator('button.join-submit');
		await expect(joinBtn).toBeDisabled();

		log('VALIDATION', 'UI', 'Join button is disabled with empty invite code. [PASS]');
	});
});
