<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';

	interface FriendRequest {
		id: string;
		username: string;
		displayName: string | null;
		avatarUrl: string | null;
	}

	let requests: FriendRequest[] = [];
	let addUsername = '';
	let adding = false;
	let loading = true;

	async function loadRequests() {
		try {
			const res = await api.get<{ requests: FriendRequest[] }>('/api/v1/users/me/friend-requests');
			requests = res.requests;
		} catch {
			// non-fatal
		} finally {
			loading = false;
		}
	}

	async function sendRequest() {
		if (!addUsername.trim()) return;
		adding = true;
		try {
			await api.post(`/api/v1/users/by/${encodeURIComponent(addUsername.trim())}/friend-request`, {});
			toast.success('Friend request sent!');
			addUsername = '';
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to send friend request');
		} finally {
			adding = false;
		}
	}

	async function accept(username: string) {
		try {
			await api.put(`/api/v1/users/by/${encodeURIComponent(username)}/friend-request`, {});
			toast.success('Friend request accepted!');
			requests = requests.filter((r) => r.username !== username);
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to accept');
		}
	}

	async function reject(username: string) {
		try {
			await api.delete(`/api/v1/users/by/${encodeURIComponent(username)}/friend-request`);
			requests = requests.filter((r) => r.username !== username);
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to reject');
		}
	}

	onMount(loadRequests);
</script>

<svelte:head>
	<title>Direct Messages — Yapper</title>
</svelte:head>

<div class="dm-index">
	<div class="panel">
		<h2>Add Friend</h2>
		<form class="add-row" on:submit|preventDefault={sendRequest}>
			<input
				bind:value={addUsername}
				placeholder="Enter username"
				disabled={adding}
				autocomplete="off"
			/>
			<button type="submit" disabled={adding || !addUsername.trim()}>
				{adding ? 'Sending…' : 'Send Request'}
			</button>
		</form>

		<h2 class="section-title">
			Pending Requests
			{#if requests.length > 0}<span class="badge">{requests.length}</span>{/if}
		</h2>

		{#if loading}
			<p class="muted">Loading…</p>
		{:else if requests.length === 0}
			<p class="muted">No pending friend requests.</p>
		{:else}
			<ul class="request-list">
				{#each requests as req (req.id)}
					<li class="request-row">
						<button class="avatar-btn" on:click={() => goto(`/profile/${req.username}`)}>
							{#if req.avatarUrl}
								<img src={req.avatarUrl} alt={req.displayName ?? req.username} class="avatar" />
							{:else}
								<div class="avatar avatar-placeholder">
									{(req.displayName ?? req.username)[0].toUpperCase()}
								</div>
							{/if}
						</button>
						<div class="req-info">
							<span class="req-name">{req.displayName ?? req.username}</span>
							<span class="req-username">@{req.username}</span>
						</div>
						<div class="req-actions">
							<button class="btn-accept" on:click={() => accept(req.username)}>Accept</button>
							<button class="btn-reject" on:click={() => reject(req.username)}>Decline</button>
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

<style>
	.dm-index {
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding: 2rem 1rem;
		height: 100%;
		background: var(--color-bg-base);
		overflow-y: auto;
	}

	.panel {
		width: min(100%, 520px);
	}

	h2 {
		font-size: 1rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0 0 0.75rem;
	}

	.section-title {
		margin-top: 2rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.badge {
		background: var(--color-brand, #7c3aed);
		color: white;
		font-size: 0.75rem;
		font-weight: 700;
		border-radius: 999px;
		padding: 0 0.45rem;
		line-height: 1.6;
	}

	.add-row {
		display: flex;
		gap: 0.5rem;
	}

	.add-row input {
		flex: 1;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: var(--color-text-primary);
		padding: 0.6rem 0.85rem;
		font-size: 0.9rem;
	}

	.add-row input:focus {
		outline: none;
		border-color: var(--color-brand, #7c3aed);
	}

	.add-row button {
		background: var(--color-brand, #7c3aed);
		color: white;
		border: none;
		border-radius: 8px;
		padding: 0.6rem 1.1rem;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		white-space: nowrap;
	}

	.add-row button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.muted {
		font-size: 0.875rem;
		color: var(--color-text-muted, #6b7280);
	}

	.request-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.request-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 10px;
		padding: 0.65rem 0.85rem;
	}

	.avatar-btn {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		flex-shrink: 0;
	}

	.avatar {
		width: 40px;
		height: 40px;
		border-radius: 50%;
		object-fit: cover;
	}

	.avatar-placeholder {
		width: 40px;
		height: 40px;
		border-radius: 50%;
		background: linear-gradient(135deg, #7c3aed, #2e1065);
		color: white;
		font-size: 16px;
		font-weight: 700;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.req-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
	}

	.req-name {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.req-username {
		font-size: 0.78rem;
		color: var(--color-text-muted, #6b7280);
	}

	.req-actions {
		display: flex;
		gap: 0.4rem;
		flex-shrink: 0;
	}

	.btn-accept {
		background: var(--color-brand, #7c3aed);
		color: white;
		border: none;
		border-radius: 6px;
		padding: 0.35rem 0.75rem;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-reject {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-text-secondary, #9ca3af);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 6px;
		padding: 0.35rem 0.75rem;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-reject:hover {
		background: rgba(239, 68, 68, 0.15);
		color: #fca5a5;
		border-color: rgba(239, 68, 68, 0.3);
	}
</style>
