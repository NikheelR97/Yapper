/**
 * Explore store — public server discovery + full-text search.
 *
 * API surface:
 *   loadTrendingTags()    — GET /explore/trending-tags (5-min server cache)
 *   loadLiveServers()     — GET /explore/live-servers (active last 30 min)
 *   loadCommunities()     — GET /explore/communities (ranked by member count)
 *   search(q)             — GET /search?q= (pg_trgm servers + users)
 *   joinServerBySlug(slug)— POST /servers/join/:slug helper
 */

import { writable } from 'svelte/store';
import { api } from '$api/client.js';

// ─── Types ────────────────────────────────────────────────────────────────────

export interface Tag {
	tag: string;
	server_count: number;
}

export interface LiveServer {
	id: string;
	name: string;
	slug: string;
	icon_url: string | null;
	description: string | null;
	tags: string[];
	member_count: number;
	last_active: string;
}

export interface Community {
	id: string;
	name: string;
	slug: string;
	icon_url: string | null;
	description: string | null;
	tags: string[];
	member_count: number;
}

export interface SearchUser {
	id: string;
	username: string;
	display_name: string | null;
	avatar_url: string | null;
}

export interface SearchServer {
	id: string;
	name: string;
	slug: string;
	icon_url: string | null;
	description: string | null;
	tags: string[];
}

export interface ExploreState {
	tags: Tag[];
	liveServers: LiveServer[];
	communities: Community[];
	searchServers: SearchServer[];
	searchUsers: SearchUser[];
	loadingTags: boolean;
	loadingLive: boolean;
	loadingCommunities: boolean;
	searching: boolean;
	searchQuery: string;
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const exploreStore = writable<ExploreState>({
	tags: [],
	liveServers: [],
	communities: [],
	searchServers: [],
	searchUsers: [],
	loadingTags: false,
	loadingLive: false,
	loadingCommunities: false,
	searching: false,
	searchQuery: '',
});

// ─── Loaders ──────────────────────────────────────────────────────────────────

export async function loadTrendingTags(): Promise<void> {
	exploreStore.update((s) => ({ ...s, loadingTags: true }));
	try {
		const res = await api.get<{ tags: Tag[] }>('/api/v1/explore/trending-tags');
		exploreStore.update((s) => ({ ...s, tags: res.tags ?? [], loadingTags: false }));
	} catch {
		exploreStore.update((s) => ({ ...s, loadingTags: false }));
	}
}

export async function loadLiveServers(): Promise<void> {
	exploreStore.update((s) => ({ ...s, loadingLive: true }));
	try {
		const res = await api.get<{ servers: LiveServer[] }>('/api/v1/explore/live-servers');
		exploreStore.update((s) => ({ ...s, liveServers: res.servers ?? [], loadingLive: false }));
	} catch {
		exploreStore.update((s) => ({ ...s, loadingLive: false }));
	}
}

export async function loadCommunities(): Promise<void> {
	exploreStore.update((s) => ({ ...s, loadingCommunities: true }));
	try {
		const res = await api.get<{ communities: Community[] }>('/api/v1/explore/communities');
		exploreStore.update((s) => ({
			...s,
			communities: res.communities ?? [],
			loadingCommunities: false,
		}));
	} catch {
		exploreStore.update((s) => ({ ...s, loadingCommunities: false }));
	}
}

export async function search(q: string): Promise<void> {
	const trimmed = q.trim();
	exploreStore.update((s) => ({ ...s, searching: true, searchQuery: trimmed }));
	if (!trimmed) {
		exploreStore.update((s) => ({
			...s,
			searchServers: [],
			searchUsers: [],
			searching: false,
		}));
		return;
	}
	try {
		const res = await api.get<{ servers: SearchServer[]; users: SearchUser[] }>(
			`/api/v1/search?q=${encodeURIComponent(trimmed)}`
		);
		exploreStore.update((s) => ({
			...s,
			searchServers: res.servers ?? [],
			searchUsers: res.users ?? [],
			searching: false,
		}));
	} catch {
		exploreStore.update((s) => ({ ...s, searching: false }));
	}
}

/** Join a public server by id. */
export async function joinServer(id: string): Promise<void> {
	await api.post<{ status: string }>(`/api/v1/servers/${id}/join`);
}
