<script lang="ts">
	import { onMount } from 'svelte';
	import { parentalStore, loadFeed, loadActivity } from '$stores/parental.js';
	import SafetyDashboard from '$components/parental/SafetyDashboard.svelte';

	$: selectedId = $parentalStore.selectedChildId;
	$: selectedChild = $parentalStore.children.find((c) => c.id === selectedId) ?? null;
	$: alerts = $parentalStore.alerts;
	$: feed = $parentalStore.feed;
	$: activity = selectedId ? $parentalStore.activity[selectedId] ?? null : null;

	onMount(async () => {
		if (selectedId) {
			await Promise.all([loadFeed(selectedId), loadActivity(selectedId)]);
		}
	});

	$: if (selectedId) {
		loadFeed(selectedId);
		loadActivity(selectedId);
	}
</script>

<svelte:head>
	<title>Safety Dashboard — Yapper</title>
</svelte:head>

{#if !selectedChild}
	<div class="no-child">
		<span class="no-child-icon">👶</span>
		<h2>No child accounts yet</h2>
		<p>Set up a child account to start monitoring their activity.</p>
		<a class="btn-primary" href="/parent/children/setup">Add Account →</a>
	</div>
{:else}
	<SafetyDashboard
		{alerts}
		{feed}
		{activity}
		childName={selectedChild.display_name}
	/>
{/if}

<style>
	.no-child {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		height: 100%;
		padding: 48px;
		text-align: center;
	}

	.no-child-icon {
		font-size: 56px;
	}

	.no-child h2 {
		font-size: 22px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
	}

	.no-child p {
		font-size: 14px;
		color: #9ca3af;
		margin: 0;
	}

	.btn-primary {
		display: inline-block;
		margin-top: 8px;
		padding: 12px 24px;
		background: #7c3aed;
		color: white;
		border-radius: 10px;
		font-size: 15px;
		font-weight: 700;
		text-decoration: none;
		transition: opacity 150ms;
	}

	.btn-primary:hover {
		opacity: 0.85;
	}
</style>
