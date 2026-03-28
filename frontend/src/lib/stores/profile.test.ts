import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$api/client.js', () => ({
	api: {
		get: vi.fn(),
		post: vi.fn(),
		delete: vi.fn(),
	},
}));

import { api } from '$api/client.js';

import {
	followUser,
	loadProfile,
	profileStore,
	sendFriendRequest,
	unfollowUser,
} from './profile.js';

const testProfile = {
	id: 'user-1',
	username: 'tester',
	displayName: 'Test User',
	avatarUrl: null,
	bannerUrl: null,
	bannerColor: null,
	bio: 'hello',
	tags: ['friend'],
	isPremium: false,
	followerCount: 4,
	followingCount: 2,
	hypeMomentCount: 1,
	isFollowing: false,
	isFriend: false,
	isSelf: false,
	topCommunities: [],
	mutualFriends: [],
	hypeMoments: [],
};

describe('profile store', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		profileStore.set({
			profile: null,
			loading: false,
			error: null,
		});
	});

	it('loads a profile and clears loading state on success', async () => {
		vi.mocked(api.get).mockResolvedValue(testProfile);

		await loadProfile('tester');

		expect(api.get).toHaveBeenCalledWith('/api/v2/users/by/tester');
		expect(get(profileStore)).toEqual({
			profile: testProfile,
			loading: false,
			error: null,
		});
	});

	it('stores a readable error when loading fails', async () => {
		vi.mocked(api.get).mockRejectedValue(new Error('No profile'));

		await loadProfile('missing-user');

		expect(get(profileStore)).toEqual({
			profile: null,
			loading: false,
			error: 'No profile',
		});
	});

	it('updates follow state optimistically after following a user', async () => {
		profileStore.set({
			profile: testProfile,
			loading: false,
			error: null,
		});

		await followUser('tester');

		expect(api.post).toHaveBeenCalledWith('/api/v2/users/by/tester/follow');
		expect(get(profileStore).profile).toMatchObject({
			isFollowing: true,
			followerCount: 5,
		});
	});

	it('updates unfollow state and never lets follower counts go negative', async () => {
		profileStore.set({
			profile: { ...testProfile, isFollowing: true, followerCount: 0 },
			loading: false,
			error: null,
		});

		await unfollowUser('tester');

		expect(api.delete).toHaveBeenCalledWith('/api/v2/users/by/tester/follow');
		expect(get(profileStore).profile).toMatchObject({
			isFollowing: false,
			followerCount: 0,
		});
	});

	it('leaves state untouched when follow helpers run without a loaded profile', async () => {
		await followUser('tester');
		await unfollowUser('tester');

		expect(get(profileStore)).toEqual({
			profile: null,
			loading: false,
			error: null,
		});
	});

	it('sends friend requests through the expected endpoint', async () => {
		await sendFriendRequest('tester');

		expect(api.post).toHaveBeenCalledWith('/api/v2/users/by/tester/friend-request');
	});
});
