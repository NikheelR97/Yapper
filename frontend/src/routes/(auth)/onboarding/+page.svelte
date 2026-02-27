<script lang="ts">
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import { authStore } from '$stores/auth.js';
	import { onMount } from 'svelte';

	let step = 0;
	const totalSteps = 2;

	onMount(() => {
		if (!get(authStore).user) goto('/login');
	});

	function next() {
		if (step < totalSteps - 1) {
			step++;
		} else {
			goto('/explore');
		}
	}
</script>

<svelte:head>
	<title>Welcome to Yapper</title>
</svelte:head>

<div class="onboarding">
	<!-- Dot pagination -->
	<div class="dots" aria-label="Step {step + 1} of {totalSteps}">
		{#each Array(totalSteps) as _, i}
			<button
				class="dot"
				class:active={i === step}
				on:click={() => (step = i)}
				aria-label="Go to step {i + 1}"
			></button>
		{/each}
	</div>

	{#if step === 0}
		<!-- Screen 1: A New Way to Yap -->
		<div class="slide" id="slide-0">
			<div class="sphere-container">
				<div class="sphere"></div>
			</div>
			<h1 class="slide-title">A New Way to Yap.</h1>
			<p class="slide-body">
				End-to-end encrypted messages, voice <strong>Yaps</strong>, video
				<strong>Clips</strong>, and live canvases — all in one place.
			</p>
			<button class="btn-primary" on:click={next}>Continue</button>
		</div>

	{:else if step === 1}
		<!-- Screen 2: Find Your Hype -->
		<div class="slide" id="slide-1">
			<div class="community-preview">
				{#each ['Gaming', 'Music', 'Crypto', 'Anime', 'Tech'] as tag}
					<span class="community-chip">{tag}</span>
				{/each}
			</div>
			<h1 class="slide-title">Find Your Hype.</h1>
			<p class="slide-body">
				Discover thousands of communities. Join servers, follow yappers,
				and start talking — all encrypted end-to-end.
			</p>
			<button class="btn-primary" on:click={next}>Ready to Yap →</button>
		</div>
	{/if}
</div>

<style>
	.onboarding {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		min-height: 100dvh;
		padding: 2rem;
		background: var(--color-bg-base);
		text-align: center;
	}

	.dots {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 3rem;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-border);
		border: none;
		cursor: pointer;
		padding: 0;
		transition: all var(--transition-base);
	}

	.dot.active {
		background: var(--color-brand);
		width: 24px;
		border-radius: var(--radius-full);
	}

	.slide {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1.5rem;
		max-width: 420px;
		width: 100%;
		animation: fadeUp 0.3s ease;
	}

	@keyframes fadeUp {
		from { opacity: 0; transform: translateY(16px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.sphere-container {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 240px;
	}

	.sphere {
		width: 200px;
		height: 200px;
		border-radius: 50%;
		background: radial-gradient(circle at 35% 35%, #a78bfa, #7c3aed 40%, #3b0764 80%);
		box-shadow: 0 0 80px rgba(124, 58, 237, 0.5), 0 0 160px rgba(124, 58, 237, 0.2);
		animation: pulse 3s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { box-shadow: 0 0 80px rgba(124, 58, 237, 0.5), 0 0 160px rgba(124, 58, 237, 0.2); }
		50% { box-shadow: 0 0 100px rgba(124, 58, 237, 0.7), 0 0 200px rgba(124, 58, 237, 0.3); }
	}

	.community-preview {
		display: flex;
		flex-wrap: wrap;
		gap: 0.625rem;
		justify-content: center;
		padding: 2rem 0;
	}

	.community-chip {
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		color: var(--color-text-secondary);
		padding: 0.5rem 1rem;
		border-radius: var(--radius-full);
		font-size: 0.9375rem;
		font-weight: 500;
	}

	.slide-title {
		font-size: 2.25rem;
		font-weight: 800;
		color: var(--color-text-primary);
		line-height: 1.2;
	}

	.slide-body {
		color: var(--color-text-secondary);
		font-size: 1.0625rem;
		line-height: 1.6;
		max-width: 360px;
	}

	.slide-body strong {
		color: var(--color-brand-light);
	}

	.btn-primary {
		background: var(--color-brand);
		color: white;
		border: none;
		border-radius: var(--radius-md);
		font-size: 1rem;
		font-weight: 600;
		padding: 0.875rem 2.5rem;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.btn-primary:hover {
		background: var(--color-brand-dark);
	}
</style>
