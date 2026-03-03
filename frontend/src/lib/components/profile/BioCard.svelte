<script lang="ts">
	import type { UserProfile } from '$stores/profile.js';

	export let profile: UserProfile;
</script>

<div class="bio-card">
	{#if profile.bio}
		<p class="bio">{profile.bio}</p>
	{:else}
		<p class="bio empty">No bio yet.</p>
	{/if}

	{#if profile.tags && profile.tags.length > 0}
		<div class="tags">
			{#each profile.tags as tag}
				<span class="tag">{tag}</span>
			{/each}
		</div>
	{/if}

	<div class="bio-stats">
		<div class="bio-stat">
			<span class="stat-icon">⚡</span>
			<span class="stat-text">{profile.hypeMomentCount} Hype Moments</span>
		</div>
		{#if profile.topCommunities && profile.topCommunities.length > 0}
			<div class="bio-stat">
				<span class="stat-icon">🌐</span>
				<span class="stat-text">{profile.topCommunities[0].name}</span>
			</div>
		{/if}
	</div>
</div>

<style>
	.bio-card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 16px;
		padding: 20px;
		backdrop-filter: blur(8px);
	}

	.bio {
		font-size: 14px;
		color: #d1d5db;
		line-height: 1.6;
		margin: 0 0 16px;
	}

	.bio.empty {
		color: #6b7280;
		font-style: italic;
	}

	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-bottom: 16px;
	}

	.tag {
		padding: 4px 12px;
		background: rgba(124, 58, 237, 0.2);
		color: #a78bfa;
		border: 1px solid rgba(124, 58, 237, 0.3);
		border-radius: 20px;
		font-size: 12px;
		font-weight: 500;
	}

	.bio-stats {
		display: flex;
		flex-direction: column;
		gap: 8px;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		padding-top: 16px;
	}

	.bio-stat {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		color: #9ca3af;
	}

	.stat-icon {
		font-size: 14px;
	}
</style>
