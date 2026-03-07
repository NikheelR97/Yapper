<script lang="ts">
	import { onMount, tick } from 'svelte';

	export let mode: 'setup' | 'unlock';
	export let busy = false;
	export let error: string | null = null;
	export let onSubmit: (passphrase: string) => Promise<void>;

	let passphrase = '';
	let confirmPassphrase = '';
	let localError: string | null = null;
	let passphraseInput: HTMLInputElement | null = null;

	$: helperText =
		mode === 'setup'
			? 'Create a device-local passphrase. Yapper uses it to unlock your encrypted desktop vault on cold start.'
			: 'Enter your device-local passphrase to unlock the encrypted desktop vault for this session.';

	async function submit(): Promise<void> {
		localError = null;
		if (!passphrase.trim()) {
			localError = 'Passphrase is required.';
			return;
		}
		if (mode === 'setup' && passphrase !== confirmPassphrase) {
			localError = 'Passphrases do not match.';
			return;
		}

		await onSubmit(passphrase);
	}

	onMount(async () => {
		await tick();
		passphraseInput?.focus();
	});
</script>

<div class="gate-shell">
	<div class="gate-card">
		<p class="eyebrow">Secure Desktop Vault</p>
		<h1>{mode === 'setup' ? 'Set your vault passphrase' : 'Unlock your vault'}</h1>
		<p class="body">{helperText}</p>

		<div class="form-grid">
			<label class="field">
				<span>Passphrase</span>
				<input
					bind:this={passphraseInput}
					bind:value={passphrase}
					type="password"
					autocomplete={mode === 'setup' ? 'new-password' : 'current-password'}
					placeholder="Enter passphrase"
					disabled={busy}
					on:keydown={(event) => event.key === 'Enter' && void submit()}
				/>
			</label>

			{#if mode === 'setup'}
				<label class="field">
					<span>Confirm passphrase</span>
					<input
						bind:value={confirmPassphrase}
						type="password"
						autocomplete="new-password"
						placeholder="Repeat passphrase"
						disabled={busy}
						on:keydown={(event) => event.key === 'Enter' && void submit()}
					/>
				</label>
			{/if}
		</div>

		{#if localError || error}
			<p class="error">{localError ?? error}</p>
		{/if}

		<button class="submit-btn" type="button" on:click={() => void submit()} disabled={busy}>
			{#if busy}
				{mode === 'setup' ? 'Creating vault...' : 'Unlocking...'}
			{:else}
				{mode === 'setup' ? 'Create vault' : 'Unlock vault'}
			{/if}
		</button>
	</div>
</div>

<style>
	.gate-shell {
		display: grid;
		place-items: center;
		flex: 1;
		padding: 2rem;
		background:
			radial-gradient(circle at top, rgba(34, 197, 94, 0.12), transparent 42%),
			linear-gradient(180deg, rgba(13, 13, 18, 0.98), rgba(8, 8, 12, 0.98));
	}

	.gate-card {
		width: min(100%, 560px);
		border: 1px solid rgba(34, 197, 94, 0.24);
		border-radius: 20px;
		padding: 2rem;
		background: rgba(17, 17, 24, 0.94);
		box-shadow: 0 32px 80px rgba(0, 0, 0, 0.35);
	}

	.eyebrow {
		margin: 0 0 0.5rem;
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: #22c55e;
	}

	h1 {
		margin: 0 0 0.75rem;
		font-size: 1.8rem;
		font-weight: 800;
		color: var(--color-text-primary);
	}

	.body {
		margin: 0 0 1.5rem;
		color: var(--color-text-secondary);
		line-height: 1.6;
	}

	.form-grid {
		display: grid;
		gap: 0.9rem;
	}

	.field {
		display: grid;
		gap: 0.45rem;
	}

	.field span {
		font-size: 0.86rem;
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.field input {
		width: 100%;
		border-radius: 12px;
		border: 1px solid rgba(255, 255, 255, 0.08);
		background: rgba(8, 8, 12, 0.8);
		color: var(--color-text-primary);
		padding: 0.8rem 0.9rem;
		font-size: 0.95rem;
	}

	.field input:disabled {
		opacity: 0.7;
		cursor: not-allowed;
	}

	.error {
		margin: 1rem 0 0;
		color: #f87171;
		font-size: 0.86rem;
	}

	.submit-btn {
		margin-top: 1.3rem;
		border: none;
		border-radius: 999px;
		padding: 0.8rem 1.1rem;
		font-size: 0.95rem;
		font-weight: 700;
		color: #04110a;
		background: linear-gradient(135deg, #22c55e, #84cc16);
		cursor: pointer;
	}

	.submit-btn:disabled {
		opacity: 0.7;
		cursor: not-allowed;
	}
</style>
