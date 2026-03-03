<script lang="ts">
	import { onMount } from 'svelte';
	import { getCanvasStore, loadCanvas, loadClips, votePoll } from '$stores/canvas.js';
	import MusicWidget from './MusicWidget.svelte';
	import PollWidget from './PollWidget.svelte';
	import ClipsCarousel from './ClipsCarousel.svelte';
	import type { ApiError } from '$api/client.js';

	export let serverId: string;

	$: canvasStore = getCanvasStore(serverId);
	$: canvas = $canvasStore;

	let voteError = '';
	let votingPollId = '';

	onMount(() => {
		loadCanvas(serverId).catch(() => {});
		loadClips(serverId).catch(() => {});
	});

	async function handleVote(pollId: string, optionIndex: number) {
		voteError = '';
		votingPollId = pollId;
		try {
			await votePoll(serverId, pollId, optionIndex);
		} catch (e) {
			const err = e as ApiError;
			voteError = err.status === 409 ? 'Already voted' : 'Vote failed';
		} finally {
			votingPollId = '';
		}
	}
</script>

<aside class="live-canvas">
	<header class="canvas-header">
		<span class="canvas-title">Canvas</span>
		<span class="live-dot" aria-label="Live"></span>
	</header>

	<div class="canvas-body">
		{#if canvas.loadingCanvas}
			<div class="loading-state">
				<div class="spinner" aria-label="Loading…"></div>
			</div>
		{:else}
			<!-- Music -->
			{#if canvas.music}
				<section class="canvas-section">
					<h3 class="section-label">Now Playing</h3>
					<MusicWidget music={canvas.music} />
				</section>
			{/if}

			<!-- Polls -->
			{#if canvas.polls.length > 0}
				<section class="canvas-section">
					<h3 class="section-label">Polls</h3>
					<div class="polls-list">
						{#each canvas.polls as poll}
							<PollWidget
								{poll}
								disabled={votingPollId === poll.id}
								on:vote={(e) => handleVote(poll.id, e.detail)}
							/>
						{/each}
					</div>
					{#if voteError}
						<p class="vote-error">{voteError}</p>
					{/if}
				</section>
			{/if}

			<!-- Clips -->
			{#if canvas.clips.length > 0}
				<section class="canvas-section">
					<ClipsCarousel clips={canvas.clips} />
				</section>
			{/if}

			<!-- Empty state -->
			{#if !canvas.music && canvas.polls.length === 0 && canvas.clips.length === 0}
				<div class="empty-state">
					<p>No canvas activity yet.</p>
					<p class="sub">Music, polls and clips appear here.</p>
				</div>
			{/if}
		{/if}
	</div>
</aside>

<style>
	.live-canvas {
		width: 240px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		background: var(--color-bg-surface);
		border-left: 1px solid var(--color-border);
		overflow: hidden;
	}

	.canvas-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.canvas-title {
		font-size: 0.8125rem;
		font-weight: 700;
		color: var(--color-text-primary);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		flex: 1;
	}

	.live-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-success);
		animation: blink 2s ease-in-out infinite;
	}

	@keyframes blink {
		0%, 100% { opacity: 1; }
		50%       { opacity: 0.3; }
	}

	.canvas-body {
		flex: 1;
		overflow-y: auto;
		padding: 0.75rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		scrollbar-width: thin;
		scrollbar-color: var(--color-border) transparent;
	}

	.canvas-section {
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

	.polls-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.vote-error {
		font-size: 0.75rem;
		color: var(--color-error);
		margin: 0;
	}

	.loading-state {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.spinner {
		width: 24px;
		height: 24px;
		border: 2px solid var(--color-border);
		border-top-color: var(--color-brand);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.empty-state {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		gap: 0.25rem;
		padding: 1rem;
	}

	.empty-state p {
		font-size: 0.8125rem;
		color: var(--color-text-secondary);
		margin: 0;
	}

	.empty-state .sub {
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}
</style>
