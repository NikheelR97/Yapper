<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';
	import { API_URL } from '$lib/env.js';

	interface ConnectedAccount {
		provider: 'discord' | 'google' | 'apple';
		username: string | null;
		connected: boolean;
	}

	let accounts: ConnectedAccount[] = [
		{ provider: 'discord', username: null, connected: false },
		{ provider: 'google', username: null, connected: false },
		{ provider: 'apple', username: null, connected: false },
	];

	let unlinkConfirm: 'discord' | 'google' | 'apple' | null = null;

	const BASE_URL = API_URL;

	onMount(async () => {
		try {
			const me = await api.get<{ connections: Record<string, boolean> }>('/api/v2/users/me');
			const conns = me.connections ?? {};
			accounts = accounts.map((a) => ({ ...a, connected: !!conns[a.provider] }));
		} catch {
			// Non-fatal — UI just shows unconnected state
		}
	});

	function connect(provider: string) {
		if (provider === 'discord') {
			// Profile import flow — requires existing session, links to current account
			window.location.href = `${BASE_URL}/api/v2/discord/import-profile`;
		} else if (provider === 'google') {
			// Google OAuth — will link to existing account if email matches
			window.location.href = `${BASE_URL}/auth/oauth/google`;
		} else {
			toast.error('Apple sign-in coming soon.');
		}
	}

	async function unlink(provider: 'discord' | 'google' | 'apple') {
		try {
			await api.delete(`/api/v2/users/me/connections/${provider}`);
			accounts = accounts.map((a) =>
				a.provider === provider ? { ...a, connected: false, username: null } : a
			);
			toast.success(`${providerNames[provider]} account unlinked.`);
			unlinkConfirm = null;
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : 'Failed to unlink');
		}
	}

	const providerIcons: Record<string, string> = {
		discord: '🎮',
		google: '🔵',
		apple: '🍎',
	};

	const providerNames: Record<string, string> = {
		discord: 'Discord',
		google: 'Google',
		apple: 'Apple',
	};
</script>

<div class="discord-section">
	<h2 class="section-title">Connected Accounts</h2>

	<div class="accounts-card">
		{#each accounts as account}
			<div class="provider-row">
				<div class="provider-left">
					<span class="provider-icon">{providerIcons[account.provider]}</span>
					<div class="provider-info">
						<span class="provider-name">{providerNames[account.provider]}</span>
						{#if account.connected && account.username}
							<span class="provider-username">@{account.username}</span>
						{/if}
					</div>
				</div>

				<div class="provider-right">
					{#if account.connected}
						<span class="connected-label">Connected ✓</span>
						{#if unlinkConfirm === account.provider}
							<span class="unlink-confirm">
								Are you sure?
								<button class="confirm-btn" on:click={() => unlink(account.provider)}>Confirm</button>
								<button class="cancel-btn" on:click={() => (unlinkConfirm = null)}>Cancel</button>
							</span>
						{:else}
							<button class="unlink-btn" on:click={() => (unlinkConfirm = account.provider)}>
								Unlink
							</button>
						{/if}
					{:else}
						<button
							class="connect-btn"
							on:click={() => connect(account.provider)}
							disabled={account.provider === 'apple'}
							title={account.provider === 'apple' ? 'Coming soon' : undefined}
						>
							Connect →
						</button>
					{/if}
				</div>
			</div>
		{/each}
	</div>

	<!-- Discord import note -->
	<div class="import-note">
		<span class="import-icon">💡</span>
		<p>Connect Discord to import your profile name, avatar, and bio. Your data is saved to Yapper's servers, not Discord's CDN.</p>
	</div>
</div>

<style>
	.discord-section {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.section-title {
		font-size: 20px;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0;
	}

	.accounts-card {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 14px;
		overflow: hidden;
	}

	.provider-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 16px 20px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
	}

	.provider-row:last-child {
		border-bottom: none;
	}

	.provider-left {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.provider-icon {
		font-size: 22px;
		width: 32px;
		text-align: center;
	}

	.provider-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.provider-name {
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.provider-username {
		font-size: 12px;
		color: var(--color-text-muted);
	}

	.provider-right {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-shrink: 0;
	}

	.connected-label {
		font-size: 13px;
		color: var(--color-success-text);
		font-weight: 600;
	}

	.unlink-btn {
		background: none;
		border: none;
		color: var(--color-error-text);
		font-size: 13px;
		cursor: pointer;
		padding: 4px 8px;
	}

	.connect-btn {
		background: none;
		border: none;
		color: var(--color-brand-text);
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		padding: 4px 8px;
		transition: color 150ms;
	}

	.connect-btn:hover {
		color: var(--color-brand-text);
	}

	.unlink-confirm {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: var(--color-text-secondary);
	}

	.confirm-btn {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: var(--color-error-text);
		font-size: 12px;
		font-weight: 600;
		padding: 4px 10px;
		border-radius: 6px;
		cursor: pointer;
	}

	.cancel-btn {
		background: none;
		border: none;
		color: var(--color-text-muted);
		font-size: 12px;
		cursor: pointer;
		padding: 4px;
	}

	.import-note {
		display: flex;
		gap: 10px;
		padding: 14px;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 10px;
	}

	.import-icon {
		font-size: 16px;
		flex-shrink: 0;
	}

	.import-note p {
		font-size: 13px;
		color: var(--color-text-secondary);
		margin: 0;
		line-height: 1.5;
	}
</style>
