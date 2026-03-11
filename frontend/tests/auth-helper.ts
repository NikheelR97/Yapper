import { execFileSync } from 'child_process';
import { type Page } from '@playwright/test';
import { existsSync, readFileSync } from 'fs';

export interface AuthData {
	accessToken: string;
	csrfToken: string;
	user: Record<string, unknown>;
	device?: ServerDevice;
}

export interface ServerDevice {
	id: string;
	signal_device_id: number;
	installation_id: string | null;
	platform: 'web' | 'tauri' | 'capacitor';
	label: string;
	trust_state: 'trusted' | 'pending_trust' | 'revoked';
	created_at: string;
	last_seen_at: string | null;
	approved_at: string | null;
	revoked_at: string | null;
}

interface MockAuthRouteOptions {
	devices?: ServerDevice[];
	syncEvents?: unknown[];
}

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const INSTALLATION_ID_KEY = 'yapper_installation_id';
export const PRIMARY_INSTALLATION_ID =
	process.env.E2E_PRIMARY_INSTALLATION_ID ?? '11111111-1111-4111-8111-111111111111';
export const SECONDARY_INSTALLATION_ID =
	process.env.E2E_SECONDARY_INSTALLATION_ID ?? '22222222-2222-4222-8222-222222222222';

const DEFAULT_USER = {
	id: 'e2e-user',
	username: 'e2e_user',
	displayName: 'E2E User',
	avatarUrl: null,
	accountType: 'standard',
	isPremium: false,
};

export function buildMockDevice(overrides: Partial<ServerDevice> = {}): ServerDevice {
	const now = new Date().toISOString();
	return {
		id: 'e2e-device',
		signal_device_id: 1,
		installation_id: 'e2e-installation',
		platform: 'web',
		label: 'E2E Browser',
		trust_state: 'trusted',
		created_at: now,
		last_seen_at: null,
		approved_at: now,
		revoked_at: null,
		...overrides,
	};
}

export function buildMockAuthData(
	overrides: {
		accessToken?: string;
		csrfToken?: string;
		user?: Record<string, unknown>;
		device?: Partial<ServerDevice> | ServerDevice;
	} = {},
): AuthData {
	const base = loadAuthData() ?? {
		accessToken: 'e2e-access-token',
		csrfToken: 'e2e-csrf-token',
		user: DEFAULT_USER,
		device: buildMockDevice(),
	};
	const baseDevice = base.device ?? buildMockDevice();

	return {
		accessToken: overrides.accessToken ?? base.accessToken,
		csrfToken: overrides.csrfToken ?? base.csrfToken,
		user: { ...base.user, ...(overrides.user ?? {}) },
		device: {
			...baseDevice,
			...(overrides.device ?? {}),
		},
	};
}

/**
 * Load saved auth data from global setup.
 * Returns null if auth-data.json doesn't exist yet (first run / no E2E_EMAIL set).
 */
export function loadAuthData(): AuthData | null {
	const path = 'tests/auth-data.json';
	if (!existsSync(path)) return null;
	try {
		return JSON.parse(readFileSync(path, 'utf-8')) as AuthData;
	} catch {
		return null;
	}
}

/**
 * Log in directly against the API, preferring v2 device-aware auth and
 * falling back to the legacy v1 login endpoint if the target backend has not
 * deployed v2 yet.
 */
export async function loginViaApi(
	email: string,
	password: string,
	options?: {
		installationId?: string;
		label?: string;
	},
): Promise<AuthData> {
	const installationId = options?.installationId ?? 'e2e-installation';
	const label = options?.label ?? 'E2E Browser';
	let response = await fetch(`${API_URL}/api/v2/auth/login`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			email,
			password,
			device: {
				installation_id: installationId,
				platform: 'web',
				label,
			},
		}),
	});

	if (response.status === 404) {
		response = await fetch(`${API_URL}/api/v1/auth/login`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ email, password }),
		});
	}

	if (!response.ok) {
		throw new Error(`loginViaApi failed: ${response.status} ${await response.text()}`);
	}

	const body = (await response.json()) as {
		access_token?: string;
		csrf_token?: string;
		user?: Record<string, unknown>;
		device?: ServerDevice;
	};

	if (!body.access_token || !body.csrf_token || !body.user) {
		throw new Error('loginViaApi response was missing auth fields');
	}

	return {
		accessToken: body.access_token,
		csrfToken: body.csrf_token,
		user: body.user,
		device: body.device ?? buildMockDevice({ installation_id: installationId, label }),
	};
}

/**
 * Mock the auth refresh and users/me endpoints so each test doesn't burn a
 * rate-limit token. The app's (app)/+layout calls these on every page load to
 * restore session; without mocking, repeated tests from the same IP can hit
 * auth rate limits and redirect to /login.
 */
export async function mockAuthEndpoints(
	page: Page,
	authData?: AuthData | null,
	options: MockAuthRouteOptions = {},
): Promise<void> {
	const data = authData ?? loadAuthData();
	if (!data) return;

	const { accessToken, csrfToken, user, device } = data;
	const deviceSummary = device ?? buildMockDevice();
	const devices = options.devices ?? [deviceSummary];
	const syncEvents = options.syncEvents ?? [];

	await page.route(`${API_URL}/api/v2/auth/refresh`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				access_token: accessToken,
				csrf_token: csrfToken,
				user,
				device: deviceSummary,
			}),
		});
	});

	await page.route(`${API_URL}/api/v1/users/me`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify(user),
		});
	});

	await page.route(`${API_URL}/api/v2/devices`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify(devices),
		});
	});

	await page.route(`${API_URL}/api/v2/devices/sync-events`, async (route) => {
		if (route.request().method() !== 'GET') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: '{}',
			});
			return;
		}

		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify(syncEvents),
		});
	});

	await page.route(`${API_URL}/api/v2/devices/trust-requests`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: '{}',
		});
	});

	await mockSignalBootstrapEndpoints(page);
}

/**
 * Mock the expensive Signal bootstrap endpoints so auth/UI tests can focus on
 * app routing and device state rather than key-upload latency.
 */
export async function mockSignalBootstrapEndpoints(page: Page): Promise<void> {
	for (const path of [
		`${API_URL}/api/v2/keys/identity`,
		`${API_URL}/api/v2/keys/signed-prekey`,
		`${API_URL}/api/v2/keys/one-time-prekeys`,
		`${API_URL}/api/v2/keys/one-time-prekey-count`,
	]) {
		await page.route(path, async (route) => {
			if (route.request().method() === 'GET') {
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: '{"count":100,"low":false}',
				});
			} else {
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: '{}',
				});
			}
		});
	}
}

export async function setInstallationId(page: Page, installationId: string): Promise<void> {
	await page.addInitScript(
		([key, value]) => {
			window.localStorage.setItem(key, value);
		},
		[INSTALLATION_ID_KEY, installationId],
	);
}

export function seedTrustedPrimaryDevice(): void {
	try {
		execFileSync(process.execPath, ['tests/seed-devices.mjs'], {
			cwd: process.cwd(),
			env: {
				...process.env,
				E2E_APPROVE_PENDING_DEVICE: '0',
				E2E_RESET_SECONDARY_DEVICE: '1',
				E2E_PRIMARY_INSTALLATION_ID: PRIMARY_INSTALLATION_ID,
				E2E_SECONDARY_INSTALLATION_ID: SECONDARY_INSTALLATION_ID,
			},
			stdio: 'pipe',
		});
	} catch (error) {
		const details =
			error && typeof error === 'object' && 'stderr' in error
				? String((error as { stderr?: Buffer | string }).stderr ?? '')
				: String(error);
		throw new Error(`seedTrustedPrimaryDevice failed: ${details}`.trim());
	}
}
