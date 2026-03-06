<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		serversStore,
		fetchServers,
		fetchChannels,
		createServer,
		createInvite,
		joinByInvite,
	} from '$stores/servers.js';
	import type { Channel } from '$stores/servers.js';

	export let activeServerId: string;
	export let activeChannelId: string;

	let channels: Channel[] = [];
	let loadingChannels = false;

	// Create server modal state
	let showCreateModal = false;
	let newServerName = '';
	let creating = false;
	let createServerInput: HTMLInputElement | null = null;

	// Invite state
	let inviteCode = '';
	let showInvitePanel = false;

	// Join by invite state
	let joinCode = '';
	let joining = false;
	let joinError = '';

	onMount(async () => {
		if (!$serversStore.servers.length) {
			await fetchServers();
		}
		if (activeServerId) {
			await loadChannels(activeServerId);
		}
	});

	// Reload channels when the active server changes
	$: if (activeServerId) {
		loadChannels(activeServerId);
	}

	$: if (showCreateModal) {
		void focusCreateServerInput();
	}

	async function focusCreateServerInput() {
		await tick();
		createServerInput?.focus();
	}

	async function loadChannels(serverId: string) {
		loadingChannels = true;
		try {
			channels = await fetchChannels(serverId);
		} catch {
			channels = [];
		} finally {
			loadingChannels = false;
		}
	}

	function serverInitial(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	function navigateToChannel(serverId: string, channelId: string) {
		goto(`/servers/${serverId}/channels/${channelId}`);
	}

	async function handleCreateServer() {
		if (!newServerName.trim() || creating) return;
		creating = true;
		try {
			const server = await createServer(newServerName.trim());
			newServerName = '';
			showCreateModal = false;
			// Navigate to the new server (no channels yet)
			goto(`/servers/${server.id}/channels`);
		} finally {
			creating = false;
		}
	}

	async function handleGetInvite() {
		if (!activeServerId) return;
		inviteCode = await createInvite(activeServerId);
		showInvitePanel = true;
	}

	function copyInvite() {
		navigator.clipboard.writeText(inviteCode).catch(() => {});
	}

	async function handleJoinByInvite() {
		if (!joinCode.trim() || joining) return;
		joining = true;
		joinError = '';
		try {
			await joinByInvite(joinCode.trim());
			joinCode = '';
		} catch {
			joinError = 'Invalid or expired invite code.';
		} finally {
			joining = false;
		}
	}
</script>

<aside class="sidebar">
	<!-- Server list strip -->
	<nav class="server-strip" aria-label="Servers">
		{#each $serversStore.servers as server (server.id)}
			<button
				class="server-btn"
				class:active={server.id === activeServerId}
				on:click={() => {
					if (server.id !== activeServerId) {
						loadChannels(server.id);
						// Navigate to server root — channel page will pick a channel
						goto(`/servers/${server.id}/channels`);
					}
				}}
				title={server.name}
				aria-label={server.name}
				aria-current={server.id === activeServerId ? 'page' : undefined}
			>
				{#if server.iconUrl}
					<img src={server.iconUrl} alt={server.name} width="36" height="36" />
				{:else}
					<span aria-hidden="true">{serverInitial(server.name)}</span>
				{/if}
			</button>
		{/each}

		<!-- Add / join server -->
		<button
			class="server-btn add-btn"
			on:click={() => (showCreateModal = true)}
			title="Create or join a server"
			aria-label="Create or join a server"
		>
			<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<line x1="12" y1="5" x2="12" y2="19"></line>
				<line x1="5" y1="12" x2="19" y2="12"></line>
			</svg>
		</button>
	</nav>

	<!-- Channel list panel -->
	<div class="channel-panel">
		{#if activeServerId}
			{@const activeServer = $serversStore.servers.find((s) => s.id === activeServerId)}
			<header class="channel-panel-header">
				<span class="server-name">{activeServer?.name ?? '…'}</span>
				{#if activeServer}
					<button
						class="invite-btn"
						on:click={handleGetInvite}
						title="Create invite link"
						aria-label="Create invite link"
					>
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
							<path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
						</svg>
					</button>
				{/if}
			</header>

			{#if showInvitePanel}
				<div class="invite-panel">
					<span class="invite-code">{inviteCode}</span>
					<button class="copy-btn" on:click={copyInvite}>Copy</button>
					<button class="close-btn" on:click={() => (showInvitePanel = false)} aria-label="Close">✕</button>
				</div>
			{/if}

			{#if loadingChannels}
				<div class="channel-loading">Loading…</div>
			{:else if channels.length === 0}
				<div class="channel-empty">No channels yet.</div>
			{:else}
				<ul class="channel-list" role="list">
					{#each channels as ch (ch.id)}
						<li>
							<button
								class="channel-btn"
								class:active={ch.id === activeChannelId}
								on:click={() => navigateToChannel(activeServerId, ch.id)}
								aria-current={ch.id === activeChannelId ? 'page' : undefined}
							>
								<span class="channel-hash" aria-hidden="true">#</span>
								{ch.name}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		{:else}
			<div class="channel-empty">Select a server.</div>
		{/if}

		<!-- Join by invite -->
		<div class="join-section">
			<form on:submit|preventDefault={handleJoinByInvite} class="join-form">
				<input
					bind:value={joinCode}
					placeholder="Invite code…"
					class="join-input"
					disabled={joining}
					aria-label="Enter invite code"
				/>
				<button type="submit" class="join-submit" disabled={joining || !joinCode.trim()}>Join</button>
			</form>
			{#if joinError}
				<p class="join-error">{joinError}</p>
			{/if}
		</div>
	</div>
</aside>

<!-- Create Server Modal -->
{#if showCreateModal}
	<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
	<div class="modal-backdrop" on:click={() => (showCreateModal = false)}>
		<div
			class="modal"
			role="dialog"
			aria-modal="true"
			aria-label="Create server"
			tabindex="-1"
			on:click|stopPropagation
		>
			<h2 class="modal-title">Create a Server</h2>
			<form on:submit|preventDefault={handleCreateServer}>
				<input
					bind:this={createServerInput}
					bind:value={newServerName}
					placeholder="Server name"
					class="modal-input"
					maxlength="100"
					required
				/>
				<div class="modal-actions">
					<button type="button" class="modal-cancel" on:click={() => (showCreateModal = false)}>
						Cancel
					</button>
					<button type="submit" class="modal-submit" disabled={creating || !newServerName.trim()}>
						{creating ? 'Creating…' : 'Create'}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<style>
	.sidebar {
		display: flex;
		flex-shrink: 0;
		height: 100%;
		background: var(--color-bg-base);
		border-right: 1px solid var(--color-border);
	}

	/* ── Server strip ── */
	.server-strip {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.375rem;
		width: 56px;
		background: var(--color-bg-sunken, #0d0d14);
		border-right: 1px solid var(--color-border);
		overflow-y: auto;
		scrollbar-width: none;
	}
	.server-strip::-webkit-scrollbar { display: none; }

	.server-btn {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		border: none;
		background: var(--color-bg-surface);
		color: var(--color-text-primary);
		cursor: pointer;
		font-size: 0.6875rem;
		font-weight: 700;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: border-radius 0.15s, background 0.15s;
		overflow: hidden;
		flex-shrink: 0;
	}
	.server-btn:hover,
	.server-btn.active {
		border-radius: 0.625rem;
		background: var(--color-brand);
		color: white;
	}
	.server-btn img {
		width: 36px;
		height: 36px;
		object-fit: cover;
	}
	.add-btn {
		background: var(--color-bg-elevated);
		color: var(--color-brand);
	}
	.add-btn:hover {
		background: var(--color-brand);
		color: white;
	}

	/* ── Channel panel ── */
	.channel-panel {
		display: flex;
		flex-direction: column;
		width: 184px;
		overflow: hidden;
	}

	.channel-panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.875rem 0.75rem 0.625rem;
		border-bottom: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.server-name {
		font-size: 0.875rem;
		font-weight: 700;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.invite-btn {
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
	.invite-btn:hover { color: var(--color-text-primary); }

	.invite-panel {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.75rem;
		background: var(--color-bg-surface);
		border-bottom: 1px solid var(--color-border);
		font-size: 0.75rem;
		flex-shrink: 0;
	}
	.invite-code {
		flex: 1;
		font-family: monospace;
		color: var(--color-text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.copy-btn, .close-btn {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: 0.25rem;
		padding: 0.125rem 0.375rem;
		font-size: 0.6875rem;
		cursor: pointer;
		color: var(--color-text-muted);
		flex-shrink: 0;
	}
	.copy-btn:hover, .close-btn:hover { color: var(--color-text-primary); }

	.channel-loading,
	.channel-empty {
		padding: 1rem 0.75rem;
		font-size: 0.8125rem;
		color: var(--color-text-muted);
	}

	.channel-list {
		list-style: none;
		margin: 0;
		padding: 0.375rem 0;
		overflow-y: auto;
		flex: 1;
	}

	.channel-btn {
		width: 100%;
		background: none;
		border: none;
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.3125rem 0.625rem 0.3125rem 0.75rem;
		border-radius: 0.25rem;
		margin: 0 0.375rem;
		width: calc(100% - 0.75rem);
		font-size: 0.875rem;
		color: var(--color-text-muted);
		cursor: pointer;
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		transition: background 0.1s, color 0.1s;
	}
	.channel-btn:hover {
		background: var(--color-bg-surface);
		color: var(--color-text-primary);
	}
	.channel-btn.active {
		background: var(--color-bg-elevated);
		color: var(--color-text-primary);
		font-weight: 600;
	}
	.channel-hash {
		color: var(--color-text-muted);
		font-size: 1rem;
		flex-shrink: 0;
	}

	/* ── Join section ── */
	.join-section {
		padding: 0.625rem 0.75rem;
		border-top: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.join-form {
		display: flex;
		gap: 0.375rem;
	}

	.join-input {
		flex: 1;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 0.375rem;
		padding: 0.3125rem 0.5rem;
		font-size: 0.75rem;
		color: var(--color-text-primary);
		min-width: 0;
	}
	.join-input::placeholder { color: var(--color-text-muted); }

	.join-submit {
		background: var(--color-brand);
		border: none;
		border-radius: 0.375rem;
		padding: 0.3125rem 0.5rem;
		font-size: 0.75rem;
		color: white;
		cursor: pointer;
		font-weight: 600;
		flex-shrink: 0;
	}
	.join-submit:disabled { opacity: 0.5; cursor: default; }

	.join-error {
		margin: 0.25rem 0 0;
		font-size: 0.6875rem;
		color: #fca5a5;
	}

	/* ── Create Server Modal ── */
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.modal {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 0.75rem;
		padding: 1.5rem;
		width: 320px;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.modal-title {
		font-size: 1.125rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
	}

	.modal-input {
		width: 100%;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		padding: 0.625rem 0.75rem;
		font-size: 0.9375rem;
		color: var(--color-text-primary);
		box-sizing: border-box;
	}
	.modal-input::placeholder { color: var(--color-text-muted); }

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	.modal-cancel {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: 0.375rem;
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.modal-cancel:hover { color: var(--color-text-primary); }

	.modal-submit {
		background: var(--color-brand);
		border: none;
		border-radius: 0.375rem;
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		color: white;
		font-weight: 600;
		cursor: pointer;
	}
	.modal-submit:disabled { opacity: 0.5; cursor: default; }
</style>
