<script lang="ts">
	import type { SafetyEvent } from '$stores/parental.js';

	export let events: SafetyEvent[] = [];

	const dotColors: Record<string, string> = {
		content_warning: '#f59e0b',
		new_friend: '#3b82f6',
		screen_time: '#f97316',
		server_join: '#7c3aed',
		friend_request: '#3b82f6',
	};

	const typeLabels: Record<string, string> = {
		content_warning: 'Content Warning',
		new_friend: 'New Friend',
		screen_time: 'Screen Time',
		server_join: 'Server Join',
		friend_request: 'Friend Request',
	};

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

<div class="safety-feed">
	<div class="feed-header">
		<h3 class="section-title">Safety Feed</h3>
	</div>

	{#if events.length === 0}
		<div class="empty-state">
			<p>No recent activity to review.</p>
		</div>
	{:else}
		<div class="timeline">
			{#each events as event}
				<div class="event-row" class:unread={!event.read}>
					<div class="dot-col">
						<div class="dot" style="background: {dotColors[event.type] ?? '#6b7280'}"></div>
						<div class="line"></div>
					</div>
					<div class="event-content">
						<span class="event-type" style="color: {dotColors[event.type] ?? '#9ca3af'}">
							{typeLabels[event.type] ?? event.type}
						</span>
						<p class="event-desc">{event.description}</p>
						<span class="event-time">{relativeTime(event.createdAt)}</span>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.safety-feed {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.feed-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.section-title {
		font-size: 14px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.timeline {
		display: flex;
		flex-direction: column;
	}

	.event-row {
		display: flex;
		gap: 12px;
		min-height: 48px;
	}

	.event-row:last-child .line {
		display: none;
	}

	.dot-col {
		display: flex;
		flex-direction: column;
		align-items: center;
		flex-shrink: 0;
		width: 16px;
	}

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
		margin-top: 4px;
	}

	.line {
		flex: 1;
		width: 2px;
		background: var(--color-border);
		margin-top: 4px;
	}

	.event-content {
		flex: 1;
		padding-bottom: 16px;
		min-width: 0;
	}

	.event-type {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.event-desc {
		font-size: 13px;
		color: var(--color-text-secondary);
		margin: 4px 0;
		line-height: 1.4;
	}

	.event-time {
		font-size: 11px;
		color: var(--color-text-muted);
	}

	.event-row.unread .event-content {
		background: var(--color-bg-elevated);
		border-radius: 8px;
		padding: 8px;
	}

	.empty-state {
		padding: 24px;
		text-align: center;
		color: var(--color-text-muted);
		font-size: 13px;
	}
</style>
