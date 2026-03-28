<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { goto } from '$app/navigation';
	import { createServer } from '$stores/servers.js';

	export let open = false;

	const dispatch = createEventDispatcher<{ close: void }>();

	let serverName = '';
	let description = '';
	let isPublic = false;
	let iconPreview = '';
	let creating = false;
	let nameInput: HTMLInputElement | null = null;

	$: if (open) {
		serverName = '';
		description = '';
		isPublic = false;
		iconPreview = '';
		creating = false;
	}

	function handleIconChange(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		const reader = new FileReader();
		reader.onload = (ev) => { iconPreview = ev.target?.result as string; };
		reader.readAsDataURL(file);
	}

	async function handleCreate() {
		if (!serverName.trim() || creating) return;
		creating = true;
		try {
			const server = await createServer(serverName.trim());
			dispatch('close');
			goto(`/servers/${server.id}/channels`);
		} finally {
			creating = false;
		}
	}

	function handleBackdrop() {
		if (!creating) dispatch('close');
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && !creating) dispatch('close');
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
	<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
	<div class="backdrop" on:click={handleBackdrop}>
		<div
			class="modal"
			role="dialog"
			aria-modal="true"
			aria-label="Create a server"
			tabindex="-1"
			on:click|stopPropagation
		>
			<div class="modal-header">
				<h2 class="modal-title">Create a Server</h2>
				<button
					class="close-btn"
					on:click={() => dispatch('close')}
					aria-label="Close"
					disabled={creating}
				>
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
					</svg>
				</button>
			</div>

			<!-- Icon upload -->
			<div class="icon-row">
				<label class="icon-upload" title="Upload server icon (optional)">
					{#if iconPreview}
						<img src={iconPreview} alt="Server icon preview" class="icon-preview" />
					{:else}
						<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/>
							<circle cx="12" cy="13" r="4"/>
						</svg>
						<span class="icon-label">Upload Icon</span>
					{/if}
					<input type="file" accept="image/*" on:change={handleIconChange} class="sr-only" tabindex="-1" />
				</label>
				<span class="icon-hint">Optional · Square works best</span>
			</div>

			<!-- Name -->
			<label class="field-label" for="csm-name">Server Name</label>
			<input
				id="csm-name"
				bind:this={nameInput}
				bind:value={serverName}
				class="field-input"
				placeholder="My Awesome Server"
				maxlength="100"
				required
				disabled={creating}
			/>

			<!-- Description -->
			<label class="field-label" for="csm-desc">
				Description <span class="optional">(optional)</span>
			</label>
			<textarea
				id="csm-desc"
				bind:value={description}
				class="field-input field-textarea"
				placeholder="What's this server about?"
				maxlength="500"
				rows="2"
				disabled={creating}
			></textarea>

			<!-- Public toggle -->
			<label class="toggle-row">
				<input type="checkbox" bind:checked={isPublic} disabled={creating} class="toggle-check" />
				<div class="toggle-text">
					<span class="toggle-label">Make this server public</span>
					<span class="toggle-hint">Anyone can find and join</span>
				</div>
			</label>

			<div class="modal-actions">
				<button
					class="btn-cancel"
					on:click={() => dispatch('close')}
					disabled={creating}
				>Cancel</button>
				<button
					class="btn-create"
					on:click={handleCreate}
					disabled={creating || !serverName.trim()}
				>
					{creating ? 'Creating…' : 'Create Server →'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.8);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
	}

	.modal {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 0.875rem;
		padding: 2rem;
		width: 440px;
		max-width: calc(100vw - 2rem);
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.25rem;
	}

	.modal-title {
		font-size: 1.125rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 0.25rem;
		border-radius: 0.25rem;
		display: flex;
		align-items: center;
		line-height: 1;
	}
	.close-btn:hover { color: var(--color-text-primary); }
	.close-btn:disabled { opacity: 0.4; cursor: default; }

	/* Icon upload */
	.icon-row {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		margin: 0.25rem 0;
	}

	.icon-upload {
		width: 80px;
		height: 80px;
		border-radius: 50%;
		border: 2px dashed rgba(124, 58, 237, 0.4);
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.25rem;
		cursor: pointer;
		color: var(--color-text-muted);
		transition: border-color 0.15s, color 0.15s, background 0.15s;
		overflow: hidden;
	}
	.icon-upload:hover {
		border-color: rgba(124, 58, 237, 0.7);
		color: var(--color-brand);
		background: rgba(124, 58, 237, 0.06);
	}

	.icon-preview {
		width: 80px;
		height: 80px;
		object-fit: cover;
	}

	.icon-label {
		font-size: 0.625rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.icon-hint {
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border-width: 0;
	}

	/* Fields */
	.field-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		margin-bottom: -0.5rem;
	}

	.optional {
		font-weight: 400;
		text-transform: none;
		letter-spacing: 0;
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		opacity: 0.7;
	}

	.field-input {
		width: 100%;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		padding: 0.625rem 0.75rem;
		font-size: 0.9375rem;
		color: var(--color-text-primary);
		box-sizing: border-box;
		font-family: inherit;
		transition: border-color 0.15s;
	}
	.field-input:focus {
		outline: none;
		border-color: var(--color-brand);
	}
	.field-input::placeholder { color: var(--color-text-muted); }
	.field-input:disabled { opacity: 0.5; }

	.field-textarea {
		resize: none;
		line-height: 1.5;
	}

	/* Toggle */
	.toggle-row {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		cursor: pointer;
		padding: 0.25rem 0;
	}

	.toggle-check {
		margin-top: 0.125rem;
		width: 16px;
		height: 16px;
		accent-color: var(--color-brand);
		flex-shrink: 0;
		cursor: pointer;
	}

	.toggle-text {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.toggle-label {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--color-text-primary);
	}

	.toggle-hint {
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	/* Actions */
	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.625rem;
		margin-top: 0.25rem;
	}

	.btn-cancel {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		padding: 0.5625rem 1.125rem;
		font-size: 0.875rem;
		color: var(--color-text-muted);
		cursor: pointer;
		font-family: inherit;
		transition: color 0.15s, border-color 0.15s;
	}
	.btn-cancel:hover { color: var(--color-text-primary); border-color: var(--color-text-muted); }
	.btn-cancel:disabled { opacity: 0.4; cursor: default; }

	.btn-create {
		background: var(--color-brand);
		border: none;
		border-radius: 0.5rem;
		padding: 0.5625rem 1.25rem;
		font-size: 0.875rem;
		font-weight: 600;
		color: white;
		cursor: pointer;
		font-family: inherit;
		transition: opacity 0.15s;
	}
	.btn-create:hover { opacity: 0.9; }
	.btn-create:disabled { opacity: 0.4; cursor: default; }
</style>
