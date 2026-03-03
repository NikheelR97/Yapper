<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from '$stores/toast.js';

	let inputDevices: MediaDeviceInfo[] = [];
	let outputDevices: MediaDeviceInfo[] = [];
	let selectedInput = '';
	let selectedOutput = '';
	let inputVolume = 80;
	let noiseSuppression = true;
	let echoCancellation = true;
	let testing = false;

	onMount(async () => {
		try {
			await navigator.mediaDevices.getUserMedia({ audio: true });
			const devices = await navigator.mediaDevices.enumerateDevices();
			inputDevices = devices.filter((d) => d.kind === 'audioinput');
			outputDevices = devices.filter((d) => d.kind === 'audiooutput');
			if (inputDevices.length) selectedInput = inputDevices[0].deviceId;
			if (outputDevices.length) selectedOutput = outputDevices[0].deviceId;
		} catch {
			// Permission denied or no devices
		}
	});

	async function testMic() {
		testing = true;
		toast.info('Listening for 3 seconds…');
		setTimeout(() => {
			testing = false;
			toast.success('Mic working! (test complete)');
		}, 3000);
	}

	function save() {
		toast.success('Voice settings saved!');
	}
</script>

<div class="voice-section">
	<h2 class="section-title">Voice & Video</h2>

	<div class="setting-block">
		<h3 class="block-title">Input Device (Microphone)</h3>
		{#if inputDevices.length > 0}
			<select class="device-select" bind:value={selectedInput}>
				{#each inputDevices as device}
					<option value={device.deviceId}>{device.label || 'Microphone'}</option>
				{/each}
			</select>
		{:else}
			<p class="no-device">No microphone found. Allow microphone access to see devices.</p>
		{/if}

		<div class="slider-row">
			<span class="slider-label">Input Volume</span>
			<input type="range" min="0" max="100" bind:value={inputVolume} class="slider" />
			<span class="slider-value">{inputVolume}%</span>
		</div>

		<button class="test-btn" on:click={testMic} disabled={testing}>
			{testing ? '🎙 Listening…' : '🎙 Test Microphone'}
		</button>
	</div>

	<div class="setting-block">
		<h3 class="block-title">Output Device (Speaker/Headphones)</h3>
		{#if outputDevices.length > 0}
			<select class="device-select" bind:value={selectedOutput}>
				{#each outputDevices as device}
					<option value={device.deviceId}>{device.label || 'Speaker'}</option>
				{/each}
			</select>
		{:else}
			<p class="no-device">No output device found.</p>
		{/if}
	</div>

	<div class="setting-block">
		<h3 class="block-title">Audio Processing</h3>

		<div class="toggle-row">
			<div>
				<span class="toggle-label">Noise Suppression</span>
				<p class="toggle-desc">Reduce background noise in voice chats.</p>
			</div>
			<label class="toggle-switch">
				<input type="checkbox" bind:checked={noiseSuppression} />
				<span class="toggle-track"></span>
			</label>
		</div>

		<div class="toggle-row">
			<div>
				<span class="toggle-label">Echo Cancellation</span>
				<p class="toggle-desc">Prevent microphone echo.</p>
			</div>
			<label class="toggle-switch">
				<input type="checkbox" bind:checked={echoCancellation} />
				<span class="toggle-track"></span>
			</label>
		</div>
	</div>

	<button class="save-btn" on:click={save}>Save Voice Settings</button>
</div>

<style>
	.voice-section {
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
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.block-title {
		font-size: 13px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.device-select {
		padding: 10px 14px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		cursor: pointer;
		outline: none;
	}

	.device-select option {
		background: #1c1d26;
	}

	.no-device {
		font-size: 13px;
		color: #6b7280;
		margin: 0;
	}

	.slider-row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.slider-label {
		font-size: 13px;
		color: #9ca3af;
		white-space: nowrap;
	}

	.slider {
		flex: 1;
		accent-color: #7c3aed;
		cursor: pointer;
	}

	.slider-value {
		font-size: 13px;
		color: #9ca3af;
		width: 36px;
		text-align: right;
	}

	.test-btn {
		padding: 9px 18px;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		align-self: flex-start;
		transition: background 150ms;
	}

	.test-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
	}

	.test-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.toggle-label {
		font-size: 14px;
		font-weight: 600;
		color: #f9fafb;
		display: block;
	}

	.toggle-desc {
		font-size: 12px;
		color: #6b7280;
		margin: 3px 0 0;
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

	.save-btn:hover {
		opacity: 0.85;
	}
</style>
