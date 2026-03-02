<script lang="ts">
	import { page } from '$app/stores';
</script>

<svelte:head>
	<title>{$page.status} — Yapper</title>
</svelte:head>

<div class="error-shell">
	<div class="sphere" aria-hidden="true"></div>
	<h1>{$page.status}</h1>
	<p>{$page.error?.message ?? 'Something went wrong.'}</p>
	<a href="/" class="home-link">Go home</a>
</div>

<style>
	.error-shell {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		gap: 0.75rem;
		background: var(--color-bg-base);
		text-align: center;
	}

	.sphere {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		background: radial-gradient(circle at 35% 35%, #a78bfa, #7c3aed 40%, #3b0764 80%);
		opacity: 0.6;
		margin-bottom: 0.5rem;
	}

	h1 {
		font-size: 3rem;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0;
	}

	p {
		color: var(--color-text-secondary);
		margin: 0;
	}

	.home-link {
		margin-top: 1rem;
		color: var(--color-brand-light);
		font-weight: 500;
		font-size: 0.9rem;
	}
</style>
