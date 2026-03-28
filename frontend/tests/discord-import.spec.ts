import { test } from '@playwright/test';

/**
 * Feature: Discord Import - Avatar + Bot Message Display
 *
 * Backend/frontend support is not complete yet, so these tests are quarantined
 * instead of retried in CI.
 */

test.describe('Discord Import - settings page @smoke', () => {
	test.skip('reason: feature not yet implemented (connected state)', async ({ page }) => {
		// TODO YAP-NNN: Discord import backend/frontend incomplete
		// Unskip when: Discord account linking and import APIs are implemented end-to-end
		// Spec file: frontend/tests/discord-import.spec.ts
	});

	test.skip('reason: feature not yet implemented (disconnected state)', async ({ page }) => {
		// TODO YAP-NNN: Discord import backend/frontend incomplete
		// Unskip when: Discord connection absence is surfaced by a completed settings flow
		// Spec file: frontend/tests/discord-import.spec.ts
	});
});

test.describe('Bot message display in channel @smoke', () => {
	test.skip('reason: feature not yet implemented (bot message import)', async ({ page }) => {
		// TODO YAP-NNN: Discord import backend/frontend incomplete
		// Unskip when: Discord import-backed bot message ingestion is implemented end-to-end
		// Spec file: frontend/tests/discord-import.spec.ts
	});
});
