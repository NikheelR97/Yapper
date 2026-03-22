<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { get } from 'svelte/store';
	import { authStore } from '$stores/auth.js';
	import { toast } from '$stores/toast.js';
	import { API_URL } from '$lib/env.js';

	export let serverId: string;

	const dispatch = createEventDispatcher<{ uploaded: { id: string; name: string; url: string } }>();

	let name = '';
	let file: File | null = null;
	let preview: string | null = null;
	let uploading = false;
	let dragOver = false;

	function slugify(s: string): string {
		return s.toLowerCase().replace(/[^a-z0-9_]/g, '_').replace(/__+/g, '_');
	}

	$: name = file ? slugify(file.name.replace(/\.[^.]+$/, '')) : name;

	function handleFileChange(e: Event) {
		const target = e.target as HTMLInputElement;
		const f = target.files?.[0];
		if (!f) return;
		setFile(f);
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		const f = e.dataTransfer?.files?.[0];
		if (f && f.type.startsWith('image/')) setFile(f);
	}

	function setFile(f: File) {
		if (f.size > 256 * 1024) {
			toast.error('Emoji must be under 256KB.');
			return;
		}
		file = f;
		const reader = new FileReader();
		reader.onload = (ev) => (preview = ev.target?.result as string);
		reader.readAsDataURL(f);
	}

	async function upload() {
		if (!file || !name.trim()) {
			toast.error('Choose a file and enter a name.');
			return;
		}
		uploading = true;

		try {
			const formData = new FormData();
			formData.append('file', file);
			formData.append('name', name.trim());

			// Use fetch directly (FormData, not JSON)
			const { accessToken } = get(authStore);
			const BASE_URL = API_URL;

			const res = await fetch(`${BASE_URL}/api/v1/servers/${serverId}/emojis`, {
				method: 'POST',
				body: formData,
				headers: accessToken ? { Authorization: `Bearer ${accessToken}` } : {},
				credentials: 'include',
			});

			if (!res.ok) {
				const err = await res.json().catch(() => ({ error: 'Upload failed' }));
				throw new Error(err.error);
			}

			const emoji = await res.json();
			toast.success(`Emoji :${emoji.name}: uploaded!`);
			dispatch('uploaded', emoji);
			file = null;
			preview = null;
			name = '';
		} catch (e: any) {
			toast.error(e.message ?? 'Upload failed');
		} finally {
			uploading = false;
		}
	}
</script>

<div class="emoji-uploader">
	<!-- Drop zone -->
	<div
		class="drop-zone"
		class:drag-over={dragOver}
		on:dragover|preventDefault={() => (dragOver = true)}
		on:dragleave={() => (dragOver = false)}
		on:drop={handleDrop}
		role="region"
		aria-label="Drop emoji image here"
	>
		{#if preview}
			<img src={preview} alt="Emoji preview" class="preview-img" />
			<p class="preview-hint">Click to change</p>
		{:else}
			<span class="drop-icon">🖼</span>
			<p class="drop-hint">Drag & drop or <label class="file-label" for="emoji-file">browse</label></p>
			<p class="drop-sub">PNG, GIF, WEBP — max 256KB</p>
		{/if}
		<input
			id="emoji-file"
			type="file"
			accept="image/png,image/gif,image/webp"
			class="file-input"
			on:change={handleFileChange}
		/>
	</div>

	<!-- Name -->
	<div class="field">
		<label class="field-label" for="emojiName">Emoji Name</label>
		<div class="name-row">
			<span class="name-colon">:</span>
			<input
				id="emojiName"
				type="text"
				class="field-input"
				bind:value={name}
				placeholder="my_emoji"
				maxlength={32}
				pattern="[a-z0-9_]+"
			/>
			<span class="name-colon">:</span>
		</div>
	</div>

	<button class="upload-btn" on:click={upload} disabled={uploading || !file}>
		{uploading ? 'Uploading…' : 'Upload Emoji'}
	</button>
</div>

<style>
	.emoji-uploader {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.drop-zone {
		position: relative;
		border: 2px dashed rgba(255, 255, 255, 0.15);
		border-radius: 12px;
		padding: 32px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		cursor: pointer;
		transition: border-color 150ms, background 150ms;
	}

	.drop-zone:hover,
	.drop-zone.drag-over {
		border-color: #7c3aed;
		background: rgba(124, 58, 237, 0.05);
	}

	.drop-icon {
		font-size: 40px;
	}

	.drop-hint {
		font-size: 14px;
		color: #d1d5db;
		margin: 0;
		text-align: center;
	}

	.drop-sub {
		font-size: 12px;
		color: #6b7280;
		margin: 0;
	}

	.file-label {
		color: #a78bfa;
		cursor: pointer;
		font-weight: 600;
	}

	.file-input {
		position: absolute;
		inset: 0;
		opacity: 0;
		cursor: pointer;
	}

	.preview-img {
		width: 64px;
		height: 64px;
		object-fit: contain;
	}

	.preview-hint {
		font-size: 12px;
		color: #6b7280;
		margin: 0;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.field-label {
		font-size: 11px;
		font-weight: 700;
		color: #9ca3af;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.name-row {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.name-colon {
		font-size: 16px;
		color: #6b7280;
		font-weight: 700;
	}

	.field-input {
		flex: 1;
		padding: 10px 14px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 14px;
		font-family: monospace;
		outline: none;
		transition: border-color 150ms;
	}

	.field-input:focus {
		border-color: #7c3aed;
	}

	.upload-btn {
		padding: 12px 24px;
		background: #7c3aed;
		color: white;
		border: none;
		border-radius: 10px;
		font-size: 15px;
		font-weight: 700;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.upload-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.upload-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
</style>
