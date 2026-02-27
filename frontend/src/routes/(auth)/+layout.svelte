<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { authStore } from '$stores/auth.js';
</script>

<!-- Redirect to app if already logged in -->
{#if false}<!-- handled in onMount -->{/if}

<slot />

<style>
	:global(body) {
		background: var(--color-bg-base);
	}
</style>
