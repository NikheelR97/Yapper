/**
 * E2E test data seeder.
 *
 * Seeds:
 *   1. A DM conversation between USER_A and USER_B (closes test 17 skip)
 *   2. A public server owned by USER_B that USER_A is NOT a member of (closes test 24 skip)
 *
 * Usage:
 *   npx dotenv -e .env.test -- node tests/seed.mjs
 */

const API = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

async function login(email, password) {
	const res = await fetch(`${API}/api/v1/auth/login`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ email, password }),
		credentials: 'include',
	});
	if (!res.ok) {
		const text = await res.text();
		throw new Error(`Login failed for ${email}: ${res.status} ${text}`);
	}
	const data = await res.json();
	return {
		userId: data.user.id,
		username: data.user.username,
		accessToken: data.access_token,
		csrfToken: data.csrf_token,
	};
}

function headers(session) {
	return {
		'Content-Type': 'application/json',
		'Authorization': `Bearer ${session.accessToken}`,
		// CSRF double-submit: cookie value must equal header value
		'Cookie': `csrf_token=${session.csrfToken}`,
		'X-CSRF-Token': session.csrfToken,
	};
}

async function createDmConversation(session, peerId) {
	const res = await fetch(`${API}/api/v1/conversations`, {
		method: 'POST',
		headers: headers(session),
		body: JSON.stringify({ peer_id: peerId }),
	});
	if (!res.ok) {
		const text = await res.text();
		throw new Error(`createDmConversation failed: ${res.status} ${text}`);
	}
	return res.json();
}

async function listMyServers(session) {
	const res = await fetch(`${API}/api/v1/servers`, {
		headers: { 'Authorization': `Bearer ${session.accessToken}` },
	});
	if (!res.ok) throw new Error(`listMyServers failed: ${res.status}`);
	return res.json();
}

async function createServer(session, name) {
	const res = await fetch(`${API}/api/v1/servers`, {
		method: 'POST',
		headers: headers(session),
		body: JSON.stringify({ name }),
	});
	if (!res.ok) {
		const text = await res.text();
		throw new Error(`createServer failed: ${res.status} ${text}`);
	}
	return res.json();
}

async function main() {
	const emailA = process.env.E2E_EMAIL;
	const passA  = process.env.E2E_PASSWORD;
	const emailB = process.env.E2E_EMAIL_2;
	const passB  = process.env.E2E_PASSWORD_2;

	if (!emailA || !passA || !emailB || !passB) {
		console.error('Missing E2E_EMAIL / E2E_PASSWORD / E2E_EMAIL_2 / E2E_PASSWORD_2');
		process.exit(1);
	}

	console.log('Logging in as USER_A:', emailA);
	const sessionA = await login(emailA, passA);
	console.log(`  USER_A id: ${sessionA.userId} (@${sessionA.username})`);

	console.log('Logging in as USER_B:', emailB);
	const sessionB = await login(emailB, passB);
	console.log(`  USER_B id: ${sessionB.userId} (@${sessionB.username})`);

	// ── Seed 1: DM conversation ────────────────────────────────────────────────
	console.log('\nCreating DM conversation between USER_A and USER_B...');
	const convo = await createDmConversation(sessionA, sessionB.userId);
	console.log(`  Conversation id: ${convo.id} (already existed or freshly created)`);

	// ── Seed 2: Joinable public server ─────────────────────────────────────────
	// Check USER_A's current servers — we need a server USER_A is NOT in.
	// Create one owned by USER_B with a stable name so re-runs are idempotent-ish.
	const serversB = await listMyServers(sessionB);
	const seedName = 'E2E Joinable Server';
	const existing = Array.isArray(serversB)
		? serversB.find((s) => s.name === seedName)
		: null;

	let server;
	if (existing) {
		server = existing;
		console.log(`\nJoinable server already exists: "${server.name}" (id: ${server.id})`);
	} else {
		console.log(`\nCreating joinable server "${seedName}" as USER_B...`);
		server = await createServer(sessionB, seedName);
		console.log(`  Server id: ${server.id}`);
	}

	// Confirm USER_A is NOT in that server
	const serversA = await listMyServers(sessionA);
	const alreadyMember = Array.isArray(serversA) && serversA.some((s) => s.id === server.id);
	if (alreadyMember) {
		console.log(`  USER_A is already a member of "${seedName}" — test 24 may still skip.`);
		console.log('  Tip: create a new server manually or use a third account.');
	} else {
		console.log(`  USER_A is NOT a member of "${seedName}" — test 24 should pass.`);
	}

	console.log('\nSeeding complete.');
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
