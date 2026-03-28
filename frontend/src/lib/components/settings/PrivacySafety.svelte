<script lang="ts">
	import { api } from "$api/client.js";
	import { toast } from "$stores/toast.js";
	import { onMount } from "svelte";
	import { isTauri as _isTauri } from "$lib/plugins/tauri-compat.js";
	const isTauri = _isTauri();

	let dm_permission: "everyone" | "friends" | "nobody" = "everyone";
	let friend_request_permission:
		| "everyone"
		| "friends_of_friends"
		| "nobody" = "everyone";
	let show_last_seen = true;
	let saving = false;

	// Pre-populate from the API
	onMount(async () => {
		try {
			const priv = await api.get<{
				dm_permission: string;
				friend_request_permission: string;
				show_last_seen: boolean;
			}>("/api/v2/users/me/privacy");
			dm_permission = priv.dm_permission as typeof dm_permission;
			friend_request_permission =
				priv.friend_request_permission as typeof friend_request_permission;
			show_last_seen = priv.show_last_seen;
		} catch {
			// Non-critical — defaults are fine
		}
	});

	async function save() {
		saving = true;
		try {
			await api.patch("/api/v2/users/me/privacy", {
				dm_permission,
				friend_request_permission,
				show_last_seen,
			});
			toast.success("Privacy settings saved!");
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : "Failed to save");
		} finally {
			saving = false;
		}
	}
</script>

<div class="privacy-section">
	<h2 class="section-title">Privacy & Safety</h2>

	<!-- DMs -->
	<div class="setting-block">
		<div class="block-header">
			<h3 class="block-title">Who can send you DMs?</h3>
		</div>
		<div class="radio-group">
			{#each [["everyone", "Everyone"], ["friends", "Friends only"], ["nobody", "Nobody"]] as [val, label]}
				<label class="radio-row">
					<input
						type="radio"
						name="dm"
						value={val}
						bind:group={dm_permission}
					/>
					<span class="radio-label">{label}</span>
				</label>
			{/each}
		</div>
	</div>

	<!-- Friend requests -->
	<div class="setting-block">
		<div class="block-header">
			<h3 class="block-title">Who can send you friend requests?</h3>
		</div>
		<div class="radio-group">
			{#each [["everyone", "Everyone"], ["friends_of_friends", "Friends of friends"], ["nobody", "Nobody"]] as [val, label]}
				<label class="radio-row">
					<input
						type="radio"
						name="friend_req"
						value={val}
						bind:group={friend_request_permission}
					/>
					<span class="radio-label">{label}</span>
				</label>
			{/each}
		</div>
	</div>

	<!-- Last seen -->
	<div class="setting-block">
		<div class="toggle-row">
			<div>
				<h3 class="block-title">Show last seen status</h3>
				<p class="block-desc">
					Let others see when you were last online.
				</p>
			</div>
			<label class="toggle-switch">
				<input type="checkbox" bind:checked={show_last_seen} />
				<span class="toggle-track"></span>
			</label>
		</div>
	</div>

	<!-- Key Storage indicator -->
	<div class="setting-block info-block">
		<div class="block-header">
			<h3 class="block-title">KEY STORAGE</h3>
		</div>
		<div class="key-storage-row">
			<span class="check">✓</span>
			<div>
				<div class="storage-name">
					{#if isTauri}
						Tauri Stronghold (encrypted native storage)
					{:else}
						IndexedDB (browser storage)
					{/if}
				</div>
				<div class="storage-desc">
					{#if isTauri}
						Your encryption keys are protected by your OS keychain.
					{:else}
						Your encryption keys are stored locally in your browser.
					{/if}
				</div>
			</div>
		</div>
	</div>

	<button class="save-btn" on:click={save} disabled={saving}>
		{saving ? "Saving…" : "Save Privacy Settings"}
	</button>
</div>

<style>
	.privacy-section {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.section-title {
		font-size: 20px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
	}

	.setting-block {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 12px;
		padding: 18px;
	}

	.info-block {
		background: rgba(124, 58, 237, 0.05);
		border-color: rgba(124, 58, 237, 0.2);
	}

	.block-header {
		margin-bottom: 12px;
	}

	.block-title {
		font-size: 13px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.block-desc {
		font-size: 13px;
		color: #6b7280;
		margin: 4px 0 0;
	}

	.radio-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.radio-row {
		display: flex;
		align-items: center;
		gap: 10px;
		cursor: pointer;
	}

	.radio-row input[type="radio"] {
		accent-color: #7c3aed;
		width: 16px;
		height: 16px;
		cursor: pointer;
	}

	.radio-label {
		font-size: 14px;
		color: #d1d5db;
	}

	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.toggle-switch {
		position: relative;
		display: inline-block;
		width: 44px;
		height: 24px;
		flex-shrink: 0;
	}

	.toggle-switch input {
		opacity: 0;
		width: 0;
		height: 0;
	}

	.toggle-track {
		position: absolute;
		inset: 0;
		background: #374151;
		border-radius: 12px;
		cursor: pointer;
		transition: background 200ms;
	}

	.toggle-track::after {
		content: "";
		position: absolute;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: white;
		top: 3px;
		left: 3px;
		transition: transform 200ms;
	}

	.toggle-switch input:checked + .toggle-track {
		background: #22c55e;
	}

	.toggle-switch input:checked + .toggle-track::after {
		transform: translateX(20px);
	}

	.key-storage-row {
		display: flex;
		gap: 10px;
		align-items: flex-start;
	}

	.check {
		color: #22c55e;
		font-size: 16px;
		font-weight: 700;
		flex-shrink: 0;
	}

	.storage-name {
		font-size: 14px;
		font-weight: 600;
		color: #f9fafb;
	}

	.storage-desc {
		font-size: 12px;
		color: #9ca3af;
		margin-top: 2px;
	}

	.save-btn {
		padding: 12px 24px;
		background: #7c3aed;
		color: white;
		border: none;
		border-radius: 10px;
		font-size: 15px;
		font-weight: 700;
		cursor: pointer;
		align-self: flex-start;
		transition: opacity 150ms;
	}

	.save-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
