<script lang="ts">
	import type { Tag } from '$stores/explore.js';

	export let tags: Tag[];
	export let activeTag: string | null = null;

	import { createEventDispatcher } from 'svelte';
	const dispatch = createEventDispatcher<{ select: string | null }>();

	function toggle(tag: string) {
		dispatch('select', activeTag === tag ? null : tag);
	}
</script>

<div class="trending-tags" aria-label="Trending tags">
	{#each tags as t}
		<button
			class="tag-chip"
			class:active={activeTag === t.tag}
			on:click={() => toggle(t.tag)}
		>
			<span class="hash">#</span>{t.tag}
			<span class="count">{t.server_count}</span>
		</button>
	{/each}
</div>

<style>
	.trending-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
	}

	.tag-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		padding: 0.25rem 0.625rem;
		border-radius: var(--radius-full);
		border: 1px solid var(--color-border);
		background: var(--color-bg-elevated);
		color: var(--color-text-secondary);
		font-size: 0.8125rem;
		cursor: pointer;
		transition: border-color var(--transition-fast), color var(--transition-fast),
			background var(--transition-fast);
		white-space: nowrap;
	}

	.tag-chip:hover {
		border-color: var(--color-brand);
		color: var(--color-text-primary);
	}

	.tag-chip.active {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.15);
		color: var(--color-brand-light);
	}

	.hash {
		color: var(--color-brand-light);
		font-weight: 700;
	}

	.count {
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		margin-left: 0.1rem;
	}
</style>
