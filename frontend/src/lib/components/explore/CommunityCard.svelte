<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { Community, SearchServer } from '$stores/explore.js';

	export let server: Community | SearchServer;

	const dispatch = createEventDispatcher<{ join: string }>();

	function memberCount(): number {
		return 'member_count' in server ? server.member_count : 0;
	}

	function handleJoin() {
		dispatch('join', server.slug);
	}
</script>

<article class="community-card">
	<div class="card-top">
		<div class="server-icon">
			{#if server.icon_url}
				<img src={server.icon_url} alt={server.name} />
			{:else}
				<span class="icon-initial">{server.name.charAt(0).toUpperCase()}</span>
			{/if}
		</div>
		<div class="server-info">
			<h3 class="server-name">{server.name}</h3>
			{#if memberCount() > 0}
				<span class="member-count">
					<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
						<circle cx="9" cy="7" r="4"/>
					</svg>
					{memberCount().toLocaleString()} members
				</span>
			{/if}
		</div>
	</div>

	{#if server.description}
		<p class="description">{server.description}</p>
	{:else}
		<p class="description placeholder">No description.</p>
	{/if}

	{#if server.tags.length > 0}
		<div class="tags">
			{#each server.tags.slice(0, 4) as tag}
				<span class="tag">#{tag}</span>
			{/each}
		</div>
	{/if}

	<button class="join-btn" on:click={handleJoin}>Join</button>
</article>

<style>
	.community-card {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
		padding: 1rem;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		transition: border-color var(--transition-base);
	}

	.community-card:hover {
		border-color: rgba(124, 58, 237, 0.4);
	}

	.card-top {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.server-icon {
		width: 44px;
		height: 44px;
		border-radius: var(--radius-md);
		overflow: hidden;
		flex-shrink: 0;
		background: rgba(124, 58, 237, 0.15);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.server-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.icon-initial {
		font-size: 1.25rem;
		font-weight: 800;
		color: var(--color-brand-light);
	}

	.server-info {
		flex: 1;
		min-width: 0;
	}

	.server-name {
		font-size: 0.9375rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.member-count {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	.description {
		font-size: 0.8125rem;
		color: var(--color-text-secondary);
		margin: 0;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.description.placeholder {
		color: var(--color-text-muted);
		font-style: italic;
	}

	.tags {
		display: flex;
		gap: 0.25rem;
		flex-wrap: wrap;
	}

	.tag {
		font-size: 0.6875rem;
		color: var(--color-brand-light);
		background: rgba(124, 58, 237, 0.1);
		padding: 1px 6px;
		border-radius: var(--radius-full);
	}

	.join-btn {
		align-self: flex-start;
		padding: 0.35rem 1rem;
		border-radius: var(--radius-md);
		border: none;
		background: var(--color-brand);
		color: #fff;
		font-size: 0.8125rem;
		font-weight: 600;
		cursor: pointer;
		transition: background var(--transition-fast);
		margin-top: auto;
	}

	.join-btn:hover {
		background: var(--color-brand-dark);
	}
</style>
