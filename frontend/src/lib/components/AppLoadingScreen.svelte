<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	const steps = [
		'Initializing encryption…',
		'Connecting…',
		'Loading your messages…',
	];

	let stepIndex = 0;
	let interval: ReturnType<typeof setInterval>;

	onMount(() => {
		interval = setInterval(() => {
			stepIndex = (stepIndex + 1) % steps.length;
		}, 1200);
	});

	onDestroy(() => {
		clearInterval(interval);
	});
</script>

<div class="loading-screen" aria-live="polite" aria-label="Loading Yapper">
	<div class="sphere-wrap">
		<div class="sphere"></div>
	</div>

	<h1 class="app-name">Yapper</h1>

	<div class="progress-bar">
		<div class="progress-fill"></div>
	</div>

	<p class="status-text">{steps[stepIndex]}</p>
</div>

<style>
	.loading-screen {
		position: fixed;
		inset: 0;
		background: #0a0a0f;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 16px;
		z-index: 99999;
	}

	.sphere-wrap {
		width: 120px;
		height: 120px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.sphere {
		width: 80px;
		height: 80px;
		border-radius: 50%;
		background: radial-gradient(circle at 35% 35%, #c4b5fd, #7c3aed 45%, #2e1065);
		box-shadow: 0 0 40px rgba(124, 58, 237, 0.5);
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% {
			transform: scale(1);
			box-shadow: 0 0 40px rgba(124, 58, 237, 0.5);
		}
		50% {
			transform: scale(1.08);
			box-shadow: 0 0 60px rgba(124, 58, 237, 0.8);
		}
	}

	.app-name {
		font-size: 32px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
		letter-spacing: -0.5px;
	}

	.progress-bar {
		width: 240px;
		height: 3px;
		background: rgba(255, 255, 255, 0.07);
		border-radius: 2px;
		overflow: hidden;
		margin-top: 8px;
	}

	.progress-fill {
		height: 100%;
		background: #7c3aed;
		border-radius: 2px;
		animation: indeterminate 1.5s ease-in-out infinite;
	}

	@keyframes indeterminate {
		0% {
			width: 0%;
			margin-left: 0%;
		}
		50% {
			width: 70%;
			margin-left: 15%;
		}
		100% {
			width: 0%;
			margin-left: 100%;
		}
	}

	.status-text {
		font-size: 14px;
		color: #6b7280;
		margin: 0;
		animation: fade 0.4s ease-out;
	}

	@keyframes fade {
		from { opacity: 0; }
		to { opacity: 1; }
	}
</style>
