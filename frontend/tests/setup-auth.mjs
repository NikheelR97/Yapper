import { mkdirSync, writeFileSync } from 'fs';

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const PRIMARY_INSTALLATION_ID =
	process.env.E2E_PRIMARY_INSTALLATION_ID ?? '11111111-1111-4111-8111-111111111111';
const SECONDARY_INSTALLATION_ID =
	process.env.E2E_SECONDARY_INSTALLATION_ID ?? '22222222-2222-4222-8222-222222222222';

function parseSameSite(value) {
	switch ((value ?? '').toLowerCase()) {
		case 'none':
			return 'None';
		case 'strict':
			return 'Strict';
		default:
			return 'Lax';
	}
}

function parseExpires(value) {
	if (!value) {
		return -1;
	}
	const timestampMs = Date.parse(value);
	return Number.isFinite(timestampMs) ? Math.floor(timestampMs / 1000) : -1;
}

function parseSetCookie(cookieHeader, apiUrl) {
	const url = new URL(apiUrl);
	const segments = cookieHeader.split(';').map((segment) => segment.trim());
	const [nameValue, ...attributes] = segments;
	const separatorIndex = nameValue.indexOf('=');
	if (separatorIndex <= 0) {
		return null;
	}

	const name = nameValue.slice(0, separatorIndex);
	const value = nameValue.slice(separatorIndex + 1);
	const parsed = new Map();
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

function responseCookies(apiUrl, response) {
	const getSetCookie = response.headers.getSetCookie?.bind(response.headers);
	const headers = typeof getSetCookie === 'function' ? getSetCookie() : [];
	return headers.map((header) => parseSetCookie(header, apiUrl)).filter(Boolean);
}

async function login(email, password, installationId, label) {
	const response = await fetch(`${API_URL}/api/v2/auth/login`, {
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

	if (!response.ok) {
		throw new Error(`auth setup failed for ${email}: ${response.status} ${await response.text()}`);
	}

	const body = await response.json();
	if (!body.access_token || !body.csrf_token || !body.user) {
		throw new Error(`auth setup response missing fields for ${email}`);
	}

	return {
		storageState: {
			cookies: responseCookies(API_URL, response),
			origins: [],
		},
		auth: {
			accessToken: body.access_token,
			csrfToken: body.csrf_token,
			refreshToken: body.refresh_token,
			user: body.user,
			device: body.device,
		},
	};
}

async function writeArtifacts(prefix, email, password, installationId, label, writeLegacy = false) {
	if (!email || !password) {
		return;
	}

	const result = await login(email, password, installationId, label);
	writeFileSync(`tests/auth-state/${prefix}.json`, JSON.stringify(result.storageState, null, 2));
	writeFileSync(`tests/auth-state/${prefix}.data.json`, JSON.stringify(result.auth, null, 2));

	if (writeLegacy) {
		writeFileSync('tests/auth-state.json', JSON.stringify(result.storageState, null, 2));
		writeFileSync('tests/auth-data.json', JSON.stringify(result.auth, null, 2));
	}
}

async function main() {
	mkdirSync('tests/auth-state', { recursive: true });
	await writeArtifacts(
		'user-a',
		process.env.E2E_EMAIL,
		process.env.E2E_PASSWORD,
		PRIMARY_INSTALLATION_ID,
		'Playwright User A',
		true,
	);
	await writeArtifacts(
		'user-b',
		process.env.E2E_EMAIL_2,
		process.env.E2E_PASSWORD_2,
		SECONDARY_INSTALLATION_ID,
		'Playwright User B',
	);
}

main().catch((error) => {
	console.error(error);
	process.exit(1);
});
