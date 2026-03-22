<script lang="ts">
	import { createPoll } from '$stores/canvas.js';
	import type { ApiError } from '$api/client.js';

	export let channelId: string;
	export let onClose: () => void;

	type PollType = 'binary' | 'multiple_choice' | 'emoji_reaction';

	let pollType: PollType = 'multiple_choice';
	let question = '';
	let options = ['', ''];
	let anonymous = false;
	let duration = '';
	let submitting = false;
	let error = '';

	const DURATIONS: Array<{ label: string; value: string }> = [
		{ label: 'No limit', value: '' },
		{ label: '15m', value: '15' },
		{ label: '1h', value: '60' },
		{ label: '6h', value: '360' },
		{ label: '24h', value: '1440' },
	];

	const DEFAULT_EMOJI = ['\u{1F44D}', '\u{2764}\u{FE0F}', '\u{1F525}', '\u{1F602}', '\u{1F62E}', '\u{1F44E}'];

	$: if (pollType === 'binary' && options.length !== 2) {
		options = ['Yes', 'No'];
	}
	$: if (pollType === 'emoji_reaction' && (options.length < 2 || options.length > 6)) {
		options = DEFAULT_EMOJI.slice(0, 4);
	}

	$: validOptions = options.filter((o) => o.trim().length > 0);
	$: valid = question.trim().length > 0 && validOptions.length >= 2;

	function addOption() {
		if (options.length >= 6) return;
		options = [...options, ''];
	}

	function removeOption(idx: number) {
		if (options.length <= 2) return;
		options = options.filter((_, i) => i !== idx);
	}

	function toggleEmoji(emoji: string) {
		if (options.includes(emoji)) {
			if (options.length <= 2) return;
			options = options.filter((o) => o !== emoji);
		} else {
			if (options.length >= 6) return;
			options = [...options, emoji];
		}
	}

	function computeEndsAt(): string | null {
		if (!duration) return null;
		const ms = parseInt(duration, 10) * 60 * 1000;
		return new Date(Date.now() + ms).toISOString();
	}

	async function handleSubmit() {
		if (!valid || submitting) return;
		error = '';
		submitting = true;
		try {
			await createPoll(channelId, question.trim(), validOptions, {
				poll_type: pollType,
				ends_at: computeEndsAt(),
				anonymous,
			});
			onClose();
		} catch (e) {
			const apiErr = e as ApiError;
			error = apiErr.message ?? 'Failed to create poll';
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
<div class="modal-backdrop" on:click={handleBackdropClick} role="dialog" aria-modal="true" aria-label="Create poll">
	<div class="modal">
		<div class="modal-header">
			<h3>Create Poll</h3>
			<button class="btn-close" on:click={onClose} aria-label="Close">
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<line x1="18" y1="6" x2="6" y2="18"/>
					<line x1="6" y1="6" x2="18" y2="18"/>
				</svg>
			</button>
		</div>

		<form on:submit|preventDefault={handleSubmit}>
			<!-- Poll type -->
			<div class="field">
				<label>Type</label>
				<div class="type-selector">
					<button
						type="button"
						class="type-btn"
						class:active={pollType === 'binary'}
						on:click={() => (pollType = 'binary')}
					>Binary</button>
					<button
						type="button"
						class="type-btn"
						class:active={pollType === 'multiple_choice'}
						on:click={() => (pollType = 'multiple_choice')}
					>Multiple</button>
					<button
						type="button"
						class="type-btn"
						class:active={pollType === 'emoji_reaction'}
						on:click={() => (pollType = 'emoji_reaction')}
					>Emoji</button>
				</div>
			</div>

			<!-- Question -->
			<div class="field">
				<label for="poll-question">Question</label>
				<input
					id="poll-question"
					type="text"
					bind:value={question}
					maxlength="500"
					placeholder="Ask a question..."
					required
				/>
			</div>

			<!-- Options -->
			{#if pollType === 'emoji_reaction'}
				<div class="field">
					<label>Reactions ({options.length}/6)</label>
					<div class="emoji-grid">
						{#each DEFAULT_EMOJI as emoji}
							<button
								type="button"
								class="emoji-toggle"
								class:selected={options.includes(emoji)}
								on:click={() => toggleEmoji(emoji)}
							>{emoji}</button>
						{/each}
					</div>
				</div>
			{:else}
				<div class="field">
					<label>Options ({options.length}/6)</label>
					<div class="options-list">
						{#each options as opt, i}
							<div class="option-row">
								<input
									type="text"
									bind:value={options[i]}
									maxlength="200"
									placeholder="Option {i + 1}"
									disabled={pollType === 'binary'}
								/>
								{#if options.length > 2 && pollType !== 'binary'}
									<button type="button" class="btn-remove-opt" on:click={() => removeOption(i)}>
										<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
										</svg>
									</button>
								{/if}
							</div>
						{/each}
						{#if options.length < 6 && pollType !== 'binary'}
							<button type="button" class="btn-add-opt" on:click={addOption}>
								+ Add option
							</button>
						{/if}
					</div>
				</div>
			{/if}

			<!-- Duration -->
			<div class="field">
				<label>Duration</label>
				<div class="duration-btns">
					{#each DURATIONS as d}
						<button
							type="button"
							class="dur-btn"
							class:active={duration === d.value}
							on:click={() => (duration = d.value)}
						>{d.label}</button>
					{/each}
				</div>
			</div>

			<!-- Anonymous -->
			<label class="anon-toggle">
				<input type="checkbox" bind:checked={anonymous} />
				<span>Anonymous voting</span>
			</label>

			{#if error}
				<p class="error">{error}</p>
			{/if}

			<div class="actions">
				<button type="button" class="btn-cancel" on:click={onClose}>Cancel</button>
				<button type="submit" class="btn-submit" disabled={!valid || submitting}>
					{submitting ? 'Creating...' : 'Create Poll'}
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
		width: 360px;
		max-width: 90vw;
		max-height: 85vh;
		overflow-y: auto;
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

	.btn-close:hover { color: var(--color-text-primary); background: var(--color-bg-elevated); }

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

	input[type="text"] {
		padding: 0.375rem 0.5rem;
		font-size: 0.8125rem;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		outline: none;
	}

	input[type="text"]:focus { border-color: var(--color-brand); }
	input[type="text"]:disabled { opacity: 0.6; }

	/* Type selector */
	.type-selector {
		display: flex;
		gap: 0.25rem;
	}

	.type-btn {
		flex: 1;
		padding: 0.3rem 0.5rem;
		font-size: 0.75rem;
		font-weight: 500;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		background: var(--color-bg-surface);
		color: var(--color-text-secondary);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.type-btn.active {
		background: rgba(124, 58, 237, 0.12);
		border-color: var(--color-brand);
		color: var(--color-brand-light);
	}

	/* Options */
	.options-list {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.option-row {
		display: flex;
		gap: 0.25rem;
	}

	.option-row input { flex: 1; }

	.btn-remove-opt {
		display: flex;
		align-items: center;
		padding: 0.25rem;
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
	}

	.btn-remove-opt:hover { color: var(--color-error); }

	.btn-add-opt {
		font-size: 0.75rem;
		color: var(--color-brand-light);
		background: none;
		border: none;
		cursor: pointer;
		padding: 0.25rem 0;
		text-align: left;
	}

	.btn-add-opt:hover { text-decoration: underline; }

	/* Emoji grid */
	.emoji-grid {
		display: flex;
		gap: 0.375rem;
		flex-wrap: wrap;
	}

	.emoji-toggle {
		font-size: 1.25rem;
		padding: 0.375rem;
		border: 2px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-bg-surface);
		cursor: pointer;
		transition: all var(--transition-fast);
		line-height: 1;
	}

	.emoji-toggle.selected {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.1);
	}

	/* Duration */
	.duration-btns {
		display: flex;
		gap: 0.25rem;
	}

	.dur-btn {
		padding: 0.25rem 0.5rem;
		font-size: 0.6875rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		background: var(--color-bg-surface);
		color: var(--color-text-secondary);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.dur-btn.active {
		background: rgba(124, 58, 237, 0.12);
		border-color: var(--color-brand);
		color: var(--color-brand-light);
	}

	/* Anonymous toggle */
	.anon-toggle {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}

	.anon-toggle input[type="checkbox"] {
		accent-color: var(--color-brand);
	}

	.anon-toggle span {
		font-size: 0.8125rem;
		color: var(--color-text-secondary);
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

	.btn-cancel, .btn-submit {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		border-radius: var(--radius-sm);
		cursor: pointer;
		border: 1px solid var(--color-border);
	}

	.btn-cancel { background: var(--color-bg-elevated); color: var(--color-text-secondary); }
	.btn-cancel:hover { background: var(--color-bg-surface); }

	.btn-submit {
		background: var(--color-brand);
		color: white;
		border-color: var(--color-brand);
		font-weight: 600;
	}

	.btn-submit:hover:not(:disabled) { opacity: 0.9; }
	.btn-submit:disabled { opacity: 0.5; cursor: default; }
</style>
