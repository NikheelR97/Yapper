<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { closePoll } from '$stores/canvas.js';
	import type { Poll } from '$stores/canvas.js';

	export let poll: Poll;
	export let disabled = false;
	export let isAdmin = false;

	const dispatch = createEventDispatcher<{ vote: number }>();

	let closingPoll = false;

	$: totalVotes = Object.values(poll.vote_counts).reduce((a, b) => a + b, 0);
	$: isClosed = poll.status === 'closed';
	$: isExpired = poll.ends_at ? new Date(poll.ends_at) < new Date() : false;
	$: votingDisabled = disabled || isClosed || isExpired;

	function getCount(index: number): number {
		return poll.vote_counts[String(index)] ?? 0;
	}

	function getPct(index: number): number {
		if (totalVotes === 0) return 0;
		return Math.round((getCount(index) / totalVotes) * 100);
	}

	function handleVote(index: number) {
		if (votingDisabled || poll.my_vote !== null) return;
		dispatch('vote', index);
	}

	async function handleClose() {
		if (closingPoll) return;
		closingPoll = true;
		try {
			await closePoll(poll.id);
		} catch {
			// Will update via WS
		} finally {
			closingPoll = false;
		}
	}

	const BINARY_ICONS = ['thumbs-up', 'thumbs-down'];
</script>

<div class="poll-widget" class:closed={isClosed || isExpired}>
	<div class="poll-header">
		<p class="poll-question">{poll.question}</p>
		<div class="badges">
			{#if poll.anonymous}
				<span class="badge">Anonymous</span>
			{/if}
			{#if isClosed || isExpired}
				<span class="badge ended">Ended</span>
			{/if}
		</div>
	</div>

	{#if poll.poll_type === 'binary'}
		<!-- Binary: two large buttons -->
		<div class="binary-options">
			{#each poll.options as opt}
				<button
					class="binary-btn"
					class:voted={poll.my_vote === opt.index}
					class:disabled={votingDisabled || poll.my_vote !== null}
					disabled={votingDisabled || poll.my_vote !== null}
					on:click={() => handleVote(opt.index)}
				>
					<span class="binary-icon">
						{#if opt.index === 0}
							<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<path d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3H14z"/>
								<path d="M7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3"/>
							</svg>
						{:else}
							<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<path d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3H10z"/>
								<path d="M17 2h3a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-3"/>
							</svg>
						{/if}
					</span>
					<span class="binary-label">{opt.text}</span>
					<span class="binary-count">{getCount(opt.index)}</span>
				</button>
			{/each}
		</div>

	{:else if poll.poll_type === 'emoji_reaction'}
		<!-- Emoji reaction: large emoji buttons in a row -->
		<div class="emoji-options">
			{#each poll.options as opt}
				<button
					class="emoji-btn"
					class:voted={poll.my_vote === opt.index}
					class:disabled={votingDisabled || poll.my_vote !== null}
					disabled={votingDisabled || poll.my_vote !== null}
					on:click={() => handleVote(opt.index)}
					title={opt.text}
				>
					<span class="emoji-char">{opt.text}</span>
					<span class="emoji-count">{getCount(opt.index)}</span>
				</button>
			{/each}
		</div>

	{:else}
		<!-- Multiple choice: fill-bar options -->
		<ul class="poll-options">
			{#each poll.options as opt}
				<li>
					<button
						class="poll-option"
						class:voted={poll.my_vote === opt.index}
						class:disabled={votingDisabled || poll.my_vote !== null}
						disabled={votingDisabled || poll.my_vote !== null}
						on:click={() => handleVote(opt.index)}
					>
						<span class="opt-label">{opt.text}</span>
						<span class="opt-pct">{getPct(opt.index)}%</span>
						<div
							class="fill-bar"
							style="transform: scaleX({getPct(opt.index) / 100})"
							aria-hidden="true"
						></div>
					</button>
				</li>
			{/each}
		</ul>
	{/if}

	<div class="poll-footer">
		<p class="vote-total">{totalVotes} vote{totalVotes !== 1 ? 's' : ''}</p>
		{#if isAdmin && !isClosed && !isExpired}
			<button class="btn-close-poll" on:click={handleClose} disabled={closingPoll}>
				Close
			</button>
		{/if}
	</div>
</div>

<style>
	.poll-widget {
		padding: 0.75rem;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
	}

	.poll-widget.closed {
		opacity: 0.75;
	}

	.poll-header {
		margin-bottom: 0.5rem;
	}

	.poll-question {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 0.25rem;
	}

	.badges {
		display: flex;
		gap: 0.25rem;
	}

	.badge {
		display: inline-block;
		font-size: 0.6rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-muted);
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		padding: 1px 6px;
	}

	.badge.ended {
		color: var(--color-error);
		border-color: var(--color-error);
		background: rgba(239, 68, 68, 0.08);
	}

	/* Binary poll */
	.binary-options {
		display: flex;
		gap: 0.5rem;
	}

	.binary-btn {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		padding: 0.625rem 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-bg-surface);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.binary-btn:not(.disabled):hover {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.06);
	}

	.binary-btn.voted {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.12);
	}

	.binary-btn.disabled {
		cursor: default;
	}

	.binary-icon {
		color: var(--color-text-secondary);
	}

	.binary-label {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-text-primary);
	}

	.binary-count {
		font-size: 0.875rem;
		font-weight: 700;
		color: var(--color-brand-light);
	}

	/* Emoji reaction poll */
	.emoji-options {
		display: flex;
		gap: 0.375rem;
		flex-wrap: wrap;
	}

	.emoji-btn {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
		padding: 0.375rem 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-bg-surface);
		cursor: pointer;
		transition: all var(--transition-fast);
		min-width: 2.5rem;
	}

	.emoji-btn:not(.disabled):hover {
		border-color: var(--color-brand);
		transform: scale(1.08);
	}

	.emoji-btn.voted {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.12);
	}

	.emoji-btn.disabled {
		cursor: default;
	}

	.emoji-char {
		font-size: 1.25rem;
		line-height: 1;
	}

	.emoji-count {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-text-secondary);
	}

	/* Multiple choice */
	.poll-options {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.poll-option {
		position: relative;
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.4rem 0.6rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		background: var(--color-bg-surface);
		cursor: pointer;
		overflow: hidden;
		text-align: left;
		transition: border-color var(--transition-fast);
	}

	.poll-option:not(.disabled):hover {
		border-color: var(--color-brand);
	}

	.poll-option.voted {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.06);
	}

	.poll-option.disabled {
		cursor: default;
	}

	.fill-bar {
		position: absolute;
		inset-block: 0;
		left: 0;
		width: 100%;
		background: rgba(124, 58, 237, 0.18);
		transform-origin: left;
		/* Animate transform, not width, to keep the result bar off the layout path. */
		transition: transform 0.4s ease;
		pointer-events: none;
	}

	.opt-label {
		position: relative;
		font-size: 0.8125rem;
		color: var(--color-text-primary);
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.opt-pct {
		position: relative;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-brand-light);
		margin-left: 0.5rem;
		flex-shrink: 0;
	}

	/* Footer */
	.poll-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: 0.5rem;
	}

	.vote-total {
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		margin: 0;
	}

	.btn-close-poll {
		font-size: 0.6875rem;
		color: var(--color-error);
		background: none;
		border: 1px solid var(--color-error);
		border-radius: var(--radius-sm);
		padding: 0.125rem 0.5rem;
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.btn-close-poll:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.08);
	}

	.btn-close-poll:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
