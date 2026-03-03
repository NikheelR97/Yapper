<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { get } from 'svelte/store';
	import { fetchChannels } from '$stores/servers.js';

	let error = false;

	onMount(async () => {
		const serverId = get(page).params.id;
		try {
			const channels = await fetchChannels(serverId);
			const first = channels.sort((a, b) => a.position - b.position)[0];
			if (first) {
				goto(`/servers/${serverId}/channels/${first.id}`, { replaceState: true });
			} else {
				error = true;
			}
		} catch {
			error = true;
		}
	});
</script>

{#if error}
	<div class="empty">
		<p>No channels found in this server.</p>
	</div>
{:else}
	<div class="loading" aria-label="Loading channel…"></div>
{/if}

<style>
	.loading {
		display: flex;
		flex: 1;
		background: var(--color-bg-base);
	}

	.empty {
		display: flex;
		flex: 1;
		align-items: center;
		justify-content: center;
		background: var(--color-bg-base);
		color: var(--color-text-secondary);
		font-size: 0.875rem;
	}
</style>
