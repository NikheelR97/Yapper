<script lang="ts">
	import { onMount } from 'svelte';
	import {
		updateAvailable,
		checkingForUpdate,
		updateError,
		checkForUpdate,
		installUpdate,
	} from '$lib/desktop/updater.js';

	let appVersion = 'unknown';
	let installing = false;
	let checked = false;

	onMount(async () => {
		try {
			const { getVersion } = await import('@tauri-apps/api/app');
			appVersion = await getVersion();
		} catch {
			appVersion = import.meta.env.VITE_APP_VERSION ?? 'unknown';
		}
	});

	async function handleCheck() {
		checked = false;
		await checkForUpdate();
		checked = true;
	}

	async function handleInstall() {
		installing = true;
		await installUpdate();
		installing = false;
	}
</script>

<div class="about-section">
	<div class="app-identity">
		<div class="app-icon" aria-hidden="true">
			<svg width="48" height="48" viewBox="0 0 48 48" fill="none">
				<rect width="48" height="48" rx="12" fill="url(#grad)" />
				<path d="M16 32 C16 20 24 16 24 16 C24 16 32 20 32 32" stroke="white" stroke-width="2.5" stroke-linecap="round" fill="none"/>
				<circle cx="24" cy="28" r="4" fill="white" />
				<defs>
					<linearGradient id="grad" x1="0" y1="0" x2="48" y2="48">
						<stop offset="0%" stop-color="#7c3aed"/>
						<stop offset="100%" stop-color="#4f1d96"/>
					</linearGradient>
				</defs>
			</svg>
		</div>
		<div>
			<h2 class="app-name">Yapper Desktop</h2>
			<p class="app-version">Version {appVersion}</p>
			<p class="app-copy">© {new Date().getFullYear()} Yapper HQ</p>
		</div>
	</div>

	<div class="update-card">
		<button
			class="check-btn"
			on:click={handleCheck}
			disabled={$checkingForUpdate || installing}
		>
			{#if $checkingForUpdate}
				<span class="spinner" aria-hidden="true"></span>
				Checking…
			{:else}
				Check for Updates
			{/if}
		</button>

		{#if $updateError}
			<p class="status error">{$updateError}</p>
		{:else if checked && $updateAvailable}
			<div class="update-ready">
				<p class="status success">
					🚀 Version <strong>{$updateAvailable}</strong> is available
				</p>
				<button
					class="install-btn"
					on:click={handleInstall}
					disabled={installing}
				>
					{#if installing}
						<span class="spinner" aria-hidden="true"></span>
						Installing…
					{:else}
						Install & Restart
					{/if}
				</button>
			</div>
		{:else if $updateAvailable}
			<!-- Update was detected by background check on launch -->
			<div class="update-ready">
				<p class="status success">
					🚀 Version <strong>{$updateAvailable}</strong> is available
				</p>
				<button
					class="install-btn"
					on:click={handleInstall}
					disabled={installing}
				>
					{#if installing}
						<span class="spinner" aria-hidden="true"></span>
						Installing…
					{:else}
						Install & Restart
					{/if}
				</button>
			</div>
		{:else if checked}
			<p class="status up-to-date">✓ You're up to date</p>
		{/if}
	</div>
</div>

<style>
	.about-section {
		display: flex;
		flex-direction: column;
		gap: 2rem;
		max-width: 480px;
	}

	.app-identity {
		display: flex;
		align-items: center;
		gap: 1.25rem;
	}

	.app-icon {
		flex-shrink: 0;
	}

	.app-name {
		font-size: 1.375rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0 0 0.125rem;
	}

	.app-version {
		font-size: 0.875rem;
		color: var(--color-text-secondary);
		margin: 0 0 0.125rem;
	}

	.app-copy {
		font-size: 0.8125rem;
		color: var(--color-text-muted);
		margin: 0;
	}

	.update-card {
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.check-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		background: var(--color-brand);
		color: white;
		border: none;
		border-radius: var(--radius-md);
		font-size: 0.9375rem;
		font-weight: 600;
		padding: 0.625rem 1.25rem;
		cursor: pointer;
		align-self: flex-start;
		transition: background var(--transition-fast), opacity var(--transition-fast);
	}

	.check-btn:hover:not(:disabled) {
		background: var(--color-brand-dark);
	}

	.check-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.status {
		font-size: 0.875rem;
		margin: 0;
	}

	.status.up-to-date {
		color: #22c55e;
	}

	.status.success {
		color: var(--color-text-primary);
	}

	.status.error {
		color: #fca5a5;
	}

	.update-ready {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.install-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		background: #22c55e;
		color: white;
		border: none;
		border-radius: var(--radius-md);
		font-size: 0.9375rem;
		font-weight: 600;
		padding: 0.625rem 1.25rem;
		cursor: pointer;
		align-self: flex-start;
		transition: background var(--transition-fast), opacity var(--transition-fast);
	}

	.install-btn:hover:not(:disabled) {
		background: #16a34a;
	}

	.install-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.spinner {
		display: inline-block;
		width: 14px;
		height: 14px;
		border: 2px solid rgba(255, 255, 255, 0.4);
		border-top-color: white;
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
