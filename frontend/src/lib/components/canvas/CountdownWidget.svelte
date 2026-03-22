<script lang="ts">
	import { onMount } from 'svelte';
	import { updateCanvasEvent, deleteCanvasEvent } from '$stores/canvas.js';
	import type { CanvasEvent } from '$stores/canvas.js';

	export let event: CanvasEvent;
	export let isAdmin = false;

	let remaining = { days: 0, hours: 0, minutes: 0, seconds: 0 };
	let isLive = false;
	let countdownTimer: ReturnType<typeof setInterval>;

	let editing = false;
	let editTitle = '';
	let editDescription = '';
	let editEventAt = '';
	let editSubmitting = false;
	let editError = '';

	let confirmDelete = false;
	let deleting = false;

	onMount(() => {
		const tick = () => {
			const diff = new Date(event.event_at).getTime() - Date.now();
			if (diff <= 0) {
				isLive = true;
				remaining = { days: 0, hours: 0, minutes: 0, seconds: 0 };
			} else {
				isLive = false;
				remaining = {
					days: Math.floor(diff / 86400000),
					hours: Math.floor((diff % 86400000) / 3600000),
					minutes: Math.floor((diff % 3600000) / 60000),
					seconds: Math.floor((diff % 60000) / 1000),
				};
			}
		};
		tick();
		countdownTimer = setInterval(tick, 1000);
		return () => clearInterval(countdownTimer);
	});

	function pad(n: number): string {
		return n.toString().padStart(2, '0');
	}

	function startEdit() {
		editTitle = event.title;
		editDescription = event.description ?? '';
		editEventAt = new Date(event.event_at).toISOString().slice(0, 16);
		editError = '';
		editing = true;
	}

	async function handleEditSubmit() {
		if (editSubmitting) return;
		editError = '';
		editSubmitting = true;
		try {
			await updateCanvasEvent(event.id, {
				title: editTitle.trim(),
				description: editDescription.trim() || null,
				event_at: new Date(editEventAt).toISOString(),
			});
			editing = false;
		} catch {
			editError = 'Update failed';
		} finally {
			editSubmitting = false;
		}
	}

	async function handleDelete() {
		if (deleting) return;
		deleting = true;
		try {
			await deleteCanvasEvent(event.id);
			confirmDelete = false;
		} catch {
			// Will update via WS
		} finally {
			deleting = false;
		}
	}
</script>

<div class="countdown-widget" class:live={isLive}>
	{#if editing}
		<!-- Edit form -->
		<form class="edit-form" on:submit|preventDefault={handleEditSubmit}>
			<input
				type="text"
				bind:value={editTitle}
				maxlength="200"
				placeholder="Event title"
				required
			/>
			<input
				type="text"
				bind:value={editDescription}
				maxlength="500"
				placeholder="Description (optional)"
			/>
			<input
				type="datetime-local"
				bind:value={editEventAt}
				required
			/>
			{#if editError}
				<p class="error">{editError}</p>
			{/if}
			<div class="edit-actions">
				<button type="button" class="btn-sm" on:click={() => (editing = false)}>Cancel</button>
				<button type="submit" class="btn-sm btn-primary" disabled={editSubmitting}>Save</button>
			</div>
		</form>
	{:else if isLive}
		<!-- Live state -->
		<div class="live-banner">
			<span class="live-dot"></span>
			<span class="live-text">LIVE NOW</span>
		</div>
		<p class="event-title">{event.title}</p>
		{#if event.description}
			<p class="event-desc">{event.description}</p>
		{/if}
	{:else}
		<!-- Countdown state -->
		<p class="event-title">{event.title}</p>
		{#if event.description}
			<p class="event-desc">{event.description}</p>
		{/if}

		<div class="counter">
			<div class="counter-segment">
				<span class="counter-value">{pad(remaining.days)}</span>
				<span class="counter-label">days</span>
			</div>
			<span class="counter-sep">:</span>
			<div class="counter-segment">
				<span class="counter-value">{pad(remaining.hours)}</span>
				<span class="counter-label">hrs</span>
			</div>
			<span class="counter-sep">:</span>
			<div class="counter-segment">
				<span class="counter-value">{pad(remaining.minutes)}</span>
				<span class="counter-label">min</span>
			</div>
			<span class="counter-sep">:</span>
			<div class="counter-segment">
				<span class="counter-value">{pad(remaining.seconds)}</span>
				<span class="counter-label">sec</span>
			</div>
		</div>
	{/if}

	<!-- Admin controls -->
	{#if isAdmin && !editing}
		<div class="admin-controls">
			<button class="btn-sm" on:click={startEdit}>Edit</button>
			{#if confirmDelete}
				<button class="btn-sm btn-danger" on:click={handleDelete} disabled={deleting}>
					{deleting ? '...' : 'Confirm'}
				</button>
				<button class="btn-sm" on:click={() => (confirmDelete = false)}>Cancel</button>
			{:else}
				<button class="btn-sm btn-danger" on:click={() => (confirmDelete = true)}>Delete</button>
			{/if}
		</div>
	{/if}
</div>

<style>
	.countdown-widget {
		padding: 0.75rem;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.countdown-widget.live {
		border-color: var(--color-success);
		box-shadow: 0 0 8px rgba(34, 197, 94, 0.15);
	}

	/* Live banner */
	.live-banner {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.live-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-success);
		animation: pulse-live 1.5s ease-in-out infinite;
	}

	@keyframes pulse-live {
		0%, 100% { opacity: 1; transform: scale(1); }
		50% { opacity: 0.6; transform: scale(1.2); }
	}

	.live-text {
		font-size: 0.6875rem;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--color-success);
	}

	.event-title {
		font-size: 0.875rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
	}

	.event-desc {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		margin: 0;
	}

	/* Countdown counter */
	.counter {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.25rem;
	}

	.counter-segment {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
	}

	.counter-value {
		font-size: 1.25rem;
		font-weight: 800;
		color: var(--color-brand-light);
		font-variant-numeric: tabular-nums;
		background: rgba(124, 58, 237, 0.08);
		border-radius: var(--radius-sm);
		padding: 0.125rem 0.375rem;
		min-width: 2rem;
		text-align: center;
	}

	.counter-label {
		font-size: 0.5625rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-muted);
	}

	.counter-sep {
		font-size: 1.125rem;
		font-weight: 700;
		color: var(--color-text-muted);
		margin-bottom: 1rem;
	}

	/* Admin controls */
	.admin-controls {
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
		transition: all var(--transition-fast);
	}

	.btn-sm:hover:not(:disabled) { background: var(--color-bg-elevated); }
	.btn-sm:disabled { opacity: 0.5; cursor: default; }

	.btn-primary {
		background: var(--color-brand);
		color: white;
		border-color: var(--color-brand);
	}

	.btn-danger {
		color: var(--color-error);
		border-color: var(--color-error);
	}

	.btn-danger:hover:not(:disabled) { background: rgba(239, 68, 68, 0.08); }

	/* Edit form */
	.edit-form {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.edit-form input {
		padding: 0.3rem 0.5rem;
		font-size: 0.8125rem;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		outline: none;
	}

	.edit-form input:focus { border-color: var(--color-brand); }

	.edit-actions {
		display: flex;
		gap: 0.25rem;
		justify-content: flex-end;
	}

	.error {
		font-size: 0.6875rem;
		color: var(--color-error);
		margin: 0;
	}
</style>
