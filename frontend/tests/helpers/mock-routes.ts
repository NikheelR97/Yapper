/**
 * API mock recipes for tests that don't need a live backend.
 *
 * Each function mocks a specific feature area's API routes so tests can
 * assert UI behaviour without hitting real servers.
 */

import { type Page } from '@playwright/test';

// ─── Explore ──────────────────────────────────────────────────────────────────

export interface MockServer {
	id: string;
	name: string;
	description?: string;
	memberCount?: number;
	member_count?: number;
	tags?: string[];
	is_live?: boolean;
	icon_url?: string | null;
	iconUrl?: string | null;
}

export interface MockUser {
	id: string;
	username: string;
	displayName?: string;
	avatarUrl?: string | null;
}

export async function mockExploreEndpoints(
	page: Page,
	data: {
		tags?: string[];
		liveServers?: MockServer[];
		communities?: MockServer[];
		searchServers?: MockServer[];
		searchUsers?: MockUser[];
	} = {},
): Promise<void> {
	const tags = data.tags ?? ['gaming', 'music', 'art', 'tech', 'anime'];
	const liveServers = data.liveServers ?? [];
	const communities = data.communities ?? [
		{ id: 'c1', name: 'E2E Community Alpha', memberCount: 42, tags: ['gaming'] },
		{ id: 'c2', name: 'E2E Community Beta', memberCount: 17, tags: ['music'] },
	];
	const searchServers = data.searchServers ?? [];
	const searchUsers = data.searchUsers ?? [];

	// Store expects { tags: Array<{tag: string, server_count: number}> }
	await page.route(`**/api/v1/explore/trending-tags`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				tags: tags.map((t) => (typeof t === 'string' ? { tag: t, server_count: 1 } : t)),
			}),
		});
	});

	// Store expects { servers: LiveServer[] } with snake_case fields
	await page.route(`**/api/v1/explore/live-servers`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				servers: liveServers.map((s) => ({
					...s,
					slug: s.id,
					icon_url: s.icon_url ?? null,
					member_count: (s as unknown as Record<string, unknown>).memberCount ?? s.memberCount ?? 0,
					last_active: new Date().toISOString(),
					is_live: s.is_live ?? true,
				})),
			}),
		});
	});

	// Store expects { communities: Community[] } with snake_case fields
	await page.route(`**/api/v1/explore/communities`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				communities: communities.map((c) => ({
					...c,
					slug: c.id,
					icon_url: (c as unknown as Record<string, unknown>).iconUrl ?? null,
					member_count: (c as unknown as Record<string, unknown>).memberCount ?? c.memberCount ?? 0,
					tags: c.tags ?? [],
				})),
			}),
		});
	});

	// Store uses /api/v1/search?q=... and expects { servers: [...], users: [...] }
	// with users having display_name (snake_case), is_friend, request_sent
	await page.route(`**/api/v1/search**`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				servers: searchServers.map((s) => ({
					...s,
					slug: s.id,
					tags: s.tags ?? [],
				})),
				users: searchUsers.map((u) => ({
					id: u.id,
					username: u.username,
					display_name: u.displayName ?? u.username,
					avatar_url: (u as unknown as Record<string, unknown>).avatarUrl ?? null,
					friend_request_permission: 'everyone',
					is_friend: false,
					request_sent: false,
				})),
			}),
		});
	});
}

// ─── Profile ──────────────────────────────────────────────────────────────────

export interface MockProfile {
	id: string;
	username: string;
	displayName: string;
	bio?: string | null;
	avatarUrl?: string | null;
	bannerUrl?: string | null;
	followerCount?: number;
	followingCount?: number;
	isFollowing?: boolean;
	isFriend?: boolean;
	hypeMoments?: unknown[];
	topCommunities?: unknown[];
	mutualFriends?: unknown[];
}

export async function mockProfileEndpoints(page: Page, profile: MockProfile): Promise<void> {
	const profileBody = JSON.stringify({
		id: profile.id,
		username: profile.username,
		displayName: profile.displayName,
		bio: profile.bio ?? null,
		avatarUrl: profile.avatarUrl ?? null,
		bannerUrl: profile.bannerUrl ?? null,
		bannerColor: null,
		tags: [],
		isPremium: false,
		followerCount: profile.followerCount ?? 0,
		followingCount: profile.followingCount ?? 0,
		hypeMomentCount: (profile.hypeMoments ?? []).length,
		isFollowing: profile.isFollowing ?? false,
		isFriend: profile.isFriend ?? false,
		isSelf: false,
		hypeMoments: profile.hypeMoments ?? [],
		topCommunities: profile.topCommunities ?? [],
		mutualFriends: profile.mutualFriends ?? [],
	});

	// Use a function-based URL matcher to avoid glob/micromatch edge cases.
	await page.route(
		(url) => url.pathname === `/api/v1/users/by/${profile.username}`,
		async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: profileBody,
			});
		},
	);

	// Follow / unfollow
	await page.route(
		(url) => url.pathname === `/api/v1/users/by/${profile.username}/follow`,
		async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
		},
	);

	// Friend request
	await page.route(
		(url) => url.pathname === `/api/v1/users/by/${profile.username}/friend-request`,
		async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
		},
	);
}

// ─── Parental controls ────────────────────────────────────────────────────────

export interface MockChild {
	id: string;
	username: string;
	displayName: string;
	avatarUrl?: string | null;
}

export interface MockAlert {
	id: string;
	type: 'friend_request' | 'server_join';
	childId: string;
	description: string;
	createdAt: string;
}

export async function mockParentalEndpoints(
	page: Page,
	children: MockChild[] = [],
	alerts: MockAlert[] = [],
): Promise<void> {
	await page.route(`**/api/v1/parental/children`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify(children),
		});
	});

	await page.route(`**/api/v1/parental/pending-alerts`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify(alerts),
		});
	});

	await page.route(`**/api/v1/parental/activity**`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ items: [], totalMinutes: 0 }),
		});
	});

	await page.route(`**/api/v1/parental/screen-time**`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ daily: [], limit: null }),
		});
	});

	// Child creation
	await page.route(`**/api/v1/parental/children`, async (route) => {
		if (route.request().method() === 'POST') {
			await route.fulfill({
				status: 201,
				contentType: 'application/json',
				body: JSON.stringify({ id: 'new-child-id', username: 'newchild' }),
			});
		} else {
			await route.continue();
		}
	});
}

// ─── Canvas ───────────────────────────────────────────────────────────────────

export interface MockPoll {
	id: string;
	question: string;
	options: Array<{ index: number; text: string }>;
	vote_counts?: Record<string, number>;
}

export async function mockCanvasEndpoints(
	page: Page,
	data: {
		music?: { title: string; artist: string; album_art_url?: string | null; source_url?: string | null } | null;
		polls?: MockPoll[];
		clips?: unknown[];
	} = {},
): Promise<void> {
	const music = data.music ?? {
		title: 'Test Track',
		artist: 'E2E Artist',
		album_art_url: null,
		source_url: null,
		set_by: null,
		updated_at: null,
	};
	const polls: MockPoll[] = data.polls ?? [
		{
			id: 'poll-1',
			question: 'Best programming language?',
			options: [
				{ index: 0, text: 'Rust' },
				{ index: 1, text: 'TypeScript' },
			],
			vote_counts: { '0': 10, '1': 8 },
		},
	];
	const clips = data.clips ?? [];

	// Canvas state — backend path is /api/v1/canvas/servers/:server_id
	await page.route(`**/api/v1/canvas/servers/*`, async (route) => {
		if (route.request().url().includes('/clips')) {
			// clips sub-path — handled separately below
			await route.continue();
			return;
		}
		const normalizedPolls = polls.map((p) => ({
			id: p.id,
			channel_id: 'mock-channel',
			question: p.question,
			options: p.options,
			vote_counts: p.vote_counts ?? {},
			ends_at: null,
			created_at: new Date().toISOString(),
		}));
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ music, polls: normalizedPolls, clips }),
		});
	});

	// Clips sub-route
	await page.route(`**/api/v1/canvas/servers/*/clips`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ clips }),
		});
	});

	// Poll vote — backend path is /api/v1/canvas/polls/:poll_id/vote
	await page.route(`**/api/v1/canvas/polls/*/vote`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ vote_counts: {} }) });
	});
}

// ─── Support ──────────────────────────────────────────────────────────────────

export async function mockSupportEndpoints(
	page: Page,
	tickets: Array<{ id: string; type: string; priority: string; description: string; createdAt: string }> = [],
): Promise<void> {
	await page.route(`**/api/v1/support/tickets`, async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify(tickets),
			});
		} else {
			// POST — create a new ticket and return it
			const body = (await route.request().postDataJSON()) as {
				type?: string;
				priority?: string;
				description?: string;
			};
			await route.fulfill({
				status: 201,
				contentType: 'application/json',
				body: JSON.stringify({
					id: `ticket-${Date.now()}`,
					type: body.type ?? 'bug',
					priority: body.priority ?? 'medium',
					description: body.description ?? '',
					createdAt: new Date().toISOString(),
				}),
			});
		}
	});
}

// ─── Premium ──────────────────────────────────────────────────────────────────

export async function mockPremiumEndpoints(
	page: Page,
	{ promoValid = true }: { promoValid?: boolean } = {},
): Promise<void> {
	await page.route(`**/api/v1/premium/promo`, async (route) => {
		if (promoValid) {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ activated: true }),
			});
		} else {
			await route.fulfill({
				status: 400,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'Invalid or expired promo code' }),
			});
		}
	});
}

// ─── MediaStream ──────────────────────────────────────────────────────────────

/**
 * Inject a minimal fake MediaStream so recorder components can mount without
 * a real camera/microphone. Uses page.addInitScript so it runs before the app.
 */
export async function mockMediaStream(page: Page): Promise<void> {
	await page.addInitScript(() => {
		const fakeTrack = {
			kind: 'audio',
			id: 'fake-audio-track',
			enabled: true,
			muted: false,
			readyState: 'live',
			stop: () => {},
			addEventListener: () => {},
			removeEventListener: () => {},
			dispatchEvent: () => true,
			getCapabilities: () => ({}),
			getConstraints: () => ({}),
			getSettings: () => ({}),
			applyConstraints: async () => {},
			clone: function () { return this; },
			onended: null,
			onmute: null,
			onunmute: null,
		} as unknown as MediaStreamTrack;

		const fakeStream = {
			id: 'fake-stream',
			active: true,
			getTracks: () => [fakeTrack],
			getAudioTracks: () => [fakeTrack],
			getVideoTracks: () => [],
			addTrack: () => {},
			removeTrack: () => {},
			clone: function () { return this; },
			addEventListener: () => {},
			removeEventListener: () => {},
			dispatchEvent: () => true,
			onaddtrack: null,
			onremovetrack: null,
			onactive: null,
			oninactive: null,
		} as unknown as MediaStream;

		Object.defineProperty(navigator, 'mediaDevices', {
			writable: true,
			value: {
				getUserMedia: async () => fakeStream,
				enumerateDevices: async () => [],
				getSupportedConstraints: () => ({}),
				addEventListener: () => {},
				removeEventListener: () => {},
				dispatchEvent: () => true,
				ondevicechange: null,
			},
		});
	});
}
