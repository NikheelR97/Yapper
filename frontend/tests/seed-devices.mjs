/**
 * E2E multi-device seeder.
 *
 * Seeds one account with stable device registrations through the real v2 auth/device APIs.
 *
 * Default behavior:
 *   1. Log in/register a stable trusted primary web device
 *   2. Log in/register a stable secondary web device (pending_trust unless already approved)
 *   3. Optionally approve the secondary device
 *
 * Usage:
 *   npx dotenv -e .env.test -- node tests/seed-devices.mjs
 *
 * Required env:
 *   E2E_EMAIL
 *   E2E_PASSWORD
 *
 * Optional env:
 *   E2E_APPROVE_PENDING_DEVICE=1
 *   E2E_RESET_SECONDARY_DEVICE=1
 *   E2E_PRIMARY_INSTALLATION_ID
 *   E2E_SECONDARY_INSTALLATION_ID
 */

const API = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const PRIMARY_INSTALLATION_ID =
	process.env.E2E_PRIMARY_INSTALLATION_ID ?? '11111111-1111-4111-8111-111111111111';
const SECONDARY_INSTALLATION_ID =
	process.env.E2E_SECONDARY_INSTALLATION_ID ?? '22222222-2222-4222-8222-222222222222';

async function assertV2AuthAvailable() {
	const res = await fetch(`${API}/api/v2/auth/login`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: '{}',
	});

	if (res.status === 404) {
		throw new Error(
			`Target API ${API} does not expose POST /api/v2/auth/login. Deploy the v2 auth backend before running multi-device seeding.`,
		);
	}
}

async function loginV2(email, password, installationId, label) {
	const res = await fetch(`${API}/api/v2/auth/login`, {
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
		credentials: 'include',
	});

	if (!res.ok) {
		const text = await res.text();
		throw new Error(`loginV2 failed for ${label}: ${res.status} ${text}`);
	}

	const data = await res.json();
	return {
		accessToken: data.access_token,
		csrfToken: data.csrf_token,
		user: data.user,
		device: data.device,
	};
}

function authedHeaders(session) {
	return {
		'Content-Type': 'application/json',
		Authorization: `Bearer ${session.accessToken}`,
		Cookie: `csrf_token=${session.csrfToken}`,
		'X-CSRF-Token': session.csrfToken,
	};
}

async function listDevices(session) {
	const res = await fetch(`${API}/api/v2/devices`, {
		headers: {
			Authorization: `Bearer ${session.accessToken}`,
		},
	});

	if (!res.ok) {
		const text = await res.text();
		throw new Error(`listDevices failed: ${res.status} ${text}`);
	}

	return res.json();
}

async function approveDevice(session, deviceId) {
	const res = await fetch(`${API}/api/v2/devices/${deviceId}/approve`, {
		method: 'POST',
		headers: authedHeaders(session),
	});

	if (!res.ok) {
		const text = await res.text();
		throw new Error(`approveDevice failed: ${res.status} ${text}`);
	}

	return res.json();
}

async function revokeDevice(session, deviceId) {
	const res = await fetch(`${API}/api/v2/devices/${deviceId}`, {
		method: 'DELETE',
		headers: authedHeaders(session),
	});

	if (!res.ok) {
		const text = await res.text();
		throw new Error(`revokeDevice failed: ${res.status} ${text}`);
	}

	return res.json();
}

function printDevices(devices) {
	for (const device of devices) {
		console.log(
			`  - ${device.label} (${device.id}) signal#${device.signal_device_id} state=${device.trust_state}`,
		);
	}
}

async function main() {
	const email = process.env.E2E_EMAIL;
	const password = process.env.E2E_PASSWORD;
	const approvePending = process.env.E2E_APPROVE_PENDING_DEVICE === '1';
	const resetSecondary = process.env.E2E_RESET_SECONDARY_DEVICE !== '0';

	if (!email || !password) {
		console.error('Missing E2E_EMAIL / E2E_PASSWORD');
		process.exit(1);
	}

	await assertV2AuthAvailable();

	console.log(`Logging in primary device for ${email}...`);
	const primarySession = await loginV2(
		email,
		password,
		PRIMARY_INSTALLATION_ID,
		'E2E Primary Browser',
	);
	console.log(
		`  Primary device: ${primarySession.device.id} state=${primarySession.device.trust_state}`,
	);

	let devices = await listDevices(primarySession);
	const existingSecondary = devices.find(
		(device) =>
			device.installation_id === SECONDARY_INSTALLATION_ID &&
			device.revoked_at == null,
	);
	if (resetSecondary && existingSecondary) {
		console.log(`Recycling existing secondary device ${existingSecondary.id}...`);
		await revokeDevice(primarySession, existingSecondary.id);
		devices = await listDevices(primarySession);
	}

	console.log('Logging in secondary device...');
	const secondarySession = await loginV2(
		email,
		password,
		SECONDARY_INSTALLATION_ID,
		'E2E Secondary Browser',
	);
	console.log(
		`  Secondary device: ${secondarySession.device.id} state=${secondarySession.device.trust_state}`,
	);

	devices = await listDevices(primarySession);
	console.log('\nCurrent devices:');
	printDevices(devices);

	if (approvePending) {
		const pendingDevice = devices.find(
			(device) =>
				device.installation_id === SECONDARY_INSTALLATION_ID &&
				device.trust_state === 'pending_trust',
		);

		if (!pendingDevice) {
			console.log('\nNo pending secondary device found to approve.');
		} else {
			console.log(`\nApproving secondary device ${pendingDevice.id}...`);
			await approveDevice(primarySession, pendingDevice.id);
			devices = await listDevices(primarySession);
			console.log('Updated devices:');
			printDevices(devices);
		}
	}

	console.log('\nDevice seeding complete.');
}

main().catch((error) => {
	console.error(error);
	process.exit(1);
});
