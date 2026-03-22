<script lang="ts">
	import { onMount } from 'svelte';
	import {
		getCanvasStore,
		loadCanvasState,
		loadCanvas,
		loadClips,
		votePoll,
		createCanvasEvent,
	} from '$stores/canvas.js';
	import { authStore } from '$stores/auth.js';
	import { serversStore } from '$stores/servers.js';
	import type { ApiError } from '$api/client.js';
	import MusicWidget from './MusicWidget.svelte';
	import PollWidget from './PollWidget.svelte';
	import PollCreator from './PollCreator.svelte';
	import ClipsCarousel from './ClipsCarousel.svelte';
	import CountdownWidget from './CountdownWidget.svelte';

	export let serverId: string;
	export let channelId = '';

	$: canvasStore = getCanvasStore(serverId);
	$: canvas = $canvasStore;

	let voteError = '';
	let votingPollId = '';
	let showPollCreator = false;
	let showEventForm = false;

	// Event creation form
	let eventTitle = '';
	let eventDesc = '';
	let eventAt = '';
	let eventSubmitting = false;
	let eventError = '';

	// Derive admin/DJ status from the servers store membership data
	$: server = $serversStore.servers.find((s) => s.id === serverId);
	$: isAdmin = server?.isOwner ?? false;
	$: isAdminOrDj = isAdmin;

	onMount(() => {
		if (channelId) {
			loadCanvasState(serverId, channelId).catch((err) => {
				console.error('[canvas] Failed to load state:', err);
			});
		} else {
			loadCanvas(serverId).catch((err) => {
				console.error('[canvas] Failed to load canvas:', err);
			});
			loadClips(serverId).catch((err) => {
				console.error('[canvas] Failed to load clips:', err);
			});
		}
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

	async function handleCreateEvent() {
		if (eventSubmitting || !eventTitle.trim() || !eventAt) return;
		eventError = '';
		eventSubmitting = true;
		try {
			await createCanvasEvent(serverId, {
				title: eventTitle.trim(),
				description: eventDesc.trim() || null,
				event_at: new Date(eventAt).toISOString(),
			});
			showEventForm = false;
			eventTitle = '';
			eventDesc = '';
			eventAt = '';
		} catch {
			eventError = 'Failed to create event';
		} finally {
			eventSubmitting = false;
		}
	}
</script>

<aside class="live-canvas">
	<header class="canvas-header">
		<span class="canvas-title">Canvas</span>
		<span class="live-dot" aria-label="Live"></span>
	</header>

	<div class="canvas-body">
		{#if canvas.loading}
			<div class="loading-state">
				<div class="spinner" aria-label="Loading..."></div>
			</div>
		{:else}
			<!-- Event / Countdown -->
			{#if canvas.event}
				<section class="canvas-section">
					<h3 class="section-label">Event</h3>
					<CountdownWidget event={canvas.event} {isAdmin} />
				</section>
			{:else if isAdmin}
				<section class="canvas-section">
					{#if showEventForm}
						<form class="event-form" on:submit|preventDefault={handleCreateEvent}>
							<input type="text" bind:value={eventTitle} maxlength="200" placeholder="Event title" required />
							<input type="text" bind:value={eventDesc} maxlength="500" placeholder="Description (optional)" />
							<input type="datetime-local" bind:value={eventAt} required />
							{#if eventError}
								<p class="inline-error">{eventError}</p>
							{/if}
							<div class="form-actions">
								<button type="button" class="btn-sm" on:click={() => (showEventForm = false)}>Cancel</button>
								<button type="submit" class="btn-sm btn-primary" disabled={eventSubmitting}>Create</button>
							</div>
						</form>
					{:else}
						<button class="btn-add-section" on:click={() => (showEventForm = true)}>+ Add Event</button>
					{/if}
				</section>
			{/if}

			<!-- Music -->
			{#if canvas.music.now_playing}
				<section class="canvas-section">
					<h3 class="section-label">Now Playing</h3>
					<MusicWidget
						music={canvas.music.now_playing}
						queue={canvas.music.queue}
						skipVotes={canvas.music.skip_votes}
						onlineMemberCount={canvas.music.online_members}
						skipThresholdPct={canvas.music.skip_threshold_pct}
						{isAdminOrDj}
						{serverId}
					/>
				</section>
			{/if}

			<!-- Polls -->
			{#if canvas.polls.length > 0 || (channelId && isAdmin)}
				<section class="canvas-section">
					<div class="section-header">
						<h3 class="section-label">Polls</h3>
						{#if channelId && isAdmin}
							<button class="btn-add-inline" on:click={() => (showPollCreator = true)}>+</button>
						{/if}
					</div>
					<div class="polls-list">
						{#each canvas.polls.slice(0, 3) as poll (poll.id)}
							<PollWidget
								{poll}
								disabled={votingPollId === poll.id}
								{isAdmin}
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
			{#if canvas.clips.length > 0 || canvas.pinned_clips.length > 0}
				<section class="canvas-section">
					<ClipsCarousel
						clips={canvas.clips}
						pinnedClips={canvas.pinned_clips}
						{isAdmin}
						{serverId}
					/>
				</section>
			{/if}

			<!-- Empty state -->
			{#if !canvas.event && !canvas.music.now_playing && canvas.polls.length === 0 && canvas.clips.length === 0 && canvas.pinned_clips.length === 0}
				<div class="empty-state">
					<p>No canvas activity yet.</p>
					<p class="sub">Music, polls, clips and events appear here.</p>
				</div>
			{/if}
		{/if}
	</div>
</aside>

{#if showPollCreator && channelId}
	<PollCreator {channelId} onClose={() => (showPollCreator = false)} />
{/if}

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

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.section-label {
		font-size: 0.6875rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-muted);
		margin: 0;
	}

	.btn-add-inline {
		width: 18px;
		height: 18px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.875rem;
		font-weight: 700;
		color: var(--color-text-muted);
		background: none;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.btn-add-inline:hover {
		color: var(--color-brand-light);
		border-color: var(--color-brand);
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

	@keyframes spin { to { transform: rotate(360deg); } }

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

	.btn-add-section {
		font-size: 0.75rem;
		color: var(--color-brand-light);
		background: none;
		border: 1px dashed var(--color-border);
		border-radius: var(--radius-md);
		padding: 0.5rem;
		cursor: pointer;
		text-align: center;
		transition: all var(--transition-fast);
	}

	.btn-add-section:hover {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.04);
	}

	/* Event form */
	.event-form {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.event-form input {
		padding: 0.3rem 0.5rem;
		font-size: 0.8125rem;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		outline: none;
	}

	.event-form input:focus { border-color: var(--color-brand); }

	.form-actions {
		display: flex;
		gap: 0.25rem;
		justify-content: flex-end;
	}

	.btn-sm {
		padding: 0.125rem 0.5rem;
		font-size: 0.6875rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		background: var(--color-bg-surface);
		color: var(--color-text-secondary);
		cursor: pointer;
	}

	.btn-sm:hover:not(:disabled) { background: var(--color-bg-elevated); }
	.btn-sm:disabled { opacity: 0.5; cursor: default; }

	.btn-primary {
		background: var(--color-brand);
		color: white;
		border-color: var(--color-brand);
	}

	.inline-error {
		font-size: 0.6875rem;
		color: var(--color-error);
		margin: 0;
	}
</style>
