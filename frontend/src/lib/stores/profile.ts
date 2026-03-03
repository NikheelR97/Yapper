import { writable } from 'svelte/store';
import { api } from '$api/client.js';

export interface UserProfile {
	id: string;
	username: string;
	displayName: string;
	avatarUrl: string | null;
	bannerUrl: string | null;
	bannerColor: string | null;
	bio: string | null;
	tags: string[];
	isPremium: boolean;
	followerCount: number;
	followingCount: number;
	hypeMomentCount: number;
	isFollowing: boolean;
	isFriend: boolean;
	isSelf: boolean;
	topCommunities: Community[];
	mutualFriends: MutualFriend[];
	hypeMoments: HypeMoment[];
}

export interface Community {
	id: string;
	name: string;
	iconUrl: string | null;
	memberCount: number;
}

export interface MutualFriend {
	id: string;
	username: string;
	displayName: string;
	avatarUrl: string | null;
	mutualCount: number;
}

export interface HypeMoment {
	id: string;
	type: 'media' | 'text' | 'clip';
	content: string;
	mediaUrl: string | null;
	gradientStart: string | null;
	gradientEnd: string | null;
	createdAt: string;
}

interface ProfileState {
	profile: UserProfile | null;
	loading: boolean;
	error: string | null;
}

const initial: ProfileState = {
	profile: null,
	loading: false,
	error: null,
};

export const profileStore = writable<ProfileState>(initial);

export async function loadProfile(username: string) {
	profileStore.update((s) => ({ ...s, loading: true, error: null }));
	try {
		const profile = await api.get<UserProfile>(`/api/v1/users/by/${username}`);
		profileStore.update((s) => ({ ...s, profile, loading: false }));
	} catch (e: any) {
		profileStore.update((s) => ({
			...s,
			loading: false,
			error: e.message ?? 'Failed to load profile',
		}));
	}
}

export async function followUser(userId: string) {
	await api.post(`/api/v1/users/${userId}/follow`);
	profileStore.update((s) => {
		if (!s.profile) return s;
		return {
			...s,
			profile: {
				...s.profile,
				isFollowing: true,
				followerCount: s.profile.followerCount + 1,
			},
		};
	});
}

export async function unfollowUser(userId: string) {
	await api.delete(`/api/v1/users/${userId}/follow`);
	profileStore.update((s) => {
		if (!s.profile) return s;
		return {
			...s,
			profile: {
				...s.profile,
				isFollowing: false,
				followerCount: Math.max(0, s.profile.followerCount - 1),
			},
		};
	});
}

export async function sendFriendRequest(userId: string) {
	await api.post(`/api/v1/users/${userId}/friend-request`);
}
