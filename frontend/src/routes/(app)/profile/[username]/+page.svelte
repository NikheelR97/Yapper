<script lang="ts">
	import { page } from "$app/stores";
	import { onMount } from "svelte";
	import { profileStore, loadProfile } from "$stores/profile.js";
	import ProfileHeader from "$components/profile/ProfileHeader.svelte";
	import BioCard from "$components/profile/BioCard.svelte";
	import HypeMoments from "$components/profile/HypeMoments.svelte";
	import TopCommunitiesCard from "$components/profile/TopCommunitiesCard.svelte";
	import MutualConnectionsCard from "$components/profile/MutualConnectionsCard.svelte";
	import Skeleton from "$components/Skeleton.svelte";

	$: username = $page.params.username;
	$: ({ profile, loading, error } = $profileStore);

	onMount(async () => {
		if (username) await loadProfile(username);
	});

	// Reload when username param changes
	$: if (username) loadProfile(username);
</script>

<svelte:head>
	<title
		>{profile
			? `${profile.displayName} (@${profile.username}) — Yapper`
			: "Profile — Yapper"}</title
	>
</svelte:head>

<div class="profile-page">
	{#if loading && !profile}
		<!-- Skeleton loading state -->
		<div class="profile-skeleton">
			<Skeleton height="220px" />
			<div class="skeleton-body">
				<Skeleton width="112px" height="112px" circle />
				<div style="margin-top: 16px">
					<Skeleton width="200px" height="24px" />
					<div style="margin-top: 8px">
						<Skeleton width="120px" height="16px" />
					</div>
				</div>
			</div>
		</div>
	{:else if error}
		<div class="error-state">
			<span class="error-icon">⚠</span>
			<h2>Profile not found</h2>
			<p>{error}</p>
		</div>
	{:else if profile}
		<div class="profile-content">
			<ProfileHeader {profile} />

			<div class="profile-body">
				<div class="main-col">
					<BioCard {profile} />
					<HypeMoments
						moments={profile.hypeMoments}
						loading={false}
					/>
				</div>

				<aside class="side-col">
					{#if profile.topCommunities && profile.topCommunities.length > 0}
						<TopCommunitiesCard
							communities={profile.topCommunities}
						/>
					{/if}
					{#if profile.mutualFriends && profile.mutualFriends.length > 0}
						<MutualConnectionsCard
							mutuals={profile.mutualFriends}
							profileUsername={profile.username}
						/>
					{/if}
				</aside>
			</div>
		</div>
	{/if}
</div>

<style>
	.profile-page {
		flex: 1;
		overflow-y: auto;
		background: var(--color-bg-base, #0a0a0f);
		min-height: 0;
	}

	.profile-skeleton {
		max-width: 900px;
		margin: 0 auto;
	}

	.skeleton-body {
		padding: 16px 24px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.error-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		padding: 80px 24px;
		color: var(--color-text-secondary);
		text-align: center;
	}

	.error-icon {
		font-size: 48px;
	}

	.error-state h2 {
		font-size: 20px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
	}

	.error-state p {
		margin: 0;
		font-size: 14px;
	}

	.profile-content {
		max-width: 960px;
		margin: 0 auto;
		padding-bottom: 48px;
	}

	.profile-body {
		display: grid;
		grid-template-columns: 1fr 300px;
		gap: 24px;
		padding: 24px;
	}

	.main-col {
		display: flex;
		flex-direction: column;
		gap: 24px;
		min-width: 0;
	}

	.side-col {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	@media (max-width: 768px) {
		.profile-body {
			grid-template-columns: 1fr;
		}

		.side-col {
			order: -1;
		}
	}
</style>
