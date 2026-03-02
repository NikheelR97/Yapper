<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { authStore } from '$stores/auth.js';

	onMount(() => {
		// If already logged in, redirect to app
		if (get(authStore).user) {
			goto('/explore');
		}
	});
</script>

<slot />

<style>
	:global(body) {
		background: var(--color-bg-base);
	}
</style>
