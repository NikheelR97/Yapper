<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { approveAlert, declineAlert, type PendingAlert } from '$stores/parental.js';
	import { toast } from '$stores/toast.js';

	export let alerts: PendingAlert[] = [];

	const dispatch = createEventDispatcher<{ review: PendingAlert }>();

	const alertIcons: Record<string, string> = {
		friend_request: '👤',
		server_join: '🌐',
		dm_request: '💬',
	};

	async function handleApprove(alert: PendingAlert) {
		try {
			await approveAlert(alert.id, alert.type as 'friend_request' | 'server_join');
			toast.success('Approved');
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to approve');
		}
	}

	async function handleDecline(alert: PendingAlert) {
		try {
			await declineAlert(alert.id, alert.type as 'friend_request' | 'server_join');
			toast.info('Declined');
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to decline');
		}
	}

	function relativeTime(iso: string): string {
		const diff = Date.now() - new Date(iso).getTime();
		const min = Math.floor(diff / 60000);
		if (min < 1) return 'just now';
		if (min < 60) return `${min}m ago`;
		const hr = Math.floor(min / 60);
		if (hr < 24) return `${hr}h ago`;
		return `${Math.floor(hr / 24)}d ago`;
	}
</script>

<div class="pending-alerts">
	<div class="section-header">
		<h3 class="section-title">Pending Alerts</h3>
		{#if alerts.length > 0}
			<span class="badge">{alerts.length}</span>
		{/if}
	</div>

	{#if alerts.length === 0}
		<div class="empty-state">
			<span class="empty-icon">✓</span>
			<p>All clear — nothing needs your approval.</p>
		</div>
	{:else}
		<div class="alerts-list">
			{#each alerts as alert}
				<div class="alert-card">
					<div class="alert-top">
						<span class="alert-icon">{alertIcons[alert.type] ?? '⚠'}</span>
						<div class="alert-body">
							<p class="alert-desc">{alert.description}</p>
							<span class="alert-meta">{alert.childName} · {relativeTime(alert.createdAt)}</span>
						</div>
					</div>
					<div class="alert-actions">
						<button class="btn-approve" on:click={() => handleApprove(alert)}>Approve</button>
						<button class="btn-decline" on:click={() => handleDecline(alert)}>Decline</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.pending-alerts {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.section-title {
		font-size: 14px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.badge {
		background: #f59e0b;
		color: #0a0a0f;
		font-size: 11px;
		font-weight: 700;
		padding: 2px 7px;
		border-radius: 20px;
	}

	.alerts-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.alert-card {
		background: rgba(245, 158, 11, 0.06);
		border: 1px solid rgba(245, 158, 11, 0.25);
		border-left: 3px solid #f59e0b;
		border-radius: 12px;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.alert-top {
		display: flex;
		gap: 10px;
	}

	.alert-icon {
		font-size: 20px;
		flex-shrink: 0;
	}

	.alert-body {
		flex: 1;
		min-width: 0;
	}

	.alert-desc {
		font-size: 13px;
		color: #f9fafb;
		margin: 0 0 4px;
		line-height: 1.4;
	}

	.alert-meta {
		font-size: 11px;
		color: #9ca3af;
	}

	.alert-actions {
		display: flex;
		gap: 8px;
	}

	.btn-approve {
		flex: 1;
		padding: 8px;
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
		border: 1px solid rgba(34, 197, 94, 0.3);
		border-radius: 8px;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: background 150ms;
	}

	.btn-approve:hover {
		background: rgba(34, 197, 94, 0.3);
	}

	.btn-decline {
		flex: 1;
		padding: 8px;
		background: rgba(107, 114, 128, 0.1);
		color: #9ca3af;
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: background 150ms;
	}

	.btn-decline:hover {
		background: rgba(239, 68, 68, 0.1);
		color: #ef4444;
		border-color: rgba(239, 68, 68, 0.3);
	}

	.empty-state {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 20px;
		background: rgba(34, 197, 94, 0.06);
		border: 1px solid rgba(34, 197, 94, 0.15);
		border-radius: 12px;
		color: #22c55e;
		font-size: 13px;
	}

	.empty-icon {
		font-size: 20px;
	}
</style>
