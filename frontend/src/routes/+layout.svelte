<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';

	// Signal key setup happens in the (app) layout after auth.
	// Nothing to pre-load here since we use pure Web Crypto (no WASM).
	onMount(() => {});
</script>

<slot />
