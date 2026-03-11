<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		serversStore,
		fetchServers,
		fetchChannels,
		joinByInvite,
	} from '$stores/servers.js';
	import type { Channel } from '$stores/servers.js';
	import CreateServerModal from '$lib/components/chat/CreateServerModal.svelte';
	import InviteModal from '$lib/components/chat/InviteModal.svelte';

	export let activeServerId: string;
	export let activeChannelId: string;

	let channels: Channel[] = [];
	let loadingChannels = false;

	// Modal state
	let showCreateModal = false;
	let showInviteModal = false;

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

	$: activeServer = $serversStore.servers.find((s) => s.id === activeServerId);
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
			<header class="channel-panel-header">
				<span class="server-name">{activeServer?.name ?? '…'}</span>
				{#if activeServer}
					<button
						class="invite-btn"
						on:click={() => (showInviteModal = true)}
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

<CreateServerModal open={showCreateModal} on:close={() => (showCreateModal = false)} />

<InviteModal
	open={showInviteModal}
	serverId={activeServerId ?? ''}
	serverName={activeServer?.name ?? ''}
	on:close={() => (showInviteModal = false)}
/>

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

</style>
