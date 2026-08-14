<script lang="ts">
	import { goto } from '$app/navigation';
	import { authStore } from '$stores/auth.js';
	import { followUser, unfollowUser, sendFriendRequest, type UserProfile } from '$stores/profile.js';
	import { toast } from '$stores/toast.js';

	export let profile: UserProfile;

	$: user = $authStore.user;

	async function handleFollow() {
		try {
			if (profile.isFollowing) {
				await unfollowUser(profile.username);
				toast.success('Unfollowed');
			} else {
				await followUser(profile.username);
				toast.success('Following!');
			}
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : 'Failed');
		}
	}

	async function handleMessage() {
		// Navigate to DM — conversation will be created on first message
		await goto(`/dm/${profile.id}`);
	}

	async function handleFriendRequest() {
		try {
			await sendFriendRequest(profile.username);
			toast.success('Friend request sent!');
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : 'Failed');
		}
	}

	function formatCount(n: number | undefined | null): string {
		if (!n) return '0';
		if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
		if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
		return n.toString();
	}
</script>

<div class="profile-header">
	<!-- Banner -->
	<div
		class="banner"
		style={profile.bannerUrl
			? `background-image: url('${profile.bannerUrl}')`
			: `background: ${profile.bannerColor ?? 'linear-gradient(135deg, #1a0533 0%, #2e1065 50%, #0a0a0f 100%)'}`}
	></div>

	<!-- Avatar row -->
	<div class="avatar-row">
		<div class="avatar-wrap">
			{#if profile.avatarUrl}
				<img src={profile.avatarUrl} alt="{profile.displayName}'s avatar" class="avatar" />
			{:else}
				<div class="avatar avatar-placeholder">
					{profile.displayName[0].toUpperCase()}
				</div>
			{/if}
		</div>

		{#if !profile.isSelf}
			<div class="actions">
				<button class="btn-primary" on:click={handleFollow}>
					{profile.isFollowing ? 'Following' : 'Follow'}
				</button>
				<button class="btn-secondary" on:click={handleMessage}>Message</button>
				{#if !profile.isFriend}
					<button class="btn-icon" title="Send friend request" aria-label="Send friend request" on:click={handleFriendRequest}>
						+
					</button>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Name + handle -->
	<div class="identity">
		<h1 class="display-name">
			{profile.displayName}
			{#if profile.isPremium}
				<span class="premium-badge" title="Yapper GoPro">🚀</span>
			{/if}
		</h1>
		<p class="username">@{profile.username}</p>
	</div>

	<!-- Stats row -->
	<div class="stats-row">
		<div class="stat">
			<span class="stat-value">{formatCount(profile.followerCount)}</span>
			<span class="stat-label">Followers</span>
		</div>
		<div class="stat">
			<span class="stat-value">{formatCount(profile.followingCount)}</span>
			<span class="stat-label">Following</span>
		</div>
		<div class="stat">
			<span class="stat-value">{formatCount(profile.hypeMomentCount)}</span>
			<span class="stat-label">Hype Moments</span>
		</div>
	</div>
</div>

<style>
	.profile-header {
		position: relative;
	}

	.banner {
		height: 220px;
		background-size: cover;
		background-position: center;
		/* ponytail: brand banner backdrop — intentional, theme-independent (sits under
		   the user's banner image; content below it is theme-responsive) */
		background-color: #1a0533;
	}

	.avatar-row {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		padding: 0 24px;
		margin-top: -56px;
		margin-bottom: 12px;
	}

	.avatar-wrap {
		width: 112px;
		height: 112px;
		border-radius: 50%;
		border: 4px solid var(--color-bg-base);
		overflow: hidden;
		flex-shrink: 0;
	}

	.avatar {
		width: 100%;
		height: 100%;
		object-fit: cover;
		border-radius: 50%;
	}

	.avatar-placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #7c3aed, #2e1065);
		color: white;
		font-size: 40px;
		font-weight: 800;
	}

	.actions {
		display: flex;
		gap: 8px;
		align-items: center;
		padding-bottom: 8px;
	}

	.btn-primary {
		padding: 8px 20px;
		background: var(--color-brand);
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.btn-primary:hover {
		opacity: 0.85;
	}

	.btn-secondary {
		padding: 8px 20px;
		background: transparent;
		color: var(--color-text-primary);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: border-color 150ms, background 150ms;
	}

	.btn-secondary:hover {
		background: var(--color-bg-elevated);
		border-color: var(--color-text-muted);
	}

	.btn-icon {
		width: 36px;
		height: 36px;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		color: var(--color-text-primary);
		font-size: 18px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 150ms;
	}

	.btn-icon:hover {
		background: rgba(124, 58, 237, 0.2);
	}

	.identity {
		padding: 0 24px;
	}

	.display-name {
		font-size: 24px;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0 0 4px;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.premium-badge {
		font-size: 18px;
	}

	.username {
		font-size: 14px;
		color: var(--color-text-secondary);
		margin: 0;
	}

	.stats-row {
		display: flex;
		gap: 24px;
		padding: 16px 24px 0;
	}

	.stat {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.stat-value {
		font-size: 18px;
		font-weight: 700;
		color: var(--color-text-primary);
	}

	.stat-label {
		font-size: 12px;
		color: var(--color-text-secondary);
	}
</style>
