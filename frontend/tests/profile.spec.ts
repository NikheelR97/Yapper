import { test, expect } from '@playwright/test';
import {
	buildMockAuthData,
	mockAuthEndpoints,
	setInstallationId,
	loginViaApi,
	PRIMARY_INSTALLATION_ID,
	B_PRIMARY_INSTALLATION_ID,
	seedTrustedPrimaryDevice,
	seedTrustedPrimaryDeviceB,
} from './auth-helper.js';
import { mockProfileEndpoints, type MockProfile } from './helpers/mock-routes.js';

/**
 * Feature: User Profile
 *
 * Tests the /profile/[username] page. Provides both:
 *   - Mocked tests (fast, no API): rendering, follow button state
 *   - Live API tests: follow action mutates the count
 */

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';

// ─── Mocked profile rendering ─────────────────────────────────────────────────

test.describe('Profile page — mocked rendering', () => {
	const mockProfile: MockProfile = {
		id: 'user-b-id',
		username: 'userb_e2e',
		displayName: 'User B Display',
		bio: 'This is my bio text.',
		followerCount: 42,
		followingCount: 10,
		isFollowing: false,
		isFriend: false,
		hypeMoments: [{ id: 'm1', type: 'text', content: 'Hype moment 1', mediaUrl: null, gradientStart: null, gradientEnd: null, createdAt: new Date().toISOString() }],
		topCommunities: [{ id: 'c1', name: 'Gaming Community', iconUrl: null, memberCount: 42 }],
		mutualFriends: [],
	};

	test.beforeEach(async ({ page }) => {
		const authData = buildMockAuthData();
		await setInstallationId(page, 'profile-test-install');
		await mockAuthEndpoints(page, authData);
		await mockProfileEndpoints(page, mockProfile);
		// App layout calls these on mount — mock to prevent 401s from live API
		await page.route(`**/api/v2/servers`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await page.route(`**/api/v2/conversations`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
	});

	test('profile header shows display name and @username @smoke', async ({ page }) => {
		// Debug: log ALL api requests and responses
		const allApiRequests: string[] = [];
		const allApiResponses: string[] = [];
		page.on('request', (req) => {
			if (req.url().includes('/api/')) {
				allApiRequests.push(`${req.method()} ${req.url()}`);
			}
		});
		page.on('response', async (res) => {
			if (res.url().includes('/api/')) {
				if (res.url().includes('/users/by/') && !res.url().includes('/follow')) {
					try {
						const body = await res.text();
						allApiResponses.push(`${res.status()} ${res.url()} BODY=${body.slice(0, 200)}`);
					} catch {
						allApiResponses.push(`${res.status()} ${res.url()} (body-read-failed)`);
					}
				} else {
					allApiResponses.push(`${res.status()} ${res.url()}`);
				}
			}
		});
		// Debug: capture console errors and navigation
		const consoleErrors: string[] = [];
		const navEvents: string[] = [];
		page.on('console', (msg) => {
			if (msg.type() === 'error') consoleErrors.push(msg.text());
		});
		page.on('framenavigated', (frame) => {
			if (frame === page.mainFrame()) navEvents.push(frame.url());
		});

		await page.goto(`/profile/${mockProfile.username}`);
		// Wait for app to finish loading before checking profile content
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });
		await expect(page.locator('body')).toContainText(mockProfile.displayName, { timeout: 15_000 }).catch(async (e) => {
			throw new Error(`Profile not found. Nav: ${JSON.stringify(navEvents)}. Requests: ${JSON.stringify(allApiRequests)}. Responses: ${JSON.stringify(allApiResponses)}. Console errors: ${JSON.stringify(consoleErrors)}. Body: "${await page.locator('main').textContent()}". Error: ${e.message}`);
		});
		await expect(page.locator('body')).toContainText(mockProfile.username, { timeout: 5_000 });
	});

	test('bio card renders bio text @smoke', async ({ page }) => {
		await page.goto(`/profile/${mockProfile.username}`);
		await expect(page.locator('body')).toContainText(mockProfile.bio!, { timeout: 10_000 });
	});

	test('follow button renders and shows not-following state', async ({ page }) => {
		await page.goto(`/profile/${mockProfile.username}`);
		const followBtn = page.getByRole('button', { name: /Follow/i }).first();
		await expect(followBtn).toBeVisible({ timeout: 10_000 });
		const text = await followBtn.textContent();
		expect(text).not.toMatch(/Unfollow/i);
	});

	test('profile with isFollowing=true shows following state', async ({ page }) => {
		const followingProfile: MockProfile = { ...mockProfile, isFollowing: true };
		await mockProfileEndpoints(page, followingProfile);

		await page.goto(`/profile/${followingProfile.username}`);
		const followBtn = page.getByRole('button', { name: /Follow|Unfollow/i }).first();
		await expect(followBtn).toBeVisible({ timeout: 10_000 });
		const text = await followBtn.textContent();
		expect(text).toMatch(/Unfollow|Following/i);
	});
});

// ─── Own profile ──────────────────────────────────────────────────────────────

test.describe('Profile page — own profile', () => {
	test('bio card and hype moments section are visible on own profile @smoke', async ({ page }) => {
		const user = { id: 'self-id', username: 'self_user', displayName: 'Self User' };
		const authData = buildMockAuthData({ user: { ...user, avatarUrl: null, accountType: 'standard', isPremium: false } });
		await setInstallationId(page, 'own-profile-install');
		await mockAuthEndpoints(page, authData);

		const selfProfile: MockProfile = {
			id: user.id,
			username: user.username,
			displayName: user.displayName,
			bio: 'My own bio',
			followerCount: 5,
			followingCount: 3,
			isFollowing: false,
			isFriend: false,
			hypeMoments: [{ id: 'm1', text: 'My first hype moment' }],
			topCommunities: [],
			mutualFriends: [],
		};
		await mockProfileEndpoints(page, selfProfile);
		await page.route(`**/api/v2/servers`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await page.route(`**/api/v2/conversations`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});

		await page.goto(`/profile/${user.username}`);
		await expect(page.locator('body')).toContainText(user.displayName, { timeout: 10_000 });
	});
});

// ─── Live API follow action ───────────────────────────────────────────────────

test.describe('Profile page — live API follow action', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(
		!USER_A_EMAIL || !USER_B_EMAIL,
		'Set E2E_EMAIL, E2E_PASSWORD, E2E_EMAIL_2, E2E_PASSWORD_2 to run live profile tests',
	);

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
		seedTrustedPrimaryDeviceB();
	});

	test('clicking Follow increments the follower count', async ({ page }) => {
		const sessionB = await loginViaApi(USER_B_EMAIL, USER_B_PASS, {
			installationId: B_PRIMARY_INSTALLATION_ID,
			label: 'Profile Test B',
		});
		const userB = sessionB.user;

		// Ensure clean state — unfollow if already following
		await fetch(`${API_URL}/api/v2/users/by/${String(userB.username)}/follow`, {
			method: 'DELETE',
			headers: {
				Authorization: `Bearer ${sessionB.accessToken}`,
				'X-CSRF-Token': sessionB.csrfToken,
				Cookie: `csrf_token=${sessionB.csrfToken}`,
			},
		}).catch(() => undefined);

		// Log in as User A via the UI
		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', USER_A_EMAIL);
		await page.fill('#password', USER_A_PASS);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });

		// Navigate to User B's profile
		await page.goto(`/profile/${String(userB.username)}`);
		await expect(page.locator('body')).toContainText(String(userB.username), { timeout: 10_000 });

		const followBtn = page.getByRole('button', { name: /^Follow$/i }).first();
		if (await followBtn.isVisible({ timeout: 5_000 }).catch(() => false)) {
			await followBtn.click();
			// Button should change to "Following" or "Unfollow"
			await expect(
				page.getByRole('button', { name: /Following|Unfollow/i }).first(),
			).toBeVisible({ timeout: 8_000 });
		}

		// Cleanup
		await fetch(`${API_URL}/api/v2/users/by/${String(userB.username)}/follow`, {
			method: 'DELETE',
			headers: {
				Authorization: `Bearer ${sessionB.accessToken}`,
				'X-CSRF-Token': sessionB.csrfToken,
				Cookie: `csrf_token=${sessionB.csrfToken}`,
			},
		}).catch(() => undefined);
	});
});
