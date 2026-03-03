<script lang="ts">
	import { parentalStore } from '$stores/parental.js';
	import ScreenTimeDashboard from '$components/parental/ScreenTimeDashboard.svelte';

	$: selectedId = $parentalStore.selectedChildId;
	$: selectedChild = $parentalStore.children.find((c) => c.id === selectedId) ?? null;
</script>

<svelte:head>
	<title>Screen Time — Yapper</title>
</svelte:head>

{#if !selectedChild || !selectedId}
	<div class="no-child">
		<p>Select a child from the sidebar to view their screen time.</p>
	</div>
{:else}
	<ScreenTimeDashboard childId={selectedId} childName={selectedChild.displayName} />
{/if}

<style>
	.no-child {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		font-size: 14px;
		color: #6b7280;
	}
</style>
