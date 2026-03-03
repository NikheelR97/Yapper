<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { Poll } from '$stores/canvas.js';

	export let poll: Poll;
	export let disabled = false;

	const dispatch = createEventDispatcher<{ vote: number }>();

	$: totalVotes = Object.values(poll.vote_counts).reduce((a, b) => a + b, 0);
	$: isExpired = poll.ends_at ? new Date(poll.ends_at) < new Date() : false;

	function getCount(index: number): number {
		return poll.vote_counts[String(index)] ?? 0;
	}

	function getPct(index: number): number {
		if (totalVotes === 0) return 0;
		return Math.round((getCount(index) / totalVotes) * 100);
	}

	function handleVote(index: number) {
		if (disabled || isExpired) return;
		dispatch('vote', index);
	}
</script>

<div class="poll-widget">
	<p class="poll-question">{poll.question}</p>

	{#if isExpired}
		<span class="ended-badge">Ended</span>
	{/if}

	<ul class="poll-options">
		{#each poll.options as opt}
			<li>
				<button
					class="poll-option"
					class:disabled={disabled || isExpired}
					on:click={() => handleVote(opt.index)}
					{disabled}
				>
					<span class="opt-label">{opt.text}</span>
					<span class="opt-pct">{getPct(opt.index)}%</span>
					<div
						class="fill-bar"
						style="width: {getPct(opt.index)}%"
						aria-hidden="true"
					></div>
				</button>
			</li>
		{/each}
	</ul>

	<p class="vote-total">{totalVotes} vote{totalVotes !== 1 ? 's' : ''}</p>
</div>

<style>
	.poll-widget {
		padding: 0.75rem;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
	}

	.poll-question {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 0.5rem;
	}

	.ended-badge {
		display: inline-block;
		font-size: 0.65rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-muted);
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		padding: 1px 6px;
		margin-bottom: 0.5rem;
	}

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

	.poll-option.disabled {
		cursor: default;
	}

	.fill-bar {
		position: absolute;
		inset-block: 0;
		left: 0;
		background: rgba(124, 58, 237, 0.18);
		transition: width 0.4s ease;
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

	.vote-total {
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		margin: 0.5rem 0 0;
		text-align: right;
	}
</style>
