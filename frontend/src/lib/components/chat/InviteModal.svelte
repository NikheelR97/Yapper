<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { createInvite } from '$stores/servers.js';

	export let open = false;
	export let serverId = '';
	export let serverName = '';
	export let channelName = '';

	const dispatch = createEventDispatcher<{ close: void }>();

	type Expiry = 'never' | '24h' | '7d' | '30d';
	type MaxUses = 'unlimited' | 'custom';

	let inviteCode = '';
	let generating = false;
	let copied = false;
	let expiry: Expiry = '24h';
	let maxUsesMode: MaxUses = 'unlimited';
	let maxUsesValue = 10;
	let error = '';

	$: inviteLink = inviteCode ? `yapperhq.com/join/${inviteCode}` : '';

	$: if (open && !inviteCode) {
		void generate();
	}

	$: if (!open) {
		inviteCode = '';
		copied = false;
		error = '';
	}

	async function generate() {
		if (!serverId || generating) return;
		generating = true;
		error = '';
		try {
			inviteCode = await createInvite(serverId);
		} catch {
			error = 'Failed to generate invite link.';
		} finally {
			generating = false;
		}
	}

	async function copyLink() {
		if (!inviteLink) return;
		try {
			await navigator.clipboard.writeText(`https://${inviteLink}`);
			copied = true;
			setTimeout(() => { copied = false; }, 2000);
		} catch {
			// fallback: select input
		}
	}

	function handleBackdrop() {
		dispatch('close');
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') dispatch('close');
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
	<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
	<div class="backdrop" on:click={handleBackdrop}>
		<div
			class="modal"
			role="dialog"
			aria-modal="true"
			aria-label="Invite people"
			tabindex="-1"
			on:click|stopPropagation
		>
			<div class="modal-header">
				<h2 class="modal-title">
					Invite People
					{#if channelName}
						<span class="channel-ref">to #{channelName}</span>
					{:else if serverName}
						<span class="channel-ref">to {serverName}</span>
					{/if}
				</h2>
				<button class="close-btn" on:click={() => dispatch('close')} aria-label="Close">
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
					</svg>
				</button>
			</div>

			<p class="section-label">Share this link</p>

			<!-- Invite link field -->
			<div class="link-row">
				{#if generating}
					<div class="link-input loading">Generating…</div>
				{:else if error}
					<div class="link-input error-text">{error}</div>
				{:else}
					<div class="link-input" title={inviteLink}>{inviteLink || '—'}</div>
				{/if}
				<button
					class="copy-btn"
					class:copied
					on:click={copyLink}
					disabled={!inviteLink || generating}
					aria-label={copied ? 'Copied!' : 'Copy link'}
				>
					{#if copied}
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<polyline points="20 6 9 17 4 12"/>
						</svg>
						Copied!
					{:else}
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
							<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
						</svg>
						Copy
					{/if}
				</button>
			</div>

			<!-- Options -->
			<div class="options-grid">
				<!-- Expiry -->
				<div class="option-group">
					<p class="option-label">Expiry</p>
					<div class="radio-row">
						<label class="radio-opt">
							<input type="radio" bind:group={expiry} value="never" />
							<span>Never</span>
						</label>
						<label class="radio-opt">
							<input type="radio" bind:group={expiry} value="24h" />
							<span>Expire in:</span>
							<select
								class="inline-select"
								bind:value={expiry}
								on:change={() => { if (expiry === 'never') expiry = '24h'; }}
								disabled={expiry === 'never'}
							>
								<option value="24h">24 hours</option>
								<option value="7d">7 days</option>
								<option value="30d">30 days</option>
							</select>
						</label>
					</div>
				</div>

				<!-- Max uses -->
				<div class="option-group">
					<p class="option-label">Max uses</p>
					<div class="radio-row">
						<label class="radio-opt">
							<input type="radio" bind:group={maxUsesMode} value="unlimited" />
							<span>Unlimited</span>
						</label>
						<label class="radio-opt">
							<input type="radio" bind:group={maxUsesMode} value="custom" />
							<span>Max uses:</span>
							<input
								type="number"
								class="uses-input"
								bind:value={maxUsesValue}
								min="1"
								max="1000"
								disabled={maxUsesMode === 'unlimited'}
							/>
						</label>
					</div>
				</div>
			</div>

			<button class="btn-generate" on:click={generate} disabled={generating}>
				{generating ? 'Generating…' : 'Generate New Link'}
			</button>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.8);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
	}

	.modal {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 0.875rem;
		padding: 1.75rem;
		width: 420px;
		max-width: calc(100vw - 2rem);
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.modal-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
	}

	.modal-title {
		font-size: 1.0625rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.375rem;
	}

	.channel-ref {
		font-size: 0.9375rem;
		font-weight: 500;
		color: var(--color-text-muted);
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 0.25rem;
		border-radius: 0.25rem;
		display: flex;
		align-items: center;
		flex-shrink: 0;
	}
	.close-btn:hover { color: var(--color-text-primary); }

	.section-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		margin: 0;
	}

	/* Link row */
	.link-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		padding: 0.5rem 0.5rem 0.5rem 0.875rem;
	}

	.link-input {
		flex: 1;
		font-size: 0.8125rem;
		color: var(--color-text-primary);
		font-family: monospace;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}
	.link-input.loading { color: var(--color-text-muted); font-style: italic; }
	.link-input.error-text { color: #fca5a5; font-family: inherit; font-style: italic; }

	.copy-btn {
		display: flex;
		align-items: center;
		gap: 0.3125rem;
		background: var(--color-brand);
		border: none;
		border-radius: 0.375rem;
		padding: 0.4375rem 0.875rem;
		font-size: 0.8125rem;
		font-weight: 600;
		color: white;
		cursor: pointer;
		flex-shrink: 0;
		font-family: inherit;
		transition: opacity 0.15s;
	}
	.copy-btn:hover { opacity: 0.9; }
	.copy-btn:disabled { opacity: 0.4; cursor: default; }
	.copy-btn.copied { background: #16a34a; }

	/* Options */
	.options-grid {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		padding: 0.875rem;
	}

	.option-group {
		display: flex;
		flex-direction: column;
		gap: 0.4375rem;
	}

	.option-label {
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-muted);
		margin: 0;
	}

	.radio-row {
		display: flex;
		flex-direction: column;
		gap: 0.3125rem;
	}

	.radio-opt {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: var(--color-text-primary);
		cursor: pointer;
	}

	.radio-opt input[type="radio"] {
		accent-color: var(--color-brand);
		width: 14px;
		height: 14px;
		cursor: pointer;
	}

	.inline-select {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 0.25rem;
		padding: 0.125rem 0.375rem;
		font-size: 0.8125rem;
		color: var(--color-text-primary);
		cursor: pointer;
		font-family: inherit;
	}
	.inline-select:disabled { opacity: 0.4; }

	.uses-input {
		width: 60px;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 0.25rem;
		padding: 0.125rem 0.375rem;
		font-size: 0.8125rem;
		color: var(--color-text-primary);
		text-align: center;
		font-family: inherit;
	}
	.uses-input:disabled { opacity: 0.4; }

	.btn-generate {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		padding: 0.5625rem 1rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--color-text-muted);
		cursor: pointer;
		font-family: inherit;
		transition: color 0.15s, border-color 0.15s;
		align-self: flex-start;
	}
	.btn-generate:hover { color: var(--color-text-primary); border-color: var(--color-text-muted); }
	.btn-generate:disabled { opacity: 0.4; cursor: default; }
</style>
