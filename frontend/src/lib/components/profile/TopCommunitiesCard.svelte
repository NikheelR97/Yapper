<script lang="ts">
	import { toast } from '$stores/toast.js';
	import { api } from '$api/client.js';
	import type { Community } from '$stores/profile.js';

	export let communities: Community[] = [];

	async function handleJoin(community: Community) {
		try {
			await api.post(`/api/v2/servers/${community.id}/join`);
			toast.success(`Joined ${community.name}!`);
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : 'Failed to join server');
		}
	}

	function formatCount(n: number | undefined | null): string {
		if (!n) return '0';
		if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
		return n.toString();
	}
</script>

<div class="card">
	<h3 class="card-title">Top Communities</h3>
	{#if communities.length === 0}
		<p class="empty">No communities yet.</p>
	{:else}
		<div class="list">
			{#each communities.slice(0, 3) as community}
				<div class="community-row">
					<div class="community-icon">
						{#if community.iconUrl}
							<img src={community.iconUrl} alt={community.name} />
						{:else}
							<div class="icon-placeholder">{community.name[0]}</div>
						{/if}
					</div>
					<div class="community-info">
						<span class="community-name">{community.name}</span>
						<span class="community-members">{formatCount(community.memberCount)} members</span>
					</div>
					<button class="join-btn" on:click={() => handleJoin(community)}>Join</button>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.card {
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 16px;
		padding: 20px;
	}

	.card-title {
		font-size: 14px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0 0 16px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.community-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.community-icon {
		width: 40px;
		height: 40px;
		border-radius: 12px;
		overflow: hidden;
		flex-shrink: 0;
	}

	.community-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.icon-placeholder {
		width: 100%;
		height: 100%;
		background: linear-gradient(135deg, #7c3aed, #2e1065);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		color: white;
		font-size: 16px;
	}

	.community-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.community-name {
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.community-members {
		font-size: 12px;
		color: var(--color-text-secondary);
	}

	.join-btn {
		padding: 6px 14px;
		background: rgba(124, 58, 237, 0.15);
		color: var(--color-brand-text);
		border: 1px solid rgba(124, 58, 237, 0.3);
		border-radius: 6px;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		flex-shrink: 0;
		transition: background 150ms;
	}

	.join-btn:hover {
		background: rgba(124, 58, 237, 0.3);
	}

	.empty {
		font-size: 14px;
		color: var(--color-text-secondary);
		margin: 0;
	}
</style>
