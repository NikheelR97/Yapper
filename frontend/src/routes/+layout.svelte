<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';

	// Initialize Signal Protocol WASM on app load (background — ~200ms)
	onMount(async () => {
		// Signal WASM init happens here so it's ready when the user opens a chat
		// Non-blocking: the user can navigate while WASM loads
		try {
			const { initSignal } = await import('$signal/index.js');
			await initSignal();
		} catch (e) {
			console.error('Signal init failed:', e);
		}
	});
</script>

<slot />
