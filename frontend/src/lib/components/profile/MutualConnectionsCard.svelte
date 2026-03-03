<script lang="ts">
	import { goto } from '$app/navigation';
	import type { MutualFriend } from '$stores/profile.js';

	export let mutuals: MutualFriend[] = [];
	export let profileUsername: string = '';
</script>

<div class="card">
	<h3 class="card-title">Mutual Connections</h3>
	{#if mutuals.length === 0}
		<p class="empty">No mutual connections.</p>
	{:else}
		<div class="list">
			{#each mutuals.slice(0, 5) as mutual}
				<button class="mutual-row" on:click={() => goto(`/profile/${mutual.username}`)}>
					<div class="avatar">
						{#if mutual.avatarUrl}
							<img src={mutual.avatarUrl} alt={mutual.displayName} />
						{:else}
							<div class="avatar-placeholder">{mutual.displayName[0]}</div>
						{/if}
					</div>
					<div class="mutual-info">
						<span class="mutual-name">{mutual.displayName}</span>
						<span class="mutual-count">{mutual.mutualCount} mutual friends</span>
					</div>
				</button>
			{/each}
		</div>

		{#if mutuals.length > 5}
			<button class="view-all" on:click={() => goto(`/profile/${profileUsername}/mutuals`)}>
				View all {mutuals.length} →
			</button>
		{/if}
	{/if}
</div>

<style>
	.card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 16px;
		padding: 20px;
	}

	.card-title {
		font-size: 14px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0 0 16px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.mutual-row {
		display: flex;
		align-items: center;
		gap: 10px;
		background: none;
		border: none;
		cursor: pointer;
		padding: 4px 0;
		width: 100%;
		text-align: left;
		border-radius: 8px;
		transition: background 100ms;
	}

	.mutual-row:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.avatar {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		overflow: hidden;
		flex-shrink: 0;
	}

	.avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.avatar-placeholder {
		width: 100%;
		height: 100%;
		background: linear-gradient(135deg, #7c3aed, #2e1065);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		color: white;
		font-size: 14px;
	}

	.mutual-info {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.mutual-name {
		font-size: 14px;
		font-weight: 600;
		color: #f9fafb;
	}

	.mutual-count {
		font-size: 12px;
		color: #6b7280;
	}

	.view-all {
		display: block;
		margin-top: 12px;
		background: none;
		border: none;
		color: #7c3aed;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		padding: 0;
	}

	.view-all:hover {
		color: #a78bfa;
	}

	.empty {
		font-size: 14px;
		color: #6b7280;
		margin: 0;
	}
</style>
