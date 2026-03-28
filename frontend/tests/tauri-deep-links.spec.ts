/**
 * Tauri Desktop - Deep Links
 *
 * @desktop @regression
 */

import { test, expect } from './fixtures/auth.fixture';

test.describe('Tauri deep links @desktop @regression', () => {
	test.beforeEach(async ({}, testInfo) => {
		if (!process.env.TAURI_BINARY) {
			testInfo.skip(true, 'TAURI_BINARY not set — skipping Tauri-specific test');
		}
	});

	test.skip('reason: feature not yet implemented', async ({ userPage }) => {
		// TODO YAP-NNN: deep links handler backend/frontend incomplete
		// Unskip when: the Tauri deep-link event handler routes yapper://invite/:code into the join flow
		// Spec file: frontend/tests/tauri-deep-links.spec.ts
		await userPage.goto('/explore');
		await expect(userPage.locator('body')).toBeVisible();
	});
});
