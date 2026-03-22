<script lang="ts">
	import type { MusicQueueEntry } from '$stores/canvas.js';
	import { removeFromQueue, reorderQueue } from '$stores/canvas.js';
	import AddTrackModal from './AddTrackModal.svelte';

	export let queue: MusicQueueEntry[] = [];
	export let serverId: string;
	export let isAdminOrDj = false;

	let showAddTrack = false;
	let removingId = '';
	let dragIdx: number | null = null;
	let dropIdx: number | null = null;

	function formatDuration(secs: number): string {
		const m = Math.floor(secs / 60);
		const s = secs % 60;
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	async function handleRemove(trackId: string) {
		if (removingId) return;
		removingId = trackId;
		try {
			await removeFromQueue(serverId, trackId);
		} catch {
			// Queue will refresh via WS
		} finally {
			removingId = '';
		}
	}

	function handleDragStart(idx: number) {
		if (!isAdminOrDj) return;
		dragIdx = idx;
	}

	function handleDragOver(e: DragEvent, idx: number) {
		if (dragIdx === null || !isAdminOrDj) return;
		e.preventDefault();
		dropIdx = idx;
	}

	async function handleDrop(idx: number) {
		if (dragIdx === null || dragIdx === idx || !isAdminOrDj) {
			dragIdx = null;
			dropIdx = null;
			return;
		}

		const reordered = [...queue];
		const [moved] = reordered.splice(dragIdx, 1);
		reordered.splice(idx, 0, moved);
		dragIdx = null;
		dropIdx = null;

		try {
			await reorderQueue(serverId, reordered.map((t) => t.id));
		} catch {
			// Queue will refresh via WS
		}
	}

	function handleDragEnd() {
		dragIdx = null;
		dropIdx = null;
	}
</script>

<div class="queue-panel">
	<div class="queue-header">
		<span class="queue-title">Queue ({queue.length})</span>
		{#if isAdminOrDj}
			<button class="btn-add" on:click={() => (showAddTrack = true)}>
				<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<line x1="12" y1="5" x2="12" y2="19"/>
					<line x1="5" y1="12" x2="19" y2="12"/>
				</svg>
				Add
			</button>
		{/if}
	</div>

	{#if queue.length === 0}
		<p class="empty">Queue is empty. {isAdminOrDj ? 'Add a track to get started.' : ''}</p>
	{:else}
		<ul class="queue-list">
			{#each queue as track, i (track.id)}
				<li
					class="queue-item"
					class:dragging={dragIdx === i}
					class:drop-target={dropIdx === i}
					draggable={isAdminOrDj}
					on:dragstart={() => handleDragStart(i)}
					on:dragover={(e) => handleDragOver(e, i)}
					on:drop={() => handleDrop(i)}
					on:dragend={handleDragEnd}
					role="listitem"
				>
					{#if isAdminOrDj}
						<span class="drag-handle" aria-hidden="true">
							<svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
								<circle cx="9" cy="5" r="1.5"/><circle cx="15" cy="5" r="1.5"/>
								<circle cx="9" cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/>
								<circle cx="9" cy="19" r="1.5"/><circle cx="15" cy="19" r="1.5"/>
							</svg>
						</span>
					{/if}
					<div class="track-info">
						<span class="track-title" title={track.title}>{track.title}</span>
						<span class="track-artist" title={track.artist}>{track.artist}</span>
					</div>
					<span class="track-dur">{formatDuration(track.duration_secs)}</span>
					{#if isAdminOrDj}
						<button
							class="btn-remove"
							disabled={removingId === track.id}
							on:click={() => handleRemove(track.id)}
							title="Remove"
						>
							<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<line x1="18" y1="6" x2="6" y2="18"/>
								<line x1="6" y1="6" x2="18" y2="18"/>
							</svg>
						</button>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</div>

{#if showAddTrack}
	<AddTrackModal {serverId} onClose={() => (showAddTrack = false)} />
{/if}

<style>
	.queue-panel {
		border-top: 1px solid rgba(124, 58, 237, 0.15);
		padding-top: 0.5rem;
	}

	.queue-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.375rem;
	}

	.queue-title {
		font-size: 0.6875rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-muted);
	}

	.btn-add {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.6875rem;
		color: var(--color-brand-light);
		background: none;
		border: none;
		cursor: pointer;
		padding: 0.125rem 0.25rem;
		border-radius: var(--radius-sm);
	}

	.btn-add:hover {
		background: rgba(124, 58, 237, 0.12);
	}

	.empty {
		font-size: 0.75rem;
		color: var(--color-text-muted);
		margin: 0;
		text-align: center;
		padding: 0.5rem 0;
	}

	.queue-list {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 200px;
		overflow-y: auto;
		scrollbar-width: thin;
		scrollbar-color: var(--color-border) transparent;
	}

	.queue-item {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.3rem 0.375rem;
		border-radius: var(--radius-sm);
		background: rgba(124, 58, 237, 0.04);
		transition: background var(--transition-fast);
	}

	.queue-item:hover {
		background: rgba(124, 58, 237, 0.1);
	}

	.queue-item.dragging {
		opacity: 0.4;
	}

	.queue-item.drop-target {
		border-top: 2px solid var(--color-brand-light);
	}

	.drag-handle {
		cursor: grab;
		color: var(--color-text-muted);
		flex-shrink: 0;
	}

	.track-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.track-title {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.track-artist {
		font-size: 0.625rem;
		color: var(--color-text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.track-dur {
		font-size: 0.625rem;
		color: var(--color-text-muted);
		font-variant-numeric: tabular-nums;
		flex-shrink: 0;
	}

	.btn-remove {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.125rem;
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		border-radius: var(--radius-sm);
		flex-shrink: 0;
	}

	.btn-remove:hover:not(:disabled) {
		color: var(--color-error);
		background: rgba(239, 68, 68, 0.1);
	}

	.btn-remove:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
