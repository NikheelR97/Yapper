<script lang="ts">
	import type { Clip } from '$stores/canvas.js';

	export let clips: Clip[];

	function formatTime(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diffMs = now.getTime() - d.getTime();
		const diffMin = Math.floor(diffMs / 60_000);
		if (diffMin < 60) return `${diffMin}m ago`;
		const diffHr = Math.floor(diffMin / 60);
		if (diffHr < 24) return `${diffHr}h ago`;
		return d.toLocaleDateString();
	}
</script>

{#if clips.length > 0}
<div class="clips-section">
	<h3 class="section-label">Recent Clips</h3>
	<div class="carousel" role="list">
		{#each clips as clip}
			<div class="clip-card" role="listitem" title="Encrypted clip">
				<div class="clip-thumb" aria-hidden="true">
					<!-- Encrypted — show placeholder icon -->
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
						<polygon points="23 7 16 12 23 17 23 7"/>
						<rect x="1" y="5" width="15" height="14" rx="2" ry="2"/>
					</svg>
				</div>
				<span class="clip-time">{formatTime(clip.created_at)}</span>
			</div>
		{/each}
	</div>
</div>
{/if}

<style>
	.clips-section {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.section-label {
		font-size: 0.6875rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-muted);
		margin: 0;
	}

	.carousel {
		display: flex;
		gap: 0.5rem;
		overflow-x: auto;
		padding-bottom: 4px;
		scrollbar-width: thin;
		scrollbar-color: var(--color-border) transparent;
	}

	.clip-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		flex-shrink: 0;
		cursor: pointer;
	}

	.clip-thumb {
		width: 64px;
		height: 48px;
		border-radius: var(--radius-sm);
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		transition: border-color var(--transition-fast);
	}

	.clip-card:hover .clip-thumb {
		border-color: var(--color-brand);
	}

	.clip-time {
		font-size: 0.625rem;
		color: var(--color-text-muted);
		white-space: nowrap;
	}
</style>
