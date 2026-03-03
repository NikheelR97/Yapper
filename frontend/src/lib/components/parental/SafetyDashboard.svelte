<script lang="ts">
	import PendingAlerts from './PendingAlerts.svelte';
	import SafetyFeed from './SafetyFeed.svelte';
	import ActivitySnapshot from './ActivitySnapshot.svelte';
	import type { PendingAlert, SafetyEvent, ChildActivity } from '$stores/parental.js';

	export let alerts: PendingAlert[] = [];
	export let feed: SafetyEvent[] = [];
	export let activity: ChildActivity | null = null;
	export let childName: string = 'Your child';
</script>

<div class="safety-dashboard">
	<div class="dashboard-header">
		<h2 class="child-name">{childName}'s Safety Dashboard</h2>
	</div>

	<div class="dashboard-grid">
		<div class="col-alerts">
			<PendingAlerts {alerts} />
		</div>
		<div class="col-feed">
			<SafetyFeed events={feed} />
		</div>
		<div class="col-activity">
			<ActivitySnapshot {activity} />
		</div>
	</div>
</div>

<style>
	.safety-dashboard {
		flex: 1;
		padding: 24px;
		overflow-y: auto;
		min-height: 0;
	}

	.dashboard-header {
		margin-bottom: 24px;
	}

	.child-name {
		font-size: 22px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
	}

	.dashboard-grid {
		display: grid;
		grid-template-columns: 1fr 1.2fr 0.8fr;
		gap: 20px;
		align-items: start;
	}

	@media (max-width: 1024px) {
		.dashboard-grid {
			grid-template-columns: 1fr 1fr;
		}

		.col-activity {
			grid-column: 1 / -1;
		}
	}

	@media (max-width: 700px) {
		.dashboard-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
