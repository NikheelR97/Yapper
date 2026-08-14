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
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 16px;
		padding: 20px;
	}

	.bio {
		font-size: 14px;
		color: var(--color-text-secondary);
		line-height: 1.6;
		margin: 0 0 16px;
	}

	.bio.empty {
		color: var(--color-text-secondary);
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
		color: var(--color-brand-text);
		border: 1px solid rgba(124, 58, 237, 0.3);
		border-radius: 20px;
		font-size: 12px;
		font-weight: 500;
	}

	.bio-stats {
		display: flex;
		flex-direction: column;
		gap: 8px;
		border-top: 1px solid var(--color-border);
		padding-top: 16px;
	}

	.bio-stat {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		color: var(--color-text-secondary);
	}

	.stat-icon {
		font-size: 14px;
	}
</style>
