import type { FullConfig } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'fs';
import { resetSeedMarkers, seedTrustedPrimaryDevice, seedTrustedPrimaryDeviceB } from './auth-helper';

interface AuthData {
	accessToken: string;
	csrfToken: string;
	user: Record<string, unknown>;
	device?: Record<string, unknown>;
	refreshToken?: string;
}

interface LoginResult {
	auth: AuthData;
	storageState: {
		cookies: Array<{
			name: string;
			value: string;
			domain: string;
			path: string;
			expires: number;
			httpOnly: boolean;
			secure: boolean;
			sameSite: 'Lax' | 'None' | 'Strict';
		}>;
		origins: [];
	};
}

const DEFAULT_DEVICE_BOOTSTRAP = {
	installation_id:
		process.env.E2E_PRIMARY_INSTALLATION_ID ?? '11111111-1111-4111-8111-111111111111',
	platform: 'web',
	label: 'E2E Primary Browser',
};

function parseSameSite(value: string | undefined): 'Lax' | 'None' | 'Strict' {
	switch ((value ?? '').toLowerCase()) {
		case 'none':
			return 'None';
		case 'strict':
			return 'Strict';
		default:
			return 'Lax';
	}
}

function parseExpires(value: string | undefined): number {
	if (!value) {
		return -1;
	}

	const timestampMs = Date.parse(value);
	return Number.isFinite(timestampMs) ? Math.floor(timestampMs / 1000) : -1;
}

function parseSetCookie(cookieHeader: string, apiUrl: string) {
	const url = new URL(apiUrl);
	const segments = cookieHeader.split(';').map((segment) => segment.trim());
	const [nameValue, ...attributes] = segments;
	const separatorIndex = nameValue.indexOf('=');
	if (separatorIndex <= 0) {
		return null;
	}

	const name = nameValue.slice(0, separatorIndex);
	const value = nameValue.slice(separatorIndex + 1);
	const parsed = new Map<string, string>();
	for (const attribute of attributes) {
		const [rawKey, ...rawValue] = attribute.split('=');
		parsed.set(rawKey.toLowerCase(), rawValue.join('='));
	}

	return {
		name,
		value,
		domain: parsed.get('domain') ?? url.hostname,
		path: parsed.get('path') ?? '/',
		expires: parseExpires(parsed.get('expires')),
		httpOnly: parsed.has('httponly'),
		secure: parsed.has('secure'),
		sameSite: parseSameSite(parsed.get('samesite')),
	};
}

function responseCookies(apiUrl: string, response: Response): LoginResult['storageState']['cookies'] {
	const getSetCookie = (response.headers as Headers & { getSetCookie?: () => string[] }).getSetCookie;
	const headers = typeof getSetCookie === 'function' ? getSetCookie.call(response.headers) : [];
	return headers
		.map((header) => parseSetCookie(header, apiUrl))
		.filter((cookie): cookie is NonNullable<typeof cookie> => cookie !== null);
}

async function loginWithV2(
	apiUrl: string,
	email: string,
	password: string,
	deviceBootstrap: typeof DEFAULT_DEVICE_BOOTSTRAP,
): Promise<LoginResult> {
	let response = await fetch(`${apiUrl}/api/v2/auth/login`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			email,
			password,
			device: deviceBootstrap,
		}),
	});

	if (!response.ok) {
		throw new Error(`global setup login failed: ${response.status} ${await response.text()}`);
	}

	const body = (await response.json()) as {
		access_token?: string;
		csrf_token?: string;
		refresh_token?: string;
		user?: Record<string, unknown>;
		device?: Record<string, unknown>;
	};

	if (!body.access_token || !body.csrf_token || !body.user) {
		throw new Error('global setup login response was missing auth fields');
	}

	return {
		auth: {
			accessToken: body.access_token,
			csrfToken: body.csrf_token,
			refreshToken: body.refresh_token,
			user: body.user,
			device: body.device,
		},
		storageState: {
			cookies: responseCookies(apiUrl, response),
			origins: [],
		},
	};
}

async function fetchUser(apiUrl: string, accessToken: string): Promise<Record<string, unknown> | null> {
	const response = await fetch(`${apiUrl}/api/v2/users/me`, {
		headers: { Authorization: `Bearer ${accessToken}` },
	});

	if (!response.ok) {
		return null;
	}

	return (await response.json()) as Record<string, unknown>;
}

function writeAuthArtifacts(prefix: string, result: LoginResult): void {
	mkdirSync('tests/auth-state', { recursive: true });
	writeFileSync(`tests/auth-state/${prefix}.json`, JSON.stringify(result.storageState, null, 2));
	writeFileSync(`tests/auth-state/${prefix}.data.json`, JSON.stringify(result.auth, null, 2));
}

/**
 * Global setup writes:
 * - tests/auth-state/user-a(.data).json for the primary account
 * - tests/auth-state/user-b(.data).json for the secondary account when configured
 * - legacy tests/auth-state.json + tests/auth-data.json for mocked-auth specs
 *
 * Most E2E specs mock /api/v2/auth/refresh and /users/me using this saved data so
 * they do not burn through the backend login rate limit.
 */
export default async function globalSetup(_config: FullConfig) {
	const apiUrl = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

	const accounts = [
		{
			email: process.env.E2E_EMAIL,
			password: process.env.E2E_PASSWORD,
			prefix: 'user-a',
			writeLegacy: true,
			device: DEFAULT_DEVICE_BOOTSTRAP,
		},
		{
			email: process.env.E2E_EMAIL_2,
			password: process.env.E2E_PASSWORD_2,
			prefix: 'user-b',
			writeLegacy: false,
			device: {
				installation_id:
					process.env.E2E_SECONDARY_INSTALLATION_ID ??
					'22222222-2222-4222-8222-222222222222',
				platform: 'web',
				label: 'E2E Secondary Browser',
			},
		},
	];

	for (const account of accounts) {
		if (!account.email || !account.password) {
			continue;
		}

		const result = await loginWithV2(apiUrl, account.email, account.password, account.device);
		const latestUser = await fetchUser(apiUrl, result.auth.accessToken);
		if (latestUser) {
			result.auth.user = latestUser;
		}

		writeAuthArtifacts(account.prefix, result);
		if (account.writeLegacy) {
			writeFileSync('tests/auth-state.json', JSON.stringify(result.storageState, null, 2));
			writeFileSync('tests/auth-data.json', JSON.stringify(result.auth, null, 2));
		}
	}

	// Device seeding used to happen lazily inside specs. Because each Playwright
	// worker is its own process, that meant every worker re-seeded (a real login
	// each time) — the main source of 429 rate-limit failures and a per-spec `node`
	// spawn. Seed once here instead; workers find the markers and skip.
	resetSeedMarkers();
	if (process.env.E2E_EMAIL && process.env.E2E_PASSWORD) {
		seedTrustedPrimaryDevice();
	}
	seedTrustedPrimaryDeviceB();
}
