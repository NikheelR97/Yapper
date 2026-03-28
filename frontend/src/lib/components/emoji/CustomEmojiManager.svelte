<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';
	import EmojiUploader from './EmojiUploader.svelte';
	import { authStore } from '$stores/auth.js';

	export let serverId: string;

	interface CustomEmoji {
		id: string;
		name: string;
		url: string;
		uploaderUsername: string;
		createdAt: string;
	}

	let emojis: CustomEmoji[] = [];
	let loading = true;
	let showUploader = false;

	const FREE_LIMIT = 50;
	const PRO_LIMIT = 100;

	$: isPremium = $authStore.user?.isPremium ?? false;
	$: limit = isPremium ? PRO_LIMIT : FREE_LIMIT;
	$: atLimit = emojis.length >= limit;

	onMount(async () => {
		await loadEmojis();
	});

	async function loadEmojis() {
		loading = true;
		try {
			emojis = await api.get<CustomEmoji[]>(`/api/v2/servers/${serverId}/emojis`);
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : 'Failed to load emojis');
		} finally {
			loading = false;
		}
	}

	async function deleteEmoji(id: string, name: string) {
		try {
			await api.delete(`/api/v2/servers/${serverId}/emojis/${id}`);
			emojis = emojis.filter((e) => e.id !== id);
			toast.success(`:${name}: deleted.`);
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : 'Failed to delete emoji');
		}
	}

	function handleUploaded(e: CustomEvent<{ id: string; name: string; url: string }>) {
		emojis = [...emojis, { ...e.detail, uploaderUsername: '', createdAt: new Date().toISOString() }];
		showUploader = false;
	}

	function relativeDate(iso: string): string {
		const d = new Date(iso);
		return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
	}
</script>

<div class="emoji-manager">
	<div class="manager-header">
		<div>
			<h3 class="manager-title">Custom Emojis</h3>
			<span class="emoji-count">{emojis.length} / {limit}</span>
		</div>
		{#if !atLimit}
			<button class="add-btn" on:click={() => (showUploader = !showUploader)}>
				{showUploader ? 'Cancel' : '+ Add Emoji'}
			</button>
		{/if}
	</div>

	<!-- At limit banner -->
	{#if atLimit}
		<div class="limit-banner">
			<span class="limit-icon">🚀</span>
			<div>
				<strong>You've reached the {limit} emoji limit.</strong>
				{#if !isPremium}
					<p>Upgrade to GoPro to add up to 100 custom emojis.</p>
					<a href="/settings?tab=premium" class="upgrade-link">Upgrade to GoPro →</a>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Uploader -->
	{#if showUploader && !atLimit}
		<div class="uploader-wrap">
			<EmojiUploader {serverId} on:uploaded={handleUploaded} />
		</div>
	{/if}

	<!-- Emoji list -->
	{#if loading}
		<div class="loading">Loading emojis…</div>
	{:else if emojis.length === 0}
		<div class="empty-state">
			<span class="empty-icon">🖼</span>
			<p>No custom emojis yet. Add your first one!</p>
		</div>
	{:else}
		<div class="emoji-list">
			{#each emojis as emoji}
				<div class="emoji-row">
					<img src={emoji.url} alt=":{emoji.name}:" class="emoji-preview" />
					<div class="emoji-info">
						<span class="emoji-name">:{emoji.name}:</span>
						{#if emoji.uploaderUsername}
							<span class="emoji-uploader">by @{emoji.uploaderUsername}</span>
						{/if}
					</div>
					<span class="emoji-date">{relativeDate(emoji.createdAt)}</span>
					<button class="delete-btn" on:click={() => deleteEmoji(emoji.id, emoji.name)} title="Delete emoji" aria-label="Delete emoji">
						🗑
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.emoji-manager {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.manager-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.manager-title {
		font-size: 16px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0 0 4px;
	}

	.emoji-count {
		font-size: 13px;
		color: #6b7280;
	}

	.add-btn {
		padding: 8px 16px;
		background: rgba(124, 58, 237, 0.15);
		color: #a78bfa;
		border: 1px solid rgba(124, 58, 237, 0.3);
		border-radius: 8px;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: background 150ms;
	}

	.add-btn:hover {
		background: rgba(124, 58, 237, 0.3);
	}

	.limit-banner {
		display: flex;
		gap: 12px;
		padding: 14px 16px;
		background: linear-gradient(135deg, rgba(245, 158, 11, 0.08), rgba(124, 58, 237, 0.08));
		border: 1px solid rgba(245, 158, 11, 0.25);
		border-radius: 12px;
		font-size: 13px;
		color: #f9fafb;
	}

	.limit-icon {
		font-size: 22px;
		flex-shrink: 0;
	}

	.limit-banner strong {
		display: block;
		margin-bottom: 4px;
	}

	.limit-banner p {
		margin: 0;
		color: #9ca3af;
		font-size: 13px;
	}

	.upgrade-link {
		display: inline-block;
		margin-top: 6px;
		color: #a78bfa;
		font-weight: 600;
		font-size: 13px;
		text-decoration: none;
	}

	.upgrade-link:hover {
		color: #c4b5fd;
	}

	.uploader-wrap {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 20px;
	}

	.loading {
		font-size: 14px;
		color: #6b7280;
		text-align: center;
		padding: 24px;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding: 32px;
		color: #6b7280;
		font-size: 14px;
		text-align: center;
	}

	.empty-icon {
		font-size: 32px;
	}

	.emoji-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.emoji-row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border-radius: 8px;
		transition: background 100ms;
	}

	.emoji-row:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.emoji-preview {
		width: 32px;
		height: 32px;
		object-fit: contain;
		flex-shrink: 0;
	}

	.emoji-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.emoji-name {
		font-size: 14px;
		font-weight: 600;
		color: #f9fafb;
		font-family: monospace;
	}

	.emoji-uploader {
		font-size: 12px;
		color: #6b7280;
	}

	.emoji-date {
		font-size: 12px;
		color: #6b7280;
		flex-shrink: 0;
	}

	.delete-btn {
		background: none;
		border: none;
		font-size: 16px;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 6px;
		opacity: 0.6;
		transition: opacity 100ms, background 100ms;
	}

	.delete-btn:hover {
		opacity: 1;
		background: rgba(239, 68, 68, 0.1);
	}
</style>
