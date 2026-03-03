<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { get } from 'svelte/store';
	import { setAuth } from '$stores/auth.js';
	import type { User } from '$stores/auth.js';

	let error = '';

	const API_URL = import.meta.env.VITE_API_URL ?? 'http://localhost:8080';

	onMount(async () => {
		const params = get(page).url.searchParams;
		const accessToken = params.get('access_token');
		const csrfToken = params.get('csrf_token');
		const isNew = params.get('is_new') === 'true';
		const oauthError = params.get('oauth_error');

		if (oauthError) {
			error = 'OAuth sign-in failed. Please try again.';
			setTimeout(() => goto('/login'), 3000);
			return;
		}

		if (!accessToken) {
			error = 'No access token received.';
			setTimeout(() => goto('/login'), 3000);
			return;
		}

		try {
			const res = await fetch(`${API_URL}/api/v1/users/me`, {
				headers: { Authorization: `Bearer ${accessToken}` },
				credentials: 'include',
			});

			if (!res.ok) throw new Error('Failed to fetch user');

			const user: User = await res.json();
			setAuth(user, accessToken, csrfToken);
			await goto(isNew ? '/onboarding' : '/explore');
		} catch {
			error = 'Failed to load your account. Please try again.';
			setTimeout(() => goto('/login'), 3000);
		}
	});
</script>

<svelte:head>
	<title>Signing in… — Yapper</title>
</svelte:head>

<div class="callback-shell">
	{#if error}
		<p class="error">{error}</p>
	{:else}
		<div class="loader" aria-label="Signing you in…"></div>
		<p class="caption">Signing you in…</p>
	{/if}
</div>

<style>
	.callback-shell {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		gap: 1rem;
		background: var(--color-bg-base);
	}

	.loader {
		width: 40px;
		height: 40px;
		border: 3px solid var(--color-border);
		border-top-color: var(--color-brand);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }

	.caption {
		color: var(--color-text-secondary);
		font-size: 0.9375rem;
	}

	.error {
		color: #fca5a5;
		font-size: 0.9375rem;
	}
</style>
