<script lang="ts">
	import { api } from "$api/client.js";
	import { toast } from "$stores/toast.js";
	import { authStore } from "$stores/auth.js";
	import { API_URL } from "$lib/env.js";
	import { onMount } from "svelte";
	import { get } from "svelte/store";
	import { isTauri as _isTauri } from "$lib/plugins/tauri-compat.js";
	const isTauri = _isTauri();

	let dm_permission: "everyone" | "friends" | "nobody" = "everyone";
	let friend_request_permission:
		| "everyone"
		| "friends_of_friends"
		| "nobody" = "everyone";
	let show_last_seen = true;
	let saving = false;
	let exporting = false;
	let deleting = false;
	let showDeleteConfirm = false;

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

	async function exportData() {
		exporting = true;
		try {
			const { accessToken } = get(authStore);
			const headers: Record<string, string> = {};
			if (accessToken) headers["Authorization"] = `Bearer ${accessToken}`;
			const res = await fetch(`${API_URL}/api/v2/account/data-export`, {
				credentials: "include",
				headers,
			});
			if (!res.ok) throw new Error("Export failed");
			const blob = await res.blob();
			const url = URL.createObjectURL(blob);
			const a = document.createElement("a");
			a.href = url;
			a.download = "yapper-data-export.zip";
			a.click();
			URL.revokeObjectURL(url);
			toast.success("Data export downloaded!");
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : "Failed to export data");
		} finally {
			exporting = false;
		}
	}

	async function deleteAccount() {
		deleting = true;
		try {
			await api.delete("/api/v2/account");
			toast.success("Account scheduled for deletion.");
			window.location.href = "/login";
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : "Failed to delete account");
		} finally {
			deleting = false;
			showDeleteConfirm = false;
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

	<!-- GDPR Data Export -->
	<div class="setting-block">
		<div class="block-header">
			<h3 class="block-title">Your Data</h3>
		</div>
		<p class="block-desc" style="margin-bottom: 12px;">
			Download a copy of all your Yapper data including your profile,
			servers, messages metadata, and settings.
		</p>
		<button class="export-btn" on:click={exportData} disabled={exporting}>
			{exporting ? "Preparing export…" : "Download My Data"}
		</button>
	</div>

	<!-- Account Deletion -->
	<div class="setting-block danger-block">
		<div class="block-header">
			<h3 class="block-title danger-title">Danger Zone</h3>
		</div>
		<p class="block-desc" style="margin-bottom: 12px;">
			Permanently delete your account and all associated data. This action
			cannot be undone.
		</p>
		{#if showDeleteConfirm}
			<div class="confirm-row">
				<span class="confirm-text">Are you sure? This is irreversible.</span>
				<button
					class="delete-btn"
					on:click={deleteAccount}
					disabled={deleting}
				>
					{deleting ? "Deleting…" : "Yes, delete my account"}
				</button>
				<button
					class="cancel-btn"
					on:click={() => (showDeleteConfirm = false)}
				>
					Cancel
				</button>
			</div>
		{:else}
			<button
				class="delete-btn"
				on:click={() => (showDeleteConfirm = true)}
			>
				Delete Account
			</button>
		{/if}
	</div>
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

	.export-btn {
		padding: 10px 20px;
		background: rgba(255, 255, 255, 0.08);
		color: #f9fafb;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 8px;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: background 150ms;
	}

	.export-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.12);
	}

	.export-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.danger-block {
		background: rgba(239, 68, 68, 0.05);
		border-color: rgba(239, 68, 68, 0.2);
	}

	.danger-title {
		color: #ef4444 !important;
	}

	.confirm-row {
		display: flex;
		align-items: center;
		gap: 12px;
		flex-wrap: wrap;
	}

	.confirm-text {
		font-size: 13px;
		color: #fca5a5;
		font-weight: 600;
	}

	.delete-btn {
		padding: 10px 20px;
		background: #dc2626;
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.delete-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.delete-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.cancel-btn {
		padding: 10px 20px;
		background: rgba(255, 255, 255, 0.08);
		color: #d1d5db;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: background 150ms;
	}

	.cancel-btn:hover {
		background: rgba(255, 255, 255, 0.12);
	}
</style>
