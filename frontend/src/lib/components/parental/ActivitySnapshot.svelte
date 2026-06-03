<script lang="ts">
	import type { ChildActivity } from '$stores/parental.js';

	export let activity: ChildActivity | null = null;

	function formatTime(minutes: number): string {
		const h = Math.floor(minutes / 60);
		const m = minutes % 60;
		if (h === 0) return `${m}m`;
		return m === 0 ? `${h}h` : `${h}h ${m}m`;
	}

	$: usagePercent = activity && activity.limitMinutes
		? Math.min(100, (activity.totalMinutesToday / activity.limitMinutes) * 100)
		: null;

	$: isWarning = usagePercent !== null && usagePercent >= 80;
	$: isOver = usagePercent !== null && usagePercent >= 100;
</script>

<div class="activity-snapshot">
	<h3 class="section-title">Activity Today</h3>

	{#if !activity}
		<div class="empty">No activity data available.</div>
	{:else}
		<div class="stat-cards">
			<!-- Screen time -->
			<div class="stat-card">
				<div class="stat-header">
					<span class="stat-label">Screen Time</span>
					{#if isOver}
						<span class="badge danger">Limit reached</span>
					{:else if isWarning}
						<span class="badge warning">Near limit</span>
					{/if}
				</div>
				<div class="stat-value">{formatTime(activity.totalMinutesToday)}</div>
				{#if activity.limitMinutes}
					<div class="stat-sub">of {formatTime(activity.limitMinutes)} limit</div>
					<div class="progress-track">
						<div
							class="progress-fill"
							class:warning={isWarning}
							class:danger={isOver}
							style="transform: scaleX({Math.min(100, usagePercent ?? 0) / 100})"
						></div>
					</div>
				{/if}
			</div>

			<!-- Friends + Servers -->
			<div class="mini-stats">
				<div class="mini-card">
					<span class="mini-icon">👥</span>
					<span class="mini-value">{activity.friendCount}</span>
					<span class="mini-label">Friends</span>
				</div>
				<div class="mini-card">
					<span class="mini-icon">🌐</span>
					<span class="mini-value">{activity.serverCount}</span>
					<span class="mini-label">Servers</span>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.activity-snapshot {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.section-title {
		font-size: 14px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.stat-cards {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.stat-card {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		padding: 16px;
	}

	.stat-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 8px;
	}

	.stat-label {
		font-size: 12px;
		color: var(--color-text-secondary);
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.badge {
		font-size: 10px;
		font-weight: 700;
		padding: 2px 7px;
		border-radius: 20px;
	}

	.badge.warning {
		background: rgba(245, 158, 11, 0.15);
		color: var(--color-warning-text);
	}

	.badge.danger {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-error-text);
	}

	.stat-value {
		font-size: 28px;
		font-weight: 800;
		color: var(--color-text-primary);
	}

	.stat-sub {
		font-size: 12px;
		color: var(--color-text-muted);
		margin: 2px 0 10px;
	}

	.progress-track {
		height: 6px;
		background: var(--color-border);
		border-radius: 3px;
		overflow: hidden;
	}

	.progress-fill {
		width: 100%;
		height: 100%;
		background: var(--color-brand);
		border-radius: 3px;
		transform-origin: left;
		/* Animate transform, not width, to keep the meter off the layout path. */
		transition: transform 600ms ease;
	}

	.progress-fill.warning {
		background: var(--color-warning);
	}

	.progress-fill.danger {
		background: var(--color-error);
	}

	.mini-stats {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}

	.mini-card {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		padding: 14px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
	}

	.mini-icon {
		font-size: 20px;
	}

	.mini-value {
		font-size: 22px;
		font-weight: 800;
		color: var(--color-text-primary);
	}

	.mini-label {
		font-size: 11px;
		color: var(--color-text-muted);
	}

	.empty {
		font-size: 13px;
		color: var(--color-text-muted);
		padding: 20px;
		text-align: center;
	}
</style>
