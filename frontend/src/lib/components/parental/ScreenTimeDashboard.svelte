<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';

	export let childId: string;
	export let childName: string = 'Your child';

	type Period = 'today' | 'week' | 'month';

	let period: Period = 'today';
	let limitMinutes = 180;
	let bedtimeStart = '22:00';
	let bedtimeEnd = '07:00';
	let editingBedtime = false;
	let saving = false;

	interface AppUsage {
		appName: string;
		icon: string;
		minutes: number;
	}

	interface WeeklyData {
		day: string;
		yapperMinutes: number;
		otherMinutes: number;
	}

	let totalMinutesToday = 0;
	let appBreakdown: AppUsage[] = [];
	let weeklyData: WeeklyData[] = [];
	let loading = true;

	onMount(async () => {
		await loadData();
	});

	$: if (childId) loadData();

	async function loadData() {
		loading = true;
		try {
			const data = await api.get<{
				totalMinutesToday: number;
				limitMinutes: number;
				appBreakdown: AppUsage[];
				weeklyData: WeeklyData[];
				bedtimeStart: string | null;
				bedtimeEnd: string | null;
			}>(`/api/v1/parental/children/${childId}/screentime?period=${period}`);

			totalMinutesToday = data.totalMinutesToday;
			limitMinutes = data.limitMinutes;
			appBreakdown = data.appBreakdown;
			weeklyData = data.weeklyData;
			if (data.bedtimeStart) bedtimeStart = data.bedtimeStart;
			if (data.bedtimeEnd) bedtimeEnd = data.bedtimeEnd;
		} catch {
			// Use mock data for dev
			totalMinutesToday = 154;
			limitMinutes = 180;
			appBreakdown = [
				{ appName: 'Yapper', icon: '🟣', minutes: 72 },
				{ appName: 'TikTok', icon: '⬛', minutes: 45 },
				{ appName: 'Instagram', icon: '🔷', minutes: 22 },
				{ appName: 'YouTube', icon: '🔴', minutes: 15 },
			];
			weeklyData = [
				{ day: 'Mon', yapperMinutes: 60, otherMinutes: 40 },
				{ day: 'Tue', yapperMinutes: 90, otherMinutes: 60 },
				{ day: 'Wed', yapperMinutes: 45, otherMinutes: 80 },
				{ day: 'Thu', yapperMinutes: 120, otherMinutes: 50 },
				{ day: 'Fri', yapperMinutes: 154, otherMinutes: 82 },
				{ day: 'Sat', yapperMinutes: 200, otherMinutes: 120 },
				{ day: 'Sun', yapperMinutes: 0, otherMinutes: 0 },
			];
		} finally {
			loading = false;
		}
	}

	async function saveSettings() {
		saving = true;
		try {
			await api.patch(`/api/v1/parental/children/${childId}/screentime`, {
				limitMinutes,
				bedtimeStart,
				bedtimeEnd,
			});
			toast.success('Screen time settings saved!');
			editingBedtime = false;
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to save');
		} finally {
			saving = false;
		}
	}

	function formatTime(minutes: number): string {
		const h = Math.floor(minutes / 60);
		const m = minutes % 60;
		if (h === 0) return `${m}m`;
		return m === 0 ? `${h}h` : `${h}h ${m}m`;
	}

	$: usagePercent = limitMinutes > 0 ? Math.min(100, (totalMinutesToday / limitMinutes) * 100) : 0;
	$: isWarning = usagePercent >= 80 && usagePercent < 100;
	$: isOver = usagePercent >= 100;
	$: remaining = Math.max(0, limitMinutes - totalMinutesToday);

	// Chart dimensions
	const CHART_H = 120;
	const CHART_W = 400;
	$: maxVal = Math.max(...weeklyData.map((d) => d.yapperMinutes + d.otherMinutes), 1);
	$: barWidth = weeklyData.length > 0 ? Math.floor((CHART_W - weeklyData.length * 4) / weeklyData.length) : 40;
</script>

<div class="screen-time">
	<div class="header-row">
		<h2 class="page-title">{childName}'s Screen Time</h2>
		<div class="period-tabs">
			{#each (['today', 'week', 'month'] as Period[]) as p}
				<button class="period-tab" class:active={period === p} on:click={() => { period = p; loadData(); }}>
					{p.charAt(0).toUpperCase() + p.slice(1)}
				</button>
			{/each}
		</div>
	</div>

	{#if loading}
		<div class="loading">Loading screen time data…</div>
	{:else}
		<!-- Today summary -->
		<div class="summary-card">
			<div class="summary-header">
				<div class="summary-time">{formatTime(totalMinutesToday)} total</div>
				{#if isOver}
					<span class="alert-badge danger">Daily limit reached</span>
				{:else if isWarning}
					<span class="alert-badge warning">⚠ {formatTime(remaining)} remaining</span>
				{/if}
			</div>
			<div class="progress-track">
				<div
					class="progress-fill"
					class:warning={isWarning}
					class:danger={isOver}
					style="width: {usagePercent}%"
				></div>
			</div>
			<div class="summary-sub">Daily limit: {formatTime(limitMinutes)}</div>
		</div>

		<!-- Per-app breakdown -->
		<div class="section-card">
			<h3 class="section-heading">Per App Breakdown</h3>
			<div class="app-list">
				{#each appBreakdown as app}
					{@const pct = Math.round((app.minutes / Math.max(1, totalMinutesToday)) * 100)}
					<div class="app-row">
						<span class="app-icon">{app.icon}</span>
						<span class="app-name">{app.appName}</span>
						<span class="app-time">{formatTime(app.minutes)}</span>
						<div class="app-bar-track">
							<div
								class="app-bar-fill"
								class:yapper-bar={app.appName === 'Yapper'}
								style="width: {pct}%"
							></div>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- Weekly chart -->
		<div class="section-card">
			<h3 class="section-heading">Weekly Chart</h3>
			<div class="chart-container">
				<svg viewBox="0 0 {CHART_W} {CHART_H + 24}" class="chart-svg">
					{#each weeklyData as d, i}
						{@const x = i * (barWidth + 4)}
						{@const totalH = ((d.yapperMinutes + d.otherMinutes) / maxVal) * CHART_H}
						{@const yapperH = (d.yapperMinutes / maxVal) * CHART_H}
						{@const otherH = totalH - yapperH}

						<!-- Other apps bar (bottom) -->
						<rect
							x={x}
							y={CHART_H - totalH}
							width={barWidth}
							height={otherH}
							fill="#4b5563"
							rx="3"
						/>
						<!-- Yapper bar (top) -->
						<rect
							x={x}
							y={CHART_H - yapperH}
							width={barWidth}
							height={yapperH}
							fill="#7c3aed"
							rx="3"
						/>
						<!-- Day label -->
						<text
							x={x + barWidth / 2}
							y={CHART_H + 16}
							text-anchor="middle"
							fill="#6b7280"
							font-size="10"
						>{d.day}</text>
					{/each}
				</svg>
				<div class="chart-legend">
					<div class="legend-item">
						<span class="legend-dot yapper"></span> Yapper
					</div>
					<div class="legend-item">
						<span class="legend-dot other"></span> Other apps
					</div>
				</div>
			</div>
		</div>

		<!-- Daily limit settings -->
		<div class="section-card">
			<h3 class="section-heading">Daily Limit Settings</h3>

			<div class="limit-row">
				<label class="limit-label" for="limitSlider">
					Total daily limit: <strong>{formatTime(limitMinutes)}</strong>
				</label>
				<input
					id="limitSlider"
					type="range"
					min="30"
					max="480"
					step="30"
					bind:value={limitMinutes}
					class="slider"
				/>
				<div class="slider-labels">
					<span>30m</span>
					<span>8h</span>
				</div>
			</div>

			<div class="bedtime-row">
				<span class="bedtime-label">Bedtime block:</span>
				{#if editingBedtime}
					<div class="bedtime-edit">
						<input type="time" bind:value={bedtimeStart} class="time-input" />
						<span>→</span>
						<input type="time" bind:value={bedtimeEnd} class="time-input" />
					</div>
				{:else}
					<span class="bedtime-value">{bedtimeStart} → {bedtimeEnd}</span>
					<button class="edit-link" on:click={() => (editingBedtime = true)}>Edit</button>
				{/if}
			</div>

			<button class="save-btn" on:click={saveSettings} disabled={saving}>
				{saving ? 'Saving…' : 'Save Settings'}
			</button>
		</div>
	{/if}
</div>

<style>
	.screen-time {
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 20px;
		overflow-y: auto;
	}

	.header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
	}

	.page-title {
		font-size: 22px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
	}

	.period-tabs {
		display: flex;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 8px;
		overflow: hidden;
	}

	.period-tab {
		padding: 7px 16px;
		background: none;
		border: none;
		color: #9ca3af;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: background 100ms, color 100ms;
	}

	.period-tab.active {
		background: rgba(124, 58, 237, 0.2);
		color: #a78bfa;
	}

	.loading {
		font-size: 14px;
		color: #6b7280;
		text-align: center;
		padding: 32px;
	}

	/* Summary card */
	.summary-card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 14px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.summary-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}

	.summary-time {
		font-size: 28px;
		font-weight: 800;
		color: #f9fafb;
	}

	.alert-badge {
		font-size: 12px;
		font-weight: 700;
		padding: 4px 12px;
		border-radius: 20px;
	}

	.alert-badge.warning {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}

	.alert-badge.danger {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.progress-track {
		height: 12px;
		background: rgba(255, 255, 255, 0.07);
		border-radius: 6px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: #7c3aed;
		border-radius: 6px;
		transition: width 600ms ease;
	}

	.progress-fill.warning {
		background: #f59e0b;
	}

	.progress-fill.danger {
		background: #ef4444;
	}

	.summary-sub {
		font-size: 12px;
		color: #6b7280;
	}

	/* Section cards */
	.section-card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 14px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.section-heading {
		font-size: 13px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	/* App list */
	.app-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.app-row {
		display: grid;
		grid-template-columns: 24px 120px 60px 1fr;
		align-items: center;
		gap: 10px;
	}

	.app-icon {
		font-size: 18px;
	}

	.app-name {
		font-size: 14px;
		color: #d1d5db;
		white-space: nowrap;
	}

	.app-time {
		font-size: 13px;
		color: #9ca3af;
		text-align: right;
	}

	.app-bar-track {
		height: 6px;
		background: rgba(255, 255, 255, 0.07);
		border-radius: 3px;
		overflow: hidden;
	}

	.app-bar-fill {
		height: 100%;
		background: #4b5563;
		border-radius: 3px;
		transition: width 400ms;
	}

	.app-bar-fill.yapper-bar {
		background: #7c3aed;
	}

	/* Chart */
	.chart-container {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.chart-svg {
		width: 100%;
		overflow: visible;
	}

	.chart-legend {
		display: flex;
		gap: 16px;
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		color: #9ca3af;
	}

	.legend-dot {
		width: 10px;
		height: 10px;
		border-radius: 2px;
	}

	.legend-dot.yapper {
		background: #7c3aed;
	}

	.legend-dot.other {
		background: #4b5563;
	}

	/* Limit settings */
	.limit-row {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.limit-label {
		font-size: 14px;
		color: #d1d5db;
	}

	.slider {
		width: 100%;
		accent-color: #7c3aed;
		cursor: pointer;
	}

	.slider-labels {
		display: flex;
		justify-content: space-between;
		font-size: 11px;
		color: #6b7280;
	}

	.bedtime-row {
		display: flex;
		align-items: center;
		gap: 12px;
		flex-wrap: wrap;
	}

	.bedtime-label {
		font-size: 14px;
		color: #d1d5db;
	}

	.bedtime-value {
		font-size: 14px;
		color: #f9fafb;
		font-weight: 600;
	}

	.edit-link {
		background: none;
		border: none;
		color: #7c3aed;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		padding: 0;
		transition: color 150ms;
	}

	.edit-link:hover {
		color: #a78bfa;
	}

	.bedtime-edit {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.time-input {
		padding: 6px 10px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 6px;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		outline: none;
	}

	.save-btn {
		padding: 10px 20px;
		background: #7c3aed;
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 14px;
		font-weight: 700;
		cursor: pointer;
		align-self: flex-start;
		transition: opacity 150ms;
	}

	.save-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
