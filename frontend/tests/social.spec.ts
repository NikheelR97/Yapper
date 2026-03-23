import { test, expect, type Page } from '@playwright/test';
import {
	loginViaApi,
	setInstallationId,
	seedTrustedPrimaryDevice,
	seedTrustedPrimaryDeviceB,
	PRIMARY_INSTALLATION_ID,
	B_PRIMARY_INSTALLATION_ID,
} from './auth-helper.js';

/**
 * Social graph E2E tests — follow/unfollow, friend requests.
 *
 * Covers the API-level follow graph and friend request flows,
 * with light UI verification for key profile states.
 */

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';

const USER_A_INSTALLATION = PRIMARY_INSTALLATION_ID;
const USER_B_INSTALLATION = B_PRIMARY_INSTALLATION_ID;

interface Session {
	accessToken: string;
	csrfToken: string;
	userId: string;
	username: string;
	_email: string;
	_pass: string;
	_installId: string;
}

async function createSession(
	email: string,
	pass: string,
	installId: string,
	label: string,
): Promise<Session> {
	const d = await loginViaApi(email, pass, { installationId: installId, label });
	return {
		accessToken: d.accessToken,
		csrfToken: d.csrfToken,
		userId: String(d.user.id),
		username: String(d.user.username),
		_email: email,
		_pass: pass,
		_installId: installId,
	};
}

/** Re-authenticate the session. Mutates session in-place. */
async function refreshSession(session: Session): Promise<void> {
	const fresh = await createSession(session._email, session._pass, session._installId, 'refresh');
	Object.assign(session, fresh);
}

/**
 * Execute a fetch with auto-retry on 401 (stale JWT).
 * On 401, re-authenticates the session and retries once.
 */
async function authedFetch(
	session: Session,
	url: string,
	init?: RequestInit,
): Promise<Response> {
	const makeOpts = (): RequestInit => ({
		...init,
		headers: {
			...(init?.headers ?? {}),
			...apiHeaders(session),
		},
	});
	let res = await fetch(url, makeOpts());
	if (res.status === 401) {
		await refreshSession(session);
		res = await fetch(url, makeOpts());
	}
	return res;
}

function apiHeaders(s: Session) {
	return {
		'Content-Type': 'application/json',
		Authorization: `Bearer ${s.accessToken}`,
		Cookie: `csrf_token=${s.csrfToken}`,
		'X-CSRF-Token': s.csrfToken,
	};
}

async function getProfile(
	session: Session,
	username: string,
): Promise<Record<string, unknown>> {
	const res = await authedFetch(session, `${API_URL}/api/v1/users/by/${username}`);
	if (!res.ok) throw new Error(`getProfile failed: ${res.status}`);
	return res.json() as Promise<Record<string, unknown>>;
}

async function follow(from: Session, toUsername: string): Promise<void> {
	const res = await authedFetch(from, `${API_URL}/api/v1/users/by/${toUsername}/follow`, { method: 'POST' });
	if (!res.ok && res.status !== 409) {
		const body = await res.text().catch(() => '');
		throw new Error(`follow failed: ${res.status} ${body}`);
	}
}

async function unfollow(from: Session, toUsername: string): Promise<void> {
	const res = await authedFetch(from, `${API_URL}/api/v1/users/by/${toUsername}/follow`, { method: 'DELETE' });
	if (!res.ok && res.status !== 404) throw new Error(`unfollow failed: ${res.status}`);
}

async function sendFriendRequest(from: Session, toUsername: string): Promise<void> {
	const res = await authedFetch(from, `${API_URL}/api/v1/users/by/${toUsername}/friend-request`, { method: 'POST', body: '{}' });
	if (!res.ok && res.status !== 409) throw new Error(`sendFriendRequest failed: ${res.status}`);
}

async function acceptFriendRequest(acceptor: Session, fromUsername: string): Promise<void> {
	const res = await authedFetch(acceptor, `${API_URL}/api/v1/users/by/${fromUsername}/friend-request`, { method: 'PUT', body: '{}' });
	if (!res.ok) throw new Error(`acceptFriendRequest failed: ${res.status}`);
}

async function rejectFriendRequest(rejector: Session, fromUsername: string): Promise<void> {
	const res = await authedFetch(rejector, `${API_URL}/api/v1/users/by/${fromUsername}/friend-request`, { method: 'DELETE' });
	if (!res.ok) throw new Error(`rejectFriendRequest failed: ${res.status}`);
}

async function listIncomingFriendRequests(session: Session): Promise<unknown[]> {
	const res = await authedFetch(session, `${API_URL}/api/v1/users/me/friend-requests`);
	if (!res.ok) throw new Error(`listFriendRequests failed: ${res.status}`);
	const body = await res.json();
	return Array.isArray(body) ? (body as unknown[]) : ((body as Record<string, unknown>).requests as unknown[] ?? []);
}

// Normalise the profile field names — backend may return camelCase or snake_case
function isFollowing(profile: Record<string, unknown>): boolean {
	return Boolean(profile.isFollowing ?? profile.is_following);
}
function isFriend(profile: Record<string, unknown>): boolean {
	return Boolean(profile.isFriend ?? profile.is_friend);
}
function followerCount(profile: Record<string, unknown>): number {
	return Number(profile.followerCount ?? profile.follower_count ?? 0);
}

// ─── Follow graph ──────────────────────────────────────────────────────────────

test.describe('Social — follow graph', () => {
	test.use({ storageState: { cookies: [], origins: [] } });
	test.describe.configure({ retries: 1 });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL, E2E_PASSWORD, E2E_EMAIL_2, E2E_PASSWORD_2 to run social tests',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('following User B increments their follower count and sets isFollowing=true', async () => {
		const sessionA = await createSession(USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION, 'Social Follow A');
		const sessionB = await createSession(USER_B_EMAIL, USER_B_PASS, USER_B_INSTALLATION, 'Social Follow B');

		// Ensure clean state before test
		await unfollow(sessionA, sessionB.username);

		const before = await getProfile(sessionA, sessionB.username);
		const countBefore = followerCount(before);

		await follow(sessionA, sessionB.username);

		const after = await getProfile(sessionA, sessionB.username);
		expect(followerCount(after)).toBe(countBefore + 1);
		expect(isFollowing(after)).toBe(true);

		// Cleanup
		await unfollow(sessionA, sessionB.username);
	});

	test('unfollowing User B decrements their follower count and sets isFollowing=false', async () => {
		const sessionA = await createSession(USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION, 'Social Unfollow A');
		const sessionB = await createSession(USER_B_EMAIL, USER_B_PASS, USER_B_INSTALLATION, 'Social Unfollow B');

		await follow(sessionA, sessionB.username);

		const following = await getProfile(sessionA, sessionB.username);
		expect(isFollowing(following)).toBe(true);
		const countFollowing = followerCount(following);

		await unfollow(sessionA, sessionB.username);

		const unfollowed = await getProfile(sessionA, sessionB.username);
		expect(isFollowing(unfollowed)).toBe(false);
		expect(followerCount(unfollowed)).toBe(countFollowing - 1);
	});

	test('follow is idempotent — following twice does not double-increment the count', async () => {
		const sessionA = await createSession(USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION, 'Social Idempotent A');
		const sessionB = await createSession(USER_B_EMAIL, USER_B_PASS, USER_B_INSTALLATION, 'Social Idempotent B');

		await unfollow(sessionA, sessionB.username);

		const before = await getProfile(sessionA, sessionB.username);
		const countBefore = followerCount(before);

		await follow(sessionA, sessionB.username);
		await follow(sessionA, sessionB.username); // second follow should be a no-op (409)

		const after = await getProfile(sessionA, sessionB.username);
		expect(followerCount(after)).toBe(countBefore + 1);

		await unfollow(sessionA, sessionB.username);
	});

	test("User B's profile page renders after User A follows them", async ({ page }: { page: Page }) => {
		const sessionA = await createSession(USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION, 'Social UI A');
		const sessionB = await createSession(USER_B_EMAIL, USER_B_PASS, USER_B_INSTALLATION, 'Social UI B');

		await follow(sessionA, sessionB.username);

		await setInstallationId(page, USER_A_INSTALLATION);
		await page.goto('/login');
		await page.fill('#email', USER_A_EMAIL);
		await page.fill('#password', USER_A_PASS);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });

		await page.goto(`/profile/${sessionB.username}`);
		await expect(page.locator('body')).toBeVisible();
		// Profile page should render without errors
		await expect(page.locator('h1, [class*="username"], [data-testid="username"]').first()).toBeVisible({ timeout: 10_000 });

		await unfollow(sessionA, sessionB.username);
	});
});

// ─── Friend requests ───────────────────────────────────────────────────────────

test.describe('Social — friend requests', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL, E2E_PASSWORD, E2E_EMAIL_2, E2E_PASSWORD_2 to run social tests',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('User A sends a friend request and it appears in User B incoming list', async () => {
		const sessionA = await createSession(USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION, 'FR Send A');
		const sessionB = await createSession(USER_B_EMAIL, USER_B_PASS, USER_B_INSTALLATION, 'FR Send B');

		await sendFriendRequest(sessionA, sessionB.username);

		const pending = await listIncomingFriendRequests(sessionB);
		const fromA = (pending as Array<Record<string, unknown>>).find(
			(r) => String(r.fromUserId ?? r.from_user_id ?? (r.from_user as Record<string, unknown>)?.id ?? r.id) === sessionA.userId,
		);
		expect(fromA).toBeDefined();
	});

	test('User B accepting the request establishes a friendship (isFriend=true for both)', async () => {
		const sessionA = await createSession(USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION, 'FR Accept A');
		const sessionB = await createSession(USER_B_EMAIL, USER_B_PASS, USER_B_INSTALLATION, 'FR Accept B');

		// Ensure clean state — B rejects any existing request from A before the test
		await rejectFriendRequest(sessionB, sessionA.username).catch(() => undefined);

		await sendFriendRequest(sessionA, sessionB.username);
		await acceptFriendRequest(sessionB, sessionA.username);

		// Profile visible to B should show A as friend
		const profileA = await getProfile(sessionB, sessionA.username);
		expect(isFriend(profileA)).toBe(true);

		// Profile visible to A should show B as friend
		const profileB = await getProfile(sessionA, sessionB.username);
		expect(isFriend(profileB)).toBe(true);
	});

	test('User B rejecting the request leaves isFriend=false', async () => {
		const sessionA = await createSession(USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION, 'FR Reject A');
		const sessionB = await createSession(USER_B_EMAIL, USER_B_PASS, USER_B_INSTALLATION, 'FR Reject B');

		// Ensure clean state
		await rejectFriendRequest(sessionB, sessionA.username).catch(() => undefined);

		await sendFriendRequest(sessionA, sessionB.username);
		await rejectFriendRequest(sessionB, sessionA.username);

		const profileA = await getProfile(sessionB, sessionA.username);
		expect(isFriend(profileA)).toBe(false);
	});

	test('duplicate friend request is idempotent (409 is handled gracefully)', async () => {
		const sessionA = await createSession(USER_A_EMAIL, USER_A_PASS, USER_A_INSTALLATION, 'FR Dupe A');
		const sessionB = await createSession(USER_B_EMAIL, USER_B_PASS, USER_B_INSTALLATION, 'FR Dupe B');

		// Ensure clean state
		await rejectFriendRequest(sessionB, sessionA.username).catch(() => undefined);

		// Sending the same request twice should not throw (second returns 409, handled gracefully)
		await expect(sendFriendRequest(sessionA, sessionB.username)).resolves.toBeUndefined();
		await expect(sendFriendRequest(sessionA, sessionB.username)).resolves.toBeUndefined();

		// Cleanup
		await rejectFriendRequest(sessionB, sessionA.username).catch(() => undefined);
	});
});
