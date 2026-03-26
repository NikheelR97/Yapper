<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { api } from '$api/client.js';
	import { authStore } from '$stores/auth.js';
	import { toast } from '$stores/toast.js';

	$: serverId = $page.params.id;

	interface ServerData {
		id: string;
		name: string;
		description: string | null;
		is_public: boolean;
		icon_url: string | null;
		owner_id: string;
	}

	let server: ServerData | null = null;
	let loading = true;

	// Edit fields
	let name = '';
	let description = '';
	let isPublic = false;
	let saving = false;

	// Channel creation
	let newChannelName = '';
	let creatingChannel = false;

	// Invite
	let inviteCode = '';
	let inviteLoading = false;

	// Danger
	let confirmLeave = false;
	let confirmDelete = false;

	$: isOwner = server?.owner_id === $authStore.user?.id;

	async function load() {
		try {
			const res = await api.get<ServerData>(`/api/v2/servers/${serverId}`);
			server = res;
			name = res.name;
			description = res.description ?? '';
			isPublic = res.is_public;
			} catch {
			toast.error('Failed to load server');
		} finally {
			loading = false;
		}
	}

	async function save() {
		if (!name.trim()) return;
		saving = true;
		try {
			await api.patch(`/api/v2/servers/${serverId}`, {
				name: name.trim(),
				description: description.trim() || null,
				is_public: isPublic
			});
			toast.success('Saved!');
			if (server) server.name = name.trim();
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to save');
		} finally {
			saving = false;
		}
	}

	async function createChannel() {
		const n = newChannelName.trim().toLowerCase().replace(/\s+/g, '-');
		if (!n) return;
		creatingChannel = true;
		try {
			await api.post(`/api/v2/servers/${serverId}/channels`, { name: n });
			toast.success(`#${n} created`);
			newChannelName = '';
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to create channel');
		} finally {
			creatingChannel = false;
		}
	}

	async function generateInvite() {
		inviteLoading = true;
		try {
			const res = await api.post<{ code: string }>(`/api/v2/servers/${serverId}/invite`, {});
			inviteCode = res.code;
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to generate invite');
		} finally {
			inviteLoading = false;
		}
	}

	function copyInvite() {
		const link = `${window.location.origin}/join/${inviteCode}`;
		navigator.clipboard.writeText(link);
		toast.success('Invite link copied!');
	}

	async function leaveServer() {
		try {
			await api.delete(`/api/v2/servers/${serverId}/leave`);
			toast.success('Left server');
			await goto('/explore');
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to leave');
		}
	}

	onMount(load);
</script>

<svelte:head>
	<title>Server Settings — Yapper</title>
</svelte:head>

<div class="settings-shell">
	{#if loading}
		<p class="muted">Loading…</p>
	{:else if server}
		<div class="back-row">
			<button class="back-btn" on:click={() => history.back()}>← Back</button>
		</div>

		<h1>{server.name} — Settings</h1>

		{#if isOwner}
			<!-- ── General ── -->
			<section>
				<h2>General</h2>
				<label class="field">
					<span>Server name</span>
					<input bind:value={name} maxlength="100" placeholder="My Server" />
				</label>
				<label class="field">
					<span>Description</span>
					<textarea bind:value={description} maxlength="500" rows="3" placeholder="What's this server about?"></textarea>
				</label>
				<label class="field-row">
					<input type="checkbox" bind:checked={isPublic} />
					<span>Public server (visible in Explore)</span>
				</label>
				<button class="btn-primary" on:click={save} disabled={saving || !name.trim()}>
					{saving ? 'Saving…' : 'Save changes'}
				</button>
			</section>

			<!-- ── Channels ── -->
			<section>
				<h2>Create Channel</h2>
				<div class="row-input">
					<span class="prefix">#</span>
					<input
						bind:value={newChannelName}
						placeholder="new-channel"
						maxlength="50"
						on:keydown={(e) => e.key === 'Enter' && createChannel()}
					/>
					<button class="btn-primary" on:click={createChannel} disabled={creatingChannel || !newChannelName.trim()}>
						{creatingChannel ? '…' : 'Create'}
					</button>
				</div>
			</section>
		{/if}

		<!-- ── Invite ── -->
		<section>
			<h2>Invite Link</h2>
			{#if inviteCode}
				<div class="invite-row">
					<code class="invite-code">{window.location.origin}/join/{inviteCode}</code>
					<button class="btn-secondary" on:click={copyInvite}>Copy</button>
				</div>
			{/if}
			<button class="btn-secondary" on:click={generateInvite} disabled={inviteLoading}>
				{inviteLoading ? 'Generating…' : inviteCode ? 'New invite' : 'Generate invite'}
			</button>
		</section>

		<!-- ── Danger Zone ── -->
		<section class="danger-zone">
			<h2>Danger Zone</h2>
			{#if !isOwner}
				{#if confirmLeave}
					<p class="danger-text">Are you sure you want to leave?</p>
					<div class="danger-row">
						<button class="btn-danger" on:click={leaveServer}>Yes, leave</button>
						<button class="btn-secondary" on:click={() => (confirmLeave = false)}>Cancel</button>
					</div>
				{:else}
					<button class="btn-danger-outline" on:click={() => (confirmLeave = true)}>Leave server</button>
				{/if}
			{:else}
				<p class="muted">As owner, transfer ownership before leaving (coming soon).</p>
			{/if}
		</section>
	{/if}
</div>

<style>
	.settings-shell {
		padding: 2rem;
		max-width: 560px;
		margin: 0 auto;
		overflow-y: auto;
		height: 100%;
	}

	.back-btn {
		background: none;
		border: none;
		color: var(--color-text-secondary, #9ca3af);
		cursor: pointer;
		font-size: 0.875rem;
		padding: 0;
		margin-bottom: 1rem;
	}

	.back-btn:hover {
		color: var(--color-text-primary);
	}

	h1 {
		font-size: 1.4rem;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0 0 2rem;
	}

	section {
		margin-bottom: 2.5rem;
		padding-bottom: 2.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	h2 {
		font-size: 0.8rem;
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--color-text-secondary, #9ca3af);
		margin: 0 0 1rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		margin-bottom: 1rem;
	}

	.field span {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.field input,
	.field textarea {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: var(--color-text-primary);
		padding: 0.6rem 0.8rem;
		font-size: 0.9rem;
		font-family: inherit;
		resize: vertical;
	}

	.field input:focus,
	.field textarea:focus {
		outline: none;
		border-color: var(--color-brand, #7c3aed);
	}

	.field-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1rem;
		cursor: pointer;
		font-size: 0.9rem;
		color: var(--color-text-primary);
	}

	.row-input {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.prefix {
		color: var(--color-text-secondary, #9ca3af);
		font-weight: 600;
	}

	.row-input input {
		flex: 1;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: var(--color-text-primary);
		padding: 0.6rem 0.8rem;
		font-size: 0.9rem;
	}

	.row-input input:focus {
		outline: none;
		border-color: var(--color-brand, #7c3aed);
	}

	.invite-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 8px;
		padding: 0.5rem 0.75rem;
	}

	.invite-code {
		flex: 1;
		font-size: 0.8rem;
		color: var(--color-text-secondary, #9ca3af);
		word-break: break-all;
		font-family: monospace;
	}

	.btn-primary {
		background: var(--color-brand, #7c3aed);
		color: white;
		border: none;
		border-radius: 8px;
		padding: 0.6rem 1.2rem;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-secondary {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-text-primary);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 8px;
		padding: 0.6rem 1.2rem;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-secondary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.danger-zone h2 {
		color: #f87171;
	}

	.danger-text {
		font-size: 0.875rem;
		color: var(--color-text-secondary);
		margin: 0 0 0.75rem;
	}

	.danger-row {
		display: flex;
		gap: 0.5rem;
	}

	.btn-danger {
		background: #dc2626;
		color: white;
		border: none;
		border-radius: 8px;
		padding: 0.6rem 1.2rem;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-danger-outline {
		background: transparent;
		color: #f87171;
		border: 1px solid #f87171;
		border-radius: 8px;
		padding: 0.6rem 1.2rem;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
	}

	.muted {
		font-size: 0.875rem;
		color: var(--color-text-muted, #6b7280);
	}
</style>
