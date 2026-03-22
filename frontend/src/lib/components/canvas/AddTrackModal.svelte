<script lang="ts">
	import { enqueueTrack } from '$stores/canvas.js';
	import type { ApiError } from '$api/client.js';

	export let serverId: string;
	export let onClose: () => void;

	let artist = '';
	let title = '';
	let durationMin = '';
	let durationSec = '';
	let sourceUrl = '';
	let albumArtUrl = '';
	let submitting = false;
	let error = '';

	$: durationSecs =
		(parseInt(durationMin || '0', 10) || 0) * 60 +
		(parseInt(durationSec || '0', 10) || 0);
	$: valid = artist.trim().length > 0 && title.trim().length > 0 && durationSecs > 0;

	async function handleSubmit() {
		if (!valid || submitting) return;
		error = '';
		submitting = true;
		try {
			await enqueueTrack(serverId, {
				artist: artist.trim(),
				title: title.trim(),
				duration_secs: durationSecs,
				source_url: sourceUrl.trim() || null,
				album_art_url: albumArtUrl.trim() || null,
			});
			onClose();
		} catch (e) {
			const apiErr = e as ApiError;
			error = apiErr.message ?? 'Failed to add track';
		} finally {
			submitting = false;
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onClose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="modal-backdrop" on:click={handleBackdropClick} role="dialog" aria-modal="true" aria-label="Add track">
	<div class="modal">
		<div class="modal-header">
			<h3>Add Track</h3>
			<button class="btn-close" on:click={onClose} aria-label="Close">
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<line x1="18" y1="6" x2="6" y2="18"/>
					<line x1="6" y1="6" x2="18" y2="18"/>
				</svg>
			</button>
		</div>

		<form on:submit|preventDefault={handleSubmit}>
			<div class="field">
				<label for="add-track-artist">Artist</label>
				<input
					id="add-track-artist"
					type="text"
					bind:value={artist}
					maxlength="200"
					placeholder="Artist name"
					required
				/>
			</div>

			<div class="field">
				<label for="add-track-title">Title</label>
				<input
					id="add-track-title"
					type="text"
					bind:value={title}
					maxlength="200"
					placeholder="Track title"
					required
				/>
			</div>

			<div class="field">
				<label>Duration</label>
				<div class="duration-row">
					<input
						type="number"
						bind:value={durationMin}
						min="0"
						max="1440"
						placeholder="min"
						class="dur-input"
					/>
					<span class="dur-sep">:</span>
					<input
						type="number"
						bind:value={durationSec}
						min="0"
						max="59"
						placeholder="sec"
						class="dur-input"
					/>
					<span class="dur-preview">{durationSecs > 0 ? `${durationSecs}s` : ''}</span>
				</div>
			</div>

			<div class="field">
				<label for="add-track-source">Source URL <span class="optional">(optional)</span></label>
				<input
					id="add-track-source"
					type="url"
					bind:value={sourceUrl}
					placeholder="https://..."
				/>
			</div>

			<div class="field">
				<label for="add-track-art">Album Art URL <span class="optional">(optional)</span></label>
				<input
					id="add-track-art"
					type="url"
					bind:value={albumArtUrl}
					placeholder="https://..."
				/>
			</div>

			{#if error}
				<p class="error">{error}</p>
			{/if}

			<div class="actions">
				<button type="button" class="btn-cancel" on:click={onClose}>Cancel</button>
				<button type="submit" class="btn-submit" disabled={!valid || submitting}>
					{submitting ? 'Adding...' : 'Add to Queue'}
				</button>
			</div>
		</form>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.modal {
		width: 320px;
		max-width: 90vw;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: 1rem;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
	}

	.modal-header h3 {
		font-size: 0.9375rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
	}

	.btn-close {
		display: flex;
		padding: 0.25rem;
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		border-radius: var(--radius-sm);
	}

	.btn-close:hover {
		color: var(--color-text-primary);
		background: var(--color-bg-elevated);
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-text-secondary);
	}

	.optional {
		font-weight: 400;
		color: var(--color-text-muted);
	}

	input {
		padding: 0.375rem 0.5rem;
		font-size: 0.8125rem;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		outline: none;
		transition: border-color var(--transition-fast);
	}

	input:focus {
		border-color: var(--color-brand);
	}

	.duration-row {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.dur-input {
		width: 3.5rem;
		text-align: center;
	}

	.dur-sep {
		font-size: 0.875rem;
		color: var(--color-text-muted);
		font-weight: 700;
	}

	.dur-preview {
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		margin-left: 0.5rem;
	}

	.error {
		font-size: 0.75rem;
		color: var(--color-error);
		margin: 0;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 0.25rem;
	}

	.btn-cancel,
	.btn-submit {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		border-radius: var(--radius-sm);
		cursor: pointer;
		border: 1px solid var(--color-border);
	}

	.btn-cancel {
		background: var(--color-bg-elevated);
		color: var(--color-text-secondary);
	}

	.btn-cancel:hover {
		background: var(--color-bg-surface);
	}

	.btn-submit {
		background: var(--color-brand);
		color: white;
		border-color: var(--color-brand);
		font-weight: 600;
	}

	.btn-submit:hover:not(:disabled) {
		opacity: 0.9;
	}

	.btn-submit:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
