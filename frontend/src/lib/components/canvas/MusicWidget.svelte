<script lang="ts">
	import { onMount } from 'svelte';
	import { skipVote, advanceTrack } from '$stores/canvas.js';
	import type { MusicNowPlaying, MusicQueueEntry } from '$stores/canvas.js';
	import MusicQueue from './MusicQueue.svelte';

	export let music: MusicNowPlaying;
	export let queue: MusicQueueEntry[] = [];
	export let skipVotes = 0;
	export let onlineMemberCount = 0;
	export let skipThresholdPct = 50;
	export let isAdminOrDj = false;
	export let serverId = '';

	let showQueue = false;
	let elapsedMs = 0;
	let elapsedTimer: ReturnType<typeof setInterval>;
	let skipLoading = false;
	let advanceLoading = false;
	let error = '';

	$: durationMs = (music.duration_secs ?? 0) * 1000;
	$: progressPct = durationMs > 0 ? Math.min(100, (elapsedMs / durationMs) * 100) : 0;
	$: ended = durationMs > 0 && elapsedMs >= durationMs;
	$: votesNeeded = Math.ceil((skipThresholdPct * onlineMemberCount) / 100);

	function formatTime(ms: number): string {
		const totalSec = Math.max(0, Math.floor(ms / 1000));
		const min = Math.floor(totalSec / 60);
		const sec = totalSec % 60;
		return `${min}:${sec.toString().padStart(2, '0')}`;
	}

	onMount(() => {
		elapsedMs = Date.now() - new Date(music.started_at).getTime();
		elapsedTimer = setInterval(() => {
			elapsedMs = Date.now() - new Date(music.started_at).getTime();
		}, 1000);
		return () => clearInterval(elapsedTimer);
	});

	async function handleSkip() {
		if (skipLoading) return;
		error = '';
		skipLoading = true;
		try {
			await skipVote(serverId);
		} catch {
			error = 'Skip vote failed';
		} finally {
			skipLoading = false;
		}
	}

	async function handleAdvance() {
		if (advanceLoading) return;
		error = '';
		advanceLoading = true;
		try {
			await advanceTrack(serverId, true);
		} catch {
			error = 'Advance failed';
		} finally {
			advanceLoading = false;
		}
	}
</script>

<div class="music-widget">
	<div class="now-playing">
		<div class="album-art-wrap">
			{#if music.album_art_url}
				<img class="album-art" src={music.album_art_url} alt="Album art" loading="lazy" decoding="async" />
			{:else}
				<div class="album-art-placeholder" aria-hidden="true">
					<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="12" cy="12" r="10"/>
						<circle cx="12" cy="12" r="3"/>
						<line x1="12" y1="2" x2="12" y2="9"/>
						<line x1="12" y1="15" x2="12" y2="22"/>
					</svg>
				</div>
			{/if}
			<div class="pulse-ring" aria-hidden="true"></div>
		</div>

		<div class="track-info">
			<p class="track-title" title={music.title}>{music.title}</p>
			<p class="track-artist" title={music.artist}>{music.artist}</p>
		</div>

		<div class="eq-bars" aria-label="Now playing" aria-hidden="true">
			{#if !ended}
				<span></span><span></span><span></span><span></span>
			{/if}
		</div>
	</div>

	<!-- Progress bar -->
	{#if durationMs > 0}
		<div class="progress-row">
			<span class="time-label">{formatTime(elapsedMs)}</span>
			<div class="progress-track">
				<div class="progress-fill" style="transform: scaleX({progressPct / 100})" class:ended></div>
			</div>
			<span class="time-label">{formatTime(durationMs)}</span>
		</div>
	{/if}

	{#if ended}
		<div class="ended-badge">Track ended</div>
	{/if}

	<!-- Controls -->
	<div class="controls">
		<button
			class="btn-control"
			on:click={handleSkip}
			disabled={skipLoading}
			title="Vote to skip"
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<polygon points="5 4 15 12 5 20 5 4"/>
				<line x1="19" y1="5" x2="19" y2="19"/>
			</svg>
			<span>{skipVotes}/{votesNeeded}</span>
		</button>

		<button
			class="btn-control"
			on:click={() => (showQueue = !showQueue)}
			class:active={showQueue}
			title="Toggle queue"
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<line x1="8" y1="6" x2="21" y2="6"/>
				<line x1="8" y1="12" x2="21" y2="12"/>
				<line x1="8" y1="18" x2="21" y2="18"/>
				<line x1="3" y1="6" x2="3.01" y2="6"/>
				<line x1="3" y1="12" x2="3.01" y2="12"/>
				<line x1="3" y1="18" x2="3.01" y2="18"/>
			</svg>
			<span>{queue.length}</span>
		</button>

		{#if isAdminOrDj}
			<button
				class="btn-control btn-next"
				on:click={handleAdvance}
				disabled={advanceLoading}
				title="Next track"
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<polygon points="5 4 15 12 5 20 5 4"/>
					<polygon points="13 4 23 12 13 20 13 4"/>
				</svg>
				<span>Next</span>
			</button>
		{/if}
	</div>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<!-- Queue panel -->
	{#if showQueue}
		<MusicQueue {queue} {serverId} {isAdminOrDj} />
	{/if}
</div>

<style>
	.music-widget {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem;
		background: linear-gradient(135deg, rgba(124, 58, 237, 0.15), rgba(167, 139, 250, 0.05));
		border: 1px solid rgba(124, 58, 237, 0.25);
		border-radius: var(--radius-md);
	}

	.now-playing {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.album-art-wrap {
		position: relative;
		flex-shrink: 0;
	}

	.album-art {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		object-fit: cover;
		animation: spin 8s linear infinite;
	}

	.album-art-placeholder {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: rgba(124, 58, 237, 0.2);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-brand-light);
		animation: spin 8s linear infinite;
	}

	.pulse-ring {
		position: absolute;
		inset: -4px;
		border-radius: 50%;
		border: 2px solid rgba(124, 58, 237, 0.4);
		animation: pulse-ring 2s ease-out infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }
	@keyframes pulse-ring {
		0%   { transform: scale(1); opacity: 0.6; }
		70%  { transform: scale(1.2); opacity: 0; }
		100% { transform: scale(1.2); opacity: 0; }
	}

	.track-info {
		flex: 1;
		min-width: 0;
	}

	.track-title {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.track-artist {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.eq-bars {
		display: flex;
		align-items: flex-end;
		gap: 2px;
		height: 20px;
		flex-shrink: 0;
	}

	.eq-bars span {
		display: block;
		width: 3px;
		height: 18px;
		background: var(--color-brand-light);
		border-radius: 2px;
		transform-origin: bottom;
		/* Animate transform, not height, so the eq bars don't trigger layout every frame. */
		animation: eq-pulse var(--dur, 0.8s) ease-in-out infinite alternate;
	}

	.eq-bars span:nth-child(1) { --dur: 0.7s; animation-delay: 0s; }
	.eq-bars span:nth-child(2) { --dur: 0.9s; animation-delay: 0.15s; }
	.eq-bars span:nth-child(3) { --dur: 0.6s; animation-delay: 0.3s; }
	.eq-bars span:nth-child(4) { --dur: 1.1s; animation-delay: 0.45s; }

	@keyframes eq-pulse { from { transform: scaleY(0.22); } to { transform: scaleY(1); } }

	/* Progress bar */
	.progress-row {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.time-label {
		font-size: 0.625rem;
		color: var(--color-text-muted);
		font-variant-numeric: tabular-nums;
		min-width: 2.5em;
	}

	.time-label:last-child {
		text-align: right;
	}

	.progress-track {
		flex: 1;
		height: 3px;
		background: rgba(124, 58, 237, 0.15);
		border-radius: 2px;
		overflow: hidden;
	}

	.progress-fill {
		width: 100%;
		height: 100%;
		background: var(--color-brand-light);
		border-radius: 2px;
		transform-origin: left;
		/* Animate transform instead of width to keep the fill off the layout path. */
		transition: transform 1s linear;
	}

	.progress-fill.ended {
		background: var(--color-text-muted);
	}

	.ended-badge {
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		text-align: center;
		font-weight: 600;
	}

	/* Controls */
	.controls {
		display: flex;
		gap: 0.375rem;
	}

	.btn-control {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.25rem;
		min-height: 44px;
		padding: 0.25rem 0.625rem;
		font-size: 0.6875rem;
		color: var(--color-text-secondary);
		background: rgba(124, 58, 237, 0.08);
		border: 1px solid rgba(124, 58, 237, 0.2);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.btn-control:hover:not(:disabled) {
		background: rgba(124, 58, 237, 0.18);
		color: var(--color-text-primary);
	}

	.btn-control:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.btn-control.active {
		background: rgba(124, 58, 237, 0.25);
		border-color: rgba(124, 58, 237, 0.4);
		color: var(--color-brand-light);
	}

	.btn-next {
		margin-left: auto;
	}

	.error {
		font-size: 0.6875rem;
		color: var(--color-error);
		margin: 0;
	}
</style>
