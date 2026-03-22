<script lang="ts">
	import type { ClipV2 } from '$stores/canvas.js';
	import {
		addClipReaction,
		removeClipReaction,
		pinClip,
		unpinClip,
	} from '$stores/canvas.js';

	export let clips: ClipV2[] = [];
	export let pinnedClips: ClipV2[] = [];
	export let isAdmin = false;
	export let serverId = '';

	const REACTION_EMOJI = ['\u{1F44D}', '\u{2764}\u{FE0F}', '\u{1F525}', '\u{1F602}', '\u{1F62E}', '\u{1F44E}'];

	let openReactionClipId = '';
	let contextMenuClipId = '';
	let contextMenuPos = { x: 0, y: 0 };

	$: allClips = [
		...pinnedClips.map((c) => ({ ...c, _pinned: true as const })),
		...clips.filter((c) => !pinnedClips.some((p) => p.id === c.id)).map((c) => ({ ...c, _pinned: false as const })),
	];

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

	function topReactions(reactions: Record<string, number>): Array<{ emoji: string; count: number }> {
		return Object.entries(reactions)
			.filter(([, count]) => count > 0)
			.sort(([, a], [, b]) => b - a)
			.slice(0, 3)
			.map(([emoji, count]) => ({ emoji, count }));
	}

	async function handleReaction(clipId: string, emoji: string) {
		const clip = allClips.find((c) => c.id === clipId);
		if (!clip) return;
		const alreadyReacted = clip.my_reactions.includes(emoji);
		try {
			if (alreadyReacted) {
				await removeClipReaction(clipId, emoji);
			} else {
				await addClipReaction(clipId, emoji);
			}
		} catch {
			// Will update via WS
		}
		openReactionClipId = '';
	}

	async function handlePin(clipId: string) {
		contextMenuClipId = '';
		try {
			await pinClip(serverId, clipId);
		} catch {
			// Will update via WS
		}
	}

	async function handleUnpin(clipId: string) {
		contextMenuClipId = '';
		try {
			await unpinClip(serverId, clipId);
		} catch {
			// Will update via WS
		}
	}

	function handleContextMenu(e: MouseEvent, clipId: string) {
		if (!isAdmin) return;
		e.preventDefault();
		contextMenuClipId = clipId;
		contextMenuPos = { x: e.clientX, y: e.clientY };
	}

	function closeContextMenu() {
		contextMenuClipId = '';
	}
</script>

<svelte:window on:click={closeContextMenu} />

{#if allClips.length > 0}
<div class="clips-section">
	<h3 class="section-label">Recent Clips</h3>
	<div class="carousel" role="list">
		{#each allClips as clip (clip.id)}
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<div
				class="clip-card"
				role="listitem"
				title="Encrypted clip"
				on:contextmenu={(e) => handleContextMenu(e, clip.id)}
			>
				<div class="clip-thumb" aria-hidden="true">
					{#if clip._pinned}
						<span class="pin-badge" title="Pinned">
							<svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
								<path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5v6l1 1 1-1v-6h5v-2z"/>
							</svg>
						</span>
					{/if}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
						<polygon points="23 7 16 12 23 17 23 7"/>
						<rect x="1" y="5" width="15" height="14" rx="2" ry="2"/>
					</svg>
				</div>

				<!-- Reaction bar -->
				{#if topReactions(clip.reactions).length > 0}
					<div class="reaction-bar">
						{#each topReactions(clip.reactions) as r}
							<button
								class="reaction-chip"
								class:mine={clip.my_reactions.includes(r.emoji)}
								on:click|stopPropagation={() => handleReaction(clip.id, r.emoji)}
								title={r.emoji}
							>
								<span>{r.emoji}</span>
								<span class="r-count">{r.count}</span>
							</button>
						{/each}
					</div>
				{/if}

				<!-- Add reaction button -->
				<div class="reaction-add-wrap">
					<button
						class="btn-react"
						on:click|stopPropagation={() =>
							(openReactionClipId = openReactionClipId === clip.id ? '' : clip.id)}
						title="Add reaction"
					>
						<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<circle cx="12" cy="12" r="10"/>
							<path d="M8 14s1.5 2 4 2 4-2 4-2"/>
							<line x1="9" y1="9" x2="9.01" y2="9"/>
							<line x1="15" y1="9" x2="15.01" y2="9"/>
						</svg>
					</button>

					{#if openReactionClipId === clip.id}
						<div class="emoji-picker">
							{#each REACTION_EMOJI as emoji}
								<button
									class="emoji-pick"
									class:selected={clip.my_reactions.includes(emoji)}
									on:click|stopPropagation={() => handleReaction(clip.id, emoji)}
								>{emoji}</button>
							{/each}
						</div>
					{/if}
				</div>

				<span class="clip-time">{formatTime(clip.created_at)}</span>
			</div>
		{/each}
	</div>
</div>
{/if}

<!-- Context menu for admin pin/unpin -->
{#if contextMenuClipId && isAdmin}
	<div class="context-menu" style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px">
		{#if allClips.find((c) => c.id === contextMenuClipId)?._pinned}
			<button on:click={() => handleUnpin(contextMenuClipId)}>Unpin from Canvas</button>
		{:else}
			<button on:click={() => handlePin(contextMenuClipId)}>Pin to Canvas</button>
		{/if}
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
		position: relative;
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
		position: relative;
	}

	.clip-card:hover .clip-thumb {
		border-color: var(--color-brand);
	}

	.pin-badge {
		position: absolute;
		top: 2px;
		right: 2px;
		color: var(--color-brand-light);
	}

	/* Reactions */
	.reaction-bar {
		display: flex;
		gap: 2px;
	}

	.reaction-chip {
		display: flex;
		align-items: center;
		gap: 1px;
		padding: 0 3px;
		font-size: 0.625rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		background: var(--color-bg-surface);
		cursor: pointer;
		line-height: 1.2;
		transition: all var(--transition-fast);
	}

	.reaction-chip.mine {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.1);
	}

	.reaction-chip:hover {
		border-color: var(--color-brand);
	}

	.r-count {
		font-size: 0.5625rem;
		color: var(--color-text-muted);
	}

	.reaction-add-wrap {
		position: relative;
	}

	.btn-react {
		display: flex;
		padding: 1px 3px;
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		border-radius: var(--radius-sm);
	}

	.btn-react:hover {
		color: var(--color-brand-light);
	}

	.emoji-picker {
		position: absolute;
		bottom: 100%;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		gap: 2px;
		padding: 4px;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
		z-index: 10;
		white-space: nowrap;
	}

	.emoji-pick {
		font-size: 1rem;
		padding: 2px 3px;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		background: none;
		cursor: pointer;
		line-height: 1;
		transition: all var(--transition-fast);
	}

	.emoji-pick:hover { background: rgba(124, 58, 237, 0.1); }
	.emoji-pick.selected { border-color: var(--color-brand); }

	.clip-time {
		font-size: 0.625rem;
		color: var(--color-text-muted);
		white-space: nowrap;
	}

	/* Context menu */
	.context-menu {
		position: fixed;
		z-index: 50;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
		padding: 0.25rem;
	}

	.context-menu button {
		display: block;
		width: 100%;
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		color: var(--color-text-primary);
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		cursor: pointer;
		text-align: left;
		white-space: nowrap;
	}

	.context-menu button:hover {
		background: rgba(124, 58, 237, 0.1);
	}
</style>
