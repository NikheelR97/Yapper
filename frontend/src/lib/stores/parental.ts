import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';
import { registerSessionResetter } from '$stores/auth.js';

export interface Child {
	id: string;
	username: string;
	display_name: string;
	avatar_url: string | null;
	date_of_birth: string | null;
	last_seen_at: string | null;
}

export interface ChildSettings {
	approveFriendRequests: boolean;
	approveServerJoins: boolean;
	screenTimeLimitMinutes: number | null;
	contentFilterEnabled: boolean;
	bedtimeStart: string | null; // "HH:MM"
	bedtimeEnd: string | null;
}

export interface PendingAlert {
	id: string;
	childId: string;
	childName: string;
	type: 'friend_request' | 'server_join' | 'dm_request';
	description: string;
	metadata: Record<string, string>;
	createdAt: string;
}

export interface SafetyEvent {
	id: string;
	childId: string;
	type: 'content_warning' | 'new_friend' | 'screen_time' | 'server_join' | 'friend_request';
	description: string;
	createdAt: string;
	read: boolean;
}

export interface ChildActivity {
	totalMinutesToday: number;
	limitMinutes: number | null;
	friendCount: number;
	serverCount: number;
}

interface ParentalState {
	children: Child[];
	selectedChildId: string | null;
	alerts: PendingAlert[];
	feed: SafetyEvent[];
	activity: Record<string, ChildActivity>;
	loading: boolean;
	error: string | null;
}

const initial: ParentalState = {
	children: [],
	selectedChildId: null,
	alerts: [],
	feed: [],
	activity: {},
	loading: false,
	error: null,
};

export const parentalStore = writable<ParentalState>(initial);

registerSessionResetter(() => {
	parentalStore.set({ ...initial });
});

export async function loadChildren() {
	parentalStore.update((s) => ({ ...s, loading: true }));
	try {
		const res = await api.get<{ children: Child[] }>('/api/v2/parental/children');
		const children = res.children ?? [];
		parentalStore.update((s) => ({
			...s,
			children,
			selectedChildId: children.length > 0 ? (s.selectedChildId ?? children[0].id) : null,
			loading: false,
		}));
	} catch (e: unknown) {
		parentalStore.update((s) => ({ ...s, loading: false, error: e instanceof Error ? e.message : 'Failed to load' }));
	}
}

/** Loads pending alerts for all children and merges into a flat list. */
export async function loadAlerts() {
	const { children } = get(parentalStore);
	const alerts: PendingAlert[] = [];
	for (const child of children) {
		try {
			const res = await api.get<{ friend_requests: Record<string, unknown>[]; server_joins: Record<string, unknown>[] }>(
				`/api/v2/parental/children/${child.id}/notifications`
			);
			for (const fr of res.friend_requests ?? []) {
				if (fr.status !== 'pending') continue;
				alerts.push({
					id: fr.id as string,
					childId: child.id,
					childName: child.display_name,
					type: 'friend_request',
					description: `${fr.requester_name} wants to be friends with ${child.display_name}`,
					metadata: { requester_id: fr.requester_id as string },
					createdAt: fr.created_at as string,
				});
			}
			for (const sj of res.server_joins ?? []) {
				if (sj.status !== 'pending') continue;
				alerts.push({
					id: sj.id as string,
					childId: child.id,
					childName: child.display_name,
					type: 'server_join',
					description: `${child.display_name} wants to join ${sj.server_name}`,
					metadata: { server_id: sj.server_id as string },
					createdAt: sj.created_at as string,
				});
			}
		} catch { /* failed to load pending approvals for this child — skip */ }
	}
	parentalStore.update((s) => ({ ...s, alerts }));
}

/** Maps to the overview endpoint — no dedicated feed endpoint exists yet. */
export async function loadFeed(_childId: string) {
	// Feed endpoint not yet implemented on backend; overview data is loaded via loadActivity
}

/** Maps to GET /api/v2/parental/children/:id/overview */
export async function loadActivity(childId: string) {
	try {
		const res = await api.get<{
			pending_friend_requests: number;
			pending_server_joins: number;
			top_servers: Record<string, unknown>[];
		}>(`/api/v2/parental/children/${childId}/overview`);
		const activity: ChildActivity = {
			totalMinutesToday: 0,
			limitMinutes: null,
			friendCount: 0,
			serverCount: res.top_servers?.length ?? 0,
		};
		parentalStore.update((s) => ({
			...s,
			activity: { ...s.activity, [childId]: activity },
		}));
	} catch { /* overview endpoint failed — activity data unavailable */ }
}

export async function approveAlert(alertId: string, type: 'friend_request' | 'server_join') {
	const endpoint = type === 'friend_request'
		? `/api/v2/parental/friend-requests/${alertId}/approve`
		: `/api/v2/parental/server-joins/${alertId}/approve`;
	await api.patch(endpoint);
	parentalStore.update((s) => ({
		...s,
		alerts: s.alerts.filter((a) => a.id !== alertId),
	}));
}

export async function declineAlert(alertId: string, type: 'friend_request' | 'server_join') {
	const endpoint = type === 'friend_request'
		? `/api/v2/parental/friend-requests/${alertId}/decline`
		: `/api/v2/parental/server-joins/${alertId}/decline`;
	await api.patch(endpoint);
	parentalStore.update((s) => ({
		...s,
		alerts: s.alerts.filter((a) => a.id !== alertId),
	}));
}

export async function createChild(data: {
	username: string;
	displayName: string;
	email: string;
	password: string;
	dateOfBirth: string;
	settings: ChildSettings;
}): Promise<Child> {
	const child = await api.post<Child>('/api/v2/parental/children', {
		username: data.username,
		display_name: data.displayName,
		email: data.email,
		password: data.password,
		date_of_birth: data.dateOfBirth,
	});
	parentalStore.update((s) => ({ ...s, children: [...s.children, child] }));
	return child;
}

export function selectChild(childId: string) {
	parentalStore.update((s) => ({ ...s, selectedChildId: childId }));
}
