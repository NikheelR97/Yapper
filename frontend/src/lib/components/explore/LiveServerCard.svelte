<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { LiveServer } from '$stores/explore.js';

	export let server: LiveServer;

	const dispatch = createEventDispatcher<{ join: string }>();

	function formatLastActive(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diffMin = Math.floor((now.getTime() - d.getTime()) / 60_000);
		if (diffMin < 1) return 'Just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		return `${Math.floor(diffMin / 60)}h ago`;
	}

	function handleJoin() {
		dispatch('join', server.id);
	}
</script>

<div class="live-card" on:click={handleJoin} on:keydown={(e) => e.key === 'Enter' && handleJoin()} role="button" tabindex="0">
	<div class="card-gradient" aria-hidden="true"></div>

	<div class="card-content">
		<div class="card-header">
			<div class="server-icon">
				{#if server.icon_url}
					<img src={server.icon_url} alt={server.name} />
				{:else}
					<span class="icon-initial">{server.name.charAt(0).toUpperCase()}</span>
				{/if}
			</div>
			<div class="server-meta">
				<h3 class="server-name">{server.name}</h3>
				<span class="member-count">
					<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
						<circle cx="9" cy="7" r="4"/>
						<path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
						<path d="M16 3.13a4 4 0 0 1 0 7.75"/>
					</svg>
					{server.member_count.toLocaleString()}
				</span>
			</div>
			<div class="live-badge">
				<span class="pulse" aria-hidden="true"></span>
				LIVE
			</div>
		</div>

		{#if server.description}
			<p class="description">{server.description}</p>
		{/if}

		<footer class="card-footer">
			{#if server.tags.length > 0}
				<div class="tags">
					{#each server.tags.slice(0, 3) as tag}
						<span class="tag">#{tag}</span>
					{/each}
				</div>
			{/if}
			<span class="last-active">{formatLastActive(server.last_active)}</span>
		</footer>
	</div>
</div>

<style>
	.live-card {
		position: relative;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(124, 58, 237, 0.25);
		overflow: hidden;
		cursor: pointer;
		transition: transform var(--transition-base), border-color var(--transition-base);
	}

	.live-card:hover {
		transform: translateY(-2px);
		border-color: rgba(124, 58, 237, 0.6);
	}

	.card-gradient {
		position: absolute;
		inset: 0;
		background: linear-gradient(135deg, rgba(124, 58, 237, 0.12), rgba(167, 139, 250, 0.04));
		pointer-events: none;
	}

	.card-content {
		position: relative;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.card-header {
		display: flex;
		align-items: center;
		gap: 0.625rem;
	}

	.server-icon {
		width: 40px;
		height: 40px;
		border-radius: var(--radius-md);
		overflow: hidden;
		flex-shrink: 0;
		background: rgba(124, 58, 237, 0.2);
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
		font-size: 1.125rem;
		font-weight: 800;
		color: var(--color-brand-light);
	}

	.server-meta {
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

	.live-badge {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 3px 8px;
		border-radius: var(--radius-full);
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		font-size: 0.6875rem;
		font-weight: 800;
		color: #f87171;
		letter-spacing: 0.06em;
		flex-shrink: 0;
	}

	.pulse {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #ef4444;
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; transform: scale(1); }
		50%       { opacity: 0.4; transform: scale(0.8); }
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

	.card-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
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

	.last-active {
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		white-space: nowrap;
		flex-shrink: 0;
	}
</style>
