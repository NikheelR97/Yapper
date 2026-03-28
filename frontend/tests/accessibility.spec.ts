/**
 * Accessibility - WCAG 2.1 AA Compliance
 *
 * Scans key routes using axe-core for WCAG 2.1 Level A & AA violations.
 *
 * Rules explicitly excluded (tracked separately, not ignored permanently):
 *   - color-contrast: dark theme requires design-system audit, not test fix
 *
 * Tags: @accessibility - excluded from nightly smoke shards by default.
 * Run manually: npx playwright test --grep "@accessibility"
 *
 * @accessibility
 */

import { test as base, expect, type Page } from '@playwright/test';
import { test as authedTest } from './fixtures/auth.fixture';
import AxeBuilder from '@axe-core/playwright';
import {
	mockExploreEndpoints,
	mockParentalEndpoints,
	mockProfileEndpoints,
	mockSupportEndpoints,
} from './helpers/mock-routes.js';

async function setupShellData(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

async function runAxe(page: Page): Promise<void> {
	const results = await new AxeBuilder({ page })
		.withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
		.exclude('[data-testid="dev-only"]')
		.disableRules(['color-contrast'])
		.analyze();

	if (results.violations.length > 0) {
		const summary = results.violations
			.map((violation) => `[${violation.impact?.toUpperCase()}] ${violation.id}: ${violation.description}`)
			.join('\n');
		expect.soft(results.violations, `WCAG violations:\n${summary}`).toHaveLength(0);
	}
	expect(results.violations).toHaveLength(0);
}

async function waitForReady(page: Page): Promise<void> {
	await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });
}

base.describe('Accessibility - unauthenticated pages @accessibility', () => {
	base('/login page meets WCAG 2.1 AA', async ({ page }) => {
		await page.goto('/login');
		await page.waitForLoadState('networkidle');
		await runAxe(page);
	});

	base('/register page meets WCAG 2.1 AA', async ({ page }) => {
		await page.goto('/register');
		await page.waitForLoadState('networkidle');
		await runAxe(page);
	});

	base('/forgot-password page meets WCAG 2.1 AA', async ({ page }) => {
		await page.goto('/forgot-password');
		await page.waitForLoadState('networkidle');
		await runAxe(page);
	});
});

authedTest.describe('Accessibility - authenticated pages @accessibility', () => {
	authedTest('/explore page meets WCAG 2.1 AA', async ({ userPage }) => {
		await setupShellData(userPage);
		await mockExploreEndpoints(userPage);
		await userPage.goto('/explore');
		await waitForReady(userPage);
		await runAxe(userPage);
	});

	authedTest('/dm page meets WCAG 2.1 AA', async ({ userPage }) => {
		await setupShellData(userPage);
		await mockExploreEndpoints(userPage);
		await userPage.goto('/dm');
		await waitForReady(userPage);
		await runAxe(userPage);
	});

	authedTest('/settings page meets WCAG 2.1 AA', async ({ userPage }) => {
		await setupShellData(userPage);
		await mockExploreEndpoints(userPage);
		await mockSupportEndpoints(userPage);
		await userPage.goto('/settings');
		await waitForReady(userPage);
		await runAxe(userPage);
	});

	authedTest('/settings - each section panel meets WCAG 2.1 AA', async ({ userPage }) => {
		await setupShellData(userPage);
		await mockExploreEndpoints(userPage);
		await mockSupportEndpoints(userPage);
		await userPage.goto('/settings');
		await waitForReady(userPage);

		const navItems = userPage.locator('nav button, nav a, [role="tab"]');
		const count = await navItems.count();

		for (let index = 0; index < count; index++) {
			const item = navItems.nth(index);
			if (await item.isVisible()) {
				await item.click();
				await userPage.waitForLoadState('networkidle');
				await runAxe(userPage);
			}
		}
	});

	authedTest('/parent/children/setup meets WCAG 2.1 AA', async ({ userPage }) => {
		await setupShellData(userPage);
		await mockExploreEndpoints(userPage);
		await mockParentalEndpoints(userPage);
		await userPage.goto('/parent/children/setup');
		await waitForReady(userPage);
		await runAxe(userPage);
	});

	authedTest('/profile/:username page meets WCAG 2.1 AA', async ({ userPage }) => {
		const username = 'a11y_test_user';
		await setupShellData(userPage);
		await mockExploreEndpoints(userPage);
		await mockProfileEndpoints(userPage, {
			id: 'a11y-profile-id',
			username,
			displayName: 'A11y Test User',
			bio: 'Testing accessibility compliance',
		});
		await userPage.goto(`/profile/${username}`);
		await waitForReady(userPage);
		await runAxe(userPage);
	});
});
