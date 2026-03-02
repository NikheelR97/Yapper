<script lang="ts">
	import { authStore, clearAuth } from '$stores/auth.js';
	import { goto } from '$app/navigation';

	function handleSignOut() {
		clearAuth();
		goto('/login');
	}
</script>

<svelte:head>
	<title>Yapper</title>
</svelte:head>

<div class="explore-placeholder">
	<div class="sphere" aria-hidden="true"></div>
	<h1>You're in.</h1>
	<p>Welcome, <strong>{$authStore.user?.displayName || $authStore.user?.username}</strong>.</p>
	<p class="sub">The main app is coming in S3. Stay tuned.</p>
	<button class="sign-out" on:click={handleSignOut}>Sign out</button>
</div>

<style>
	.explore-placeholder {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		gap: 1rem;
		background: var(--color-bg-base);
		text-align: center;
	}

	.sphere {
		width: 80px;
		height: 80px;
		border-radius: 50%;
		background: radial-gradient(circle at 35% 35%, #a78bfa, #7c3aed 40%, #3b0764 80%);
		box-shadow: 0 0 40px rgba(124, 58, 237, 0.5);
		margin-bottom: 0.5rem;
	}

	h1 {
		font-size: 2rem;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0;
	}

	p {
		color: var(--color-text-secondary);
		margin: 0;
	}

	.sub {
		color: var(--color-text-muted);
		font-size: 0.875rem;
	}

	.sign-out {
		margin-top: 1rem;
		background: transparent;
		border: 1px solid var(--color-border);
		color: var(--color-text-secondary);
		border-radius: var(--radius-md);
		padding: 0.5rem 1.25rem;
		font-size: 0.875rem;
		cursor: pointer;
		transition: border-color 0.15s, color 0.15s;
	}

	.sign-out:hover {
		border-color: var(--color-brand);
		color: var(--color-text-primary);
	}
</style>
