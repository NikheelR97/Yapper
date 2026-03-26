import type { FullConfig } from '@playwright/test';
import { writeFileSync } from 'fs';

interface AuthData {
	accessToken: string;
	csrfToken: string;
	user: Record<string, unknown>;
	device?: Record<string, unknown>;
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

async function loginWithV2(apiUrl: string, email: string, password: string): Promise<LoginResult> {
	let response = await fetch(`${apiUrl}/api/v2/auth/login`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			email,
			password,
			device: DEFAULT_DEVICE_BOOTSTRAP,
		}),
	});

	if (!response.ok) {
		throw new Error(`global setup login failed: ${response.status} ${await response.text()}`);
	}

	const body = (await response.json()) as {
		access_token?: string;
		csrf_token?: string;
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

/**
 * Global setup writes:
 * - tests/auth-state.json with API cookies for refresh flows
 * - tests/auth-data.json with access token, CSRF token, user, and optional device
 *
 * Most E2E specs mock /api/v2/auth/refresh and /users/me using this saved data so
 * they do not burn through the backend login rate limit.
 */
export default async function globalSetup(_config: FullConfig) {
	const email = process.env.E2E_EMAIL;
	const password = process.env.E2E_PASSWORD;

	if (!email || !password) {
		return;
	}

	const apiUrl = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
	const result = await loginWithV2(apiUrl, email, password);
	const latestUser = await fetchUser(apiUrl, result.auth.accessToken);
	if (latestUser) {
		result.auth.user = latestUser;
	}

	writeFileSync('tests/auth-state.json', JSON.stringify(result.storageState, null, 2));
	writeFileSync('tests/auth-data.json', JSON.stringify(result.auth, null, 2));
}
