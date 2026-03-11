<script lang="ts">
	import { tick } from 'svelte';
	import { getOwnFingerprint, fetchPeerFingerprint } from '$signal/index.js';
	import { loadPeerTrustFlags, storePeerVerified } from '$signal/keystore.js';

	export let peerId: string;
	export let peerName: string;
	export let open = false;

	let ownFp: string | null = null;
	let peerFp: string | null = null;
	let loading = true;
	let keyChanged = false;
	let verified = false;
	let dialogBackdrop: HTMLDivElement | null = null;

	$: if (open && peerId) loadFingerprints();
	$: if (open) {
		void focusDialog();
	}

	async function focusDialog() {
		await tick();
		dialogBackdrop?.focus();
	}

	async function loadFingerprints() {
		loading = true;
		({ keyChanged, verified } = await loadPeerTrustFlags(peerId));
		try {
			[ownFp, peerFp] = await Promise.all([
				getOwnFingerprint(),
				fetchPeerFingerprint(peerId),
			]);
		} catch {
			// Non-fatal — show dashes
		} finally {
			loading = false;
		}
	}

	function dismiss() {
		open = false;
	}

	async function markVerified() {
		await storePeerVerified(peerId, true);
		verified = true;
		keyChanged = false;
	}

	async function clearVerification() {
		await storePeerVerified(peerId, false);
		verified = false;
	}

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) dismiss();
	}

	function handleDialogKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			dismiss();
		}
	}
</script>

{#if open}
	<div
		class="backdrop"
		bind:this={dialogBackdrop}
		on:click={handleBackdrop}
		on:keydown={handleDialogKeydown}
		role="dialog"
		aria-modal="true"
		aria-label="Safety numbers"
		tabindex="-1"
	>
		<div class="modal">
			<header class="modal-header">
				<div class="header-left">
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
						<path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
					</svg>
					<span>Safety Numbers</span>
				</div>
				<button class="close-btn" on:click={dismiss} aria-label="Close">✕</button>
			</header>

			{#if keyChanged}
				<div class="key-change-alert">
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
						<line x1="12" y1="9" x2="12" y2="13"></line>
						<line x1="12" y1="17" x2="12.01" y2="17"></line>
					</svg>
					<span><strong>{peerName}'s security code changed.</strong> Verify their new code in person or via a trusted channel.</span>
				</div>
			{/if}

			<div class="modal-body">
				<p class="description">
					Compare these codes with <strong>{peerName}</strong> over a trusted channel to confirm your messages are end-to-end encrypted.
				</p>

				<div class="fingerprint-section">
					<div class="fp-label">Your safety number</div>
					{#if loading}
						<div class="fp-loading">Loading…</div>
					{:else}
						<div class="fp-code" aria-label="Your safety number">{ownFp ?? '— — — — — —'}</div>
					{/if}
				</div>

				<div class="divider" aria-hidden="true"></div>

				<div class="fingerprint-section">
					<div class="fp-label">{peerName}'s safety number</div>
					{#if loading}
						<div class="fp-loading">Loading…</div>
					{:else}
						<div class="fp-code" aria-label="{peerName}'s safety number">{peerFp ?? '— — — — — —'}</div>
					{/if}
				</div>

				<div class="verify-row">
					{#if verified}
						<div class="verified-badge">
							<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
								<polyline points="20 6 9 17 4 12"></polyline>
							</svg>
							Verified
						</div>
						<button class="btn-ghost" on:click={clearVerification}>Clear</button>
					{:else}
						<button class="btn-verify" on:click={markVerified} disabled={loading}>
							Mark as verified
						</button>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
	}

	.modal {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		width: min(400px, 92vw);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.25rem;
		border-bottom: 1px solid var(--color-border);
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 600;
		font-size: 0.9375rem;
		color: var(--color-text-primary);
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 1rem;
		padding: 0.25rem;
		line-height: 1;
	}
	.close-btn:hover { color: var(--color-text-primary); }

	.key-change-alert {
		display: flex;
		align-items: flex-start;
		gap: 0.625rem;
		padding: 0.75rem 1.25rem;
		background: rgba(234, 179, 8, 0.12);
		border-bottom: 1px solid rgba(234, 179, 8, 0.3);
		color: #fbbf24;
		font-size: 0.8125rem;
		line-height: 1.5;
	}
	.key-change-alert svg { flex-shrink: 0; margin-top: 1px; }

	.modal-body {
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.description {
		font-size: 0.8125rem;
		color: var(--color-text-muted);
		line-height: 1.5;
		margin: 0;
	}

	.fingerprint-section {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.fp-label {
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-muted);
	}

	.fp-code {
		font-family: 'Courier New', Courier, monospace;
		font-size: 1.125rem;
		font-weight: 700;
		letter-spacing: 0.12em;
		color: var(--color-text-primary);
		background: var(--color-bg-base);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 0.75rem 1rem;
		text-align: center;
		user-select: all;
	}

	.fp-loading {
		font-size: 0.875rem;
		color: var(--color-text-muted);
		padding: 0.75rem 1rem;
		text-align: center;
	}

	.divider {
		height: 1px;
		background: var(--color-border);
	}

	.verify-row {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.75rem;
		padding-top: 0.25rem;
	}

	.btn-verify {
		background: var(--color-brand);
		color: white;
		border: none;
		border-radius: 8px;
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
	}
	.btn-verify:hover:not(:disabled) { filter: brightness(1.1); }
	.btn-verify:disabled { opacity: 0.5; cursor: default; }

	.verified-badge {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		font-size: 0.8125rem;
		font-weight: 600;
		color: #22c55e;
	}

	.btn-ghost {
		background: none;
		border: 1px solid var(--color-border);
		color: var(--color-text-muted);
		border-radius: 6px;
		padding: 0.25rem 0.625rem;
		font-size: 0.8125rem;
		cursor: pointer;
	}
	.btn-ghost:hover { color: var(--color-text-primary); }
</style>
