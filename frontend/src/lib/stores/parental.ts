import { writable } from 'svelte/store';
import { api } from '$api/client.js';

export interface Child {
	id: string;
	username: string;
	displayName: string;
	avatarUrl: string | null;
	age: number | null;
	isOnline: boolean;
	settings: ChildSettings;
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

export async function loadChildren() {
	parentalStore.update((s) => ({ ...s, loading: true }));
	try {
		const children = await api.get<Child[]>('/api/v1/parental/children');
		parentalStore.update((s) => ({
			...s,
			children,
			selectedChildId: children.length > 0 ? (s.selectedChildId ?? children[0].id) : null,
			loading: false,
		}));
	} catch (e: any) {
		parentalStore.update((s) => ({ ...s, loading: false, error: e.message }));
	}
}

export async function loadAlerts() {
	try {
		const alerts = await api.get<PendingAlert[]>('/api/v1/parental/notifications');
		parentalStore.update((s) => ({ ...s, alerts }));
	} catch {}
}

export async function loadFeed(childId: string) {
	try {
		const feed = await api.get<SafetyEvent[]>(`/api/v1/parental/children/${childId}/feed`);
		parentalStore.update((s) => ({ ...s, feed }));
	} catch {}
}

export async function loadActivity(childId: string) {
	try {
		const activity = await api.get<ChildActivity>(`/api/v1/parental/children/${childId}/activity`);
		parentalStore.update((s) => ({
			...s,
			activity: { ...s.activity, [childId]: activity },
		}));
	} catch {}
}

export async function approveAlert(alertId: string) {
	await api.post(`/api/v1/parental/notifications/${alertId}/approve`);
	parentalStore.update((s) => ({
		...s,
		alerts: s.alerts.filter((a) => a.id !== alertId),
	}));
}

export async function declineAlert(alertId: string) {
	await api.post(`/api/v1/parental/notifications/${alertId}/decline`);
	parentalStore.update((s) => ({
		...s,
		alerts: s.alerts.filter((a) => a.id !== alertId),
	}));
}

export async function createChild(data: {
	displayName: string;
	dateOfBirth: string;
	settings: ChildSettings;
}): Promise<Child> {
	const child = await api.post<Child>('/api/v1/parental/children', data);
	parentalStore.update((s) => ({ ...s, children: [...s.children, child] }));
	return child;
}

export function selectChild(childId: string) {
	parentalStore.update((s) => ({ ...s, selectedChildId: childId }));
}
