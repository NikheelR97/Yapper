<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { authStore } from '$stores/auth.js';

	onMount(() => {
		// Redirect based on auth state
		if ($authStore.user) {
			goto('/explore');
		} else {
			goto('/login');
		}
	});
</script>

<!-- Blank splash while redirect happens -->
<div class="splash">
	<div class="loader" aria-label="Loading…"></div>
</div>

<style>
	.splash {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		background: var(--color-bg-base);
	}

	.loader {
		width: 32px;
		height: 32px;
		border: 3px solid var(--color-border);
		border-top-color: var(--color-brand);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
