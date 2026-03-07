import { execFileSync } from 'child_process';
import { expect, test, type Page } from '@playwright/test';

import { mockSignalBootstrapEndpoints, setInstallationId } from './auth-helper.js';

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const TEST_EMAIL = process.env.E2E_EMAIL ?? '';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? '';
const PRIMARY_INSTALLATION_ID =
	process.env.E2E_PRIMARY_INSTALLATION_ID ?? '11111111-1111-4111-8111-111111111111';
const SECONDARY_INSTALLATION_ID =
	process.env.E2E_SECONDARY_INSTALLATION_ID ?? '22222222-2222-4222-8222-222222222222';
let multiDeviceAuthAvailable = false;
let multiDeviceSkipReason =
	'Target API does not expose the v2 multi-device auth endpoints required by this suite.';

async function probeV2MultiDeviceAuth(): Promise<void> {
	try {
		const res = await fetch(`${API_URL}/api/v2/auth/login`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: '{}',
		});

		if (res.status === 404) {
			multiDeviceSkipReason = `${API_URL} is still serving the legacy auth API; skipping multi-device E2E.`;
			return;
		}

		multiDeviceAuthAvailable = true;
	} catch (error) {
		multiDeviceSkipReason = `Could not reach ${API_URL} to probe multi-device auth: ${String(error)}`;
	}
}

async function loginWithSeededInstallation(page: Page, installationId: string) {
	await setInstallationId(page, installationId);
	await mockSignalBootstrapEndpoints(page);
	await page.goto('/login');
	await page.fill('#email', TEST_EMAIL);
	await page.fill('#password', TEST_PASSWORD);
	await page.getByRole('button', { name: /Sign In/i }).click();
}

test.describe('Multi-device auth', () => {
	test.skip(!TEST_EMAIL || !TEST_PASSWORD, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');
	test.use({ storageState: { cookies: [], origins: [] } });
	test.describe.configure({ mode: 'serial' });

	test.beforeAll(async () => {
		await probeV2MultiDeviceAuth();
		if (!multiDeviceAuthAvailable) {
			return;
		}

		execFileSync(process.execPath, ['tests/seed-devices.mjs'], {
			cwd: process.cwd(),
			env: {
				...process.env,
				E2E_APPROVE_PENDING_DEVICE: '0',
				E2E_RESET_SECONDARY_DEVICE: '1',
			},
			stdio: 'pipe',
		});
	});

	test('trusted primary device reaches the app shell and sees the pending secondary device', async ({
		page,
	}) => {
		test.skip(!multiDeviceAuthAvailable, multiDeviceSkipReason);

		await loginWithSeededInstallation(page, PRIMARY_INSTALLATION_ID);

		await expect(page).toHaveURL(/\/explore/, { timeout: 20_000 });
		await expect(page.getByText('Pending Device Approvals')).toBeVisible({ timeout: 20_000 });
		await expect(page.getByText('E2E Secondary Browser')).toBeVisible({ timeout: 20_000 });
	});

	test('secondary device is held at the pending approval gate', async ({ page }) => {
		test.skip(!multiDeviceAuthAvailable, multiDeviceSkipReason);

		await loginWithSeededInstallation(page, SECONDARY_INSTALLATION_ID);

		await expect(page).toHaveURL(/\/explore/, { timeout: 20_000 });
		await expect(page.getByText('Device Approval Required')).toBeVisible({ timeout: 20_000 });
		await expect(page.getByText('Restore from encrypted backup')).toBeVisible({ timeout: 20_000 });
	});
});
