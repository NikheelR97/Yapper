<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';

	let theme: 'dark' | 'oled' | 'light' = 'dark';
	let fontSize = 14;
	let density: 'comfortable' | 'compact' = 'comfortable';
	let reduceMotion = false;
	let saving = false;

	onMount(async () => {
		try {
			const res = await api.get<{
				theme: 'dark' | 'oled' | 'light';
				fontSize: number;
				density: 'comfortable' | 'compact';
				reduceMotion: boolean;
			}>('/api/v2/users/me/appearance');
			theme = res.theme;
			fontSize = res.fontSize;
			density = res.density;
			reduceMotion = res.reduceMotion;
			applyToDocument();
		} catch {
			// Use defaults — non-fatal
		}
	});

	function applyToDocument() {
		document.documentElement.style.setProperty('--font-size-base', `${fontSize}px`);
		document.documentElement.dataset.theme = theme;
		document.documentElement.dataset.density = density;
		if (reduceMotion) {
			document.documentElement.dataset.reduceMotion = 'true';
		} else {
			delete document.documentElement.dataset.reduceMotion;
		}
	}

	async function save() {
		saving = true;
		try {
			await api.patch('/api/v2/users/me/appearance', {
				theme,
				fontSize,
				density,
				reduceMotion,
			});
			applyToDocument();
			toast.success('Appearance saved!');
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : 'Failed to save');
		} finally {
			saving = false;
		}
	}

	const themeOptions = [
		{ value: 'dark', label: 'Dark', desc: 'Deep space (default)', preview: '#0a0a0f' },
		{ value: 'oled', label: 'OLED', desc: 'True black for OLED screens', preview: '#000000' },
		{ value: 'light', label: 'Light', desc: 'Bright theme', preview: '#f9fafb' },
	];
</script>

<div class="appearance-section">
	<h2 class="section-title">Appearance</h2>

	<!-- Theme -->
	<div class="setting-block">
		<h3 class="block-title">Theme</h3>
		<div class="theme-cards">
			{#each themeOptions as opt}
				<label class="theme-card" class:active={theme === opt.value}>
					<input type="radio" name="theme" value={opt.value} bind:group={theme} class="sr-only" />
					<div class="theme-preview" style="background: {opt.preview}; border: 1px solid var(--color-border)">
						<div class="theme-preview-inner">
							<div class="mock-bar" style="background: var(--color-bg-elevated)"></div>
							<div class="mock-msg" style="background: rgba(124,58,237,0.4)"></div>
							<div class="mock-msg short" style="background: var(--color-bg-elevated)"></div>
						</div>
					</div>
					<div class="theme-info">
						<span class="theme-name">{opt.label}</span>
						<span class="theme-desc">{opt.desc}</span>
					</div>
				</label>
			{/each}
		</div>
	</div>

	<!-- Font size -->
	<div class="setting-block">
		<h3 class="block-title">Font Size</h3>
		<div class="slider-row">
			<span class="slider-label small">A</span>
			<input type="range" min="12" max="18" step="1" bind:value={fontSize} class="slider" />
			<span class="slider-label large">A</span>
			<span class="slider-value">{fontSize}px</span>
		</div>
	</div>

	<!-- Message density -->
	<div class="setting-block">
		<h3 class="block-title">Message Density</h3>
		<div class="radio-group">
			<label class="radio-row">
				<input type="radio" name="density" value="comfortable" bind:group={density} />
				<div>
					<span class="radio-label">Comfortable</span>
					<span class="radio-desc"> — More space between messages</span>
				</div>
			</label>
			<label class="radio-row">
				<input type="radio" name="density" value="compact" bind:group={density} />
				<div>
					<span class="radio-label">Compact</span>
					<span class="radio-desc"> — See more messages at once</span>
				</div>
			</label>
		</div>
	</div>

	<!-- Reduce motion -->
	<div class="setting-block">
		<div class="toggle-row">
			<div>
				<h3 class="block-title">Reduce Motion</h3>
				<p class="block-desc">Disable animations and transitions.</p>
			</div>
			<label class="toggle-switch">
				<input type="checkbox" bind:checked={reduceMotion} />
				<span class="toggle-track"></span>
			</label>
		</div>
	</div>

	<button class="save-btn" on:click={save} disabled={saving}>
		{saving ? 'Saving…' : 'Save Appearance'}
	</button>
</div>

<style>
	.appearance-section {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.section-title {
		font-size: 20px;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0;
	}

	.setting-block {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		padding: 18px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.block-title {
		font-size: 13px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.block-desc {
		font-size: 13px;
		color: var(--color-text-secondary);
		margin: 4px 0 0;
	}

	/* Theme cards */
	.theme-cards {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
	}

	.theme-card {
		cursor: pointer;
		border: 2px solid var(--color-border);
		border-radius: 10px;
		overflow: hidden;
		transition: border-color 150ms;
	}

	.theme-card.active {
		border-color: var(--color-brand);
	}

	.theme-preview {
		height: 80px;
		padding: 8px;
	}

	.theme-preview-inner {
		display: flex;
		flex-direction: column;
		gap: 4px;
		height: 100%;
		justify-content: center;
	}

	.mock-bar {
		height: 8px;
		border-radius: 4px;
		width: 60%;
	}

	.mock-msg {
		height: 14px;
		border-radius: 4px;
		width: 80%;
	}

	.mock-msg.short {
		width: 50%;
	}

	.theme-info {
		padding: 8px 10px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.theme-name {
		font-size: 13px;
		font-weight: 700;
		color: var(--color-text-primary);
	}

	.theme-desc {
		font-size: 11px;
		color: var(--color-text-secondary);
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

	/* Slider */
	.slider-row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.slider-label {
		color: var(--color-text-secondary);
		font-weight: 600;
	}

	.slider-label.small {
		font-size: 12px;
	}

	.slider-label.large {
		font-size: 18px;
	}

	.slider {
		flex: 1;
		accent-color: var(--color-brand);
		cursor: pointer;
	}

	.slider-value {
		font-size: 13px;
		color: var(--color-text-secondary);
		width: 32px;
		text-align: right;
	}

	/* Radio */
	.radio-group {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.radio-row {
		display: flex;
		align-items: center;
		gap: 10px;
		cursor: pointer;
	}

	.radio-row input[type='radio'] {
		accent-color: var(--color-brand);
		width: 16px;
		height: 16px;
		cursor: pointer;
		flex-shrink: 0;
	}

	.radio-label {
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.radio-desc {
		font-size: 14px;
		color: var(--color-text-secondary);
	}

	/* Toggle */
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
		background: var(--color-border);
		border-radius: 12px;
		cursor: pointer;
		transition: background 200ms;
	}

	.toggle-track::after {
		content: '';
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
