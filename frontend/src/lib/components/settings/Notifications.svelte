<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';

	let pushEnabled = true;
	let notifyDMs = true;
	let notifyMentions = true;
	let notifyFriendRequests = true;
	let notifyServerActivity = false;
	let notifyYapRecordings = true;

	let dndEnabled = false;
	let dndStart = '22:00';
	let dndEnd = '07:00';

	let saving = false;

	onMount(async () => {
		try {
			const res = await api.get<{
				pushEnabled: boolean;
				notifyDMs: boolean;
				notifyMentions: boolean;
				notifyFriendRequests: boolean;
				notifyServerActivity: boolean;
				notifyYapRecordings: boolean;
				dndEnabled: boolean;
				dndStart: string | null;
				dndEnd: string | null;
			}>('/api/v1/users/me/notifications');
			pushEnabled = res.pushEnabled;
			notifyDMs = res.notifyDMs;
			notifyMentions = res.notifyMentions;
			notifyFriendRequests = res.notifyFriendRequests;
			notifyServerActivity = res.notifyServerActivity;
			notifyYapRecordings = res.notifyYapRecordings;
			dndEnabled = res.dndEnabled;
			// Postgres TIME is "HH:MM:SS" — trim to "HH:MM" for <input type="time">
			if (res.dndStart) dndStart = res.dndStart.slice(0, 5);
			if (res.dndEnd) dndEnd = res.dndEnd.slice(0, 5);
		} catch {
			// Non-fatal — defaults remain
		}
	});

	async function save() {
		saving = true;
		try {
			await api.patch('/api/v1/users/me/notifications', {
				pushEnabled,
				notifyDMs,
				notifyMentions,
				notifyFriendRequests,
				notifyServerActivity,
				notifyYapRecordings,
				dndEnabled,
				dndStart: dndEnabled ? dndStart : null,
				dndEnd: dndEnabled ? dndEnd : null,
			});
			toast.success('Notification settings saved!');
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to save');
		} finally {
			saving = false;
		}
	}

	const toggles = [
		{ key: 'notifyDMs', label: 'Direct Messages', desc: 'When you receive a new DM' },
		{ key: 'notifyMentions', label: '@Mentions', desc: 'When someone mentions you in a channel' },
		{ key: 'notifyFriendRequests', label: 'Friend Requests', desc: 'New friend request notifications' },
		{ key: 'notifyServerActivity', label: 'Server Activity', desc: 'New messages in your servers (excluding DMs)' },
		{ key: 'notifyYapRecordings', label: 'Yap Recordings', desc: 'When someone sends you a voice Yap' },
	];

	const values: Record<string, boolean> = {
		notifyDMs, notifyMentions, notifyFriendRequests, notifyServerActivity, notifyYapRecordings
	};
</script>

<div class="notifs-section">
	<h2 class="section-title">Notifications</h2>

	<!-- Master toggle -->
	<div class="setting-block">
		<div class="toggle-row">
			<div>
				<h3 class="block-title">Push Notifications</h3>
				<p class="block-desc">Receive notifications when the app is closed or backgrounded.</p>
			</div>
			<label class="toggle-switch">
				<input type="checkbox" bind:checked={pushEnabled} />
				<span class="toggle-track"></span>
			</label>
		</div>
	</div>

	<!-- Per-type toggles -->
	{#if pushEnabled}
		<div class="setting-block">
			<h3 class="block-title">Notify me about…</h3>
			<div class="toggle-list">
				<div class="toggle-row">
					<div>
						<span class="toggle-label">Direct Messages</span>
						<p class="toggle-desc">When you receive a new DM</p>
					</div>
					<label class="toggle-switch">
						<input type="checkbox" bind:checked={notifyDMs} />
						<span class="toggle-track"></span>
					</label>
				</div>
				<div class="toggle-row">
					<div>
						<span class="toggle-label">@Mentions</span>
						<p class="toggle-desc">When someone mentions you in a channel</p>
					</div>
					<label class="toggle-switch">
						<input type="checkbox" bind:checked={notifyMentions} />
						<span class="toggle-track"></span>
					</label>
				</div>
				<div class="toggle-row">
					<div>
						<span class="toggle-label">Friend Requests</span>
						<p class="toggle-desc">New friend request notifications</p>
					</div>
					<label class="toggle-switch">
						<input type="checkbox" bind:checked={notifyFriendRequests} />
						<span class="toggle-track"></span>
					</label>
				</div>
				<div class="toggle-row">
					<div>
						<span class="toggle-label">Server Activity</span>
						<p class="toggle-desc">New messages in your servers</p>
					</div>
					<label class="toggle-switch">
						<input type="checkbox" bind:checked={notifyServerActivity} />
						<span class="toggle-track"></span>
					</label>
				</div>
				<div class="toggle-row">
					<div>
						<span class="toggle-label">Yap Recordings</span>
						<p class="toggle-desc">When someone sends you a voice Yap</p>
					</div>
					<label class="toggle-switch">
						<input type="checkbox" bind:checked={notifyYapRecordings} />
						<span class="toggle-track"></span>
					</label>
				</div>
			</div>
		</div>

		<!-- DND hours -->
		<div class="setting-block">
			<div class="toggle-row">
				<div>
					<h3 class="block-title">Do Not Disturb</h3>
					<p class="block-desc">Silence notifications during set hours.</p>
				</div>
				<label class="toggle-switch">
					<input type="checkbox" bind:checked={dndEnabled} />
					<span class="toggle-track"></span>
				</label>
			</div>
			{#if dndEnabled}
				<div class="time-row">
					<div class="time-field">
						<label class="time-label" for="dndStart">From</label>
						<input id="dndStart" type="time" class="time-input" bind:value={dndStart} />
					</div>
					<span class="time-sep">→</span>
					<div class="time-field">
						<label class="time-label" for="dndEnd">Until</label>
						<input id="dndEnd" type="time" class="time-input" bind:value={dndEnd} />
					</div>
				</div>
			{/if}
		</div>
	{/if}

	<button class="save-btn" on:click={save} disabled={saving}>
		{saving ? 'Saving…' : 'Save Notification Settings'}
	</button>
</div>

<style>
	.notifs-section {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.section-title {
		font-size: 20px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
	}

	.setting-block {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 12px;
		padding: 18px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.block-title {
		font-size: 13px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.block-desc {
		font-size: 13px;
		color: #6b7280;
		margin: 4px 0 0;
	}

	.toggle-list {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.toggle-label {
		font-size: 14px;
		font-weight: 600;
		color: #f9fafb;
		display: block;
	}

	.toggle-desc {
		font-size: 12px;
		color: #6b7280;
		margin: 3px 0 0;
	}

	.toggle-switch {
		position: relative;
		display: inline-block;
		width: 44px;
		height: 24px;
		flex-shrink: 0;
	}

	.toggle-switch input {
		opacity: 0;
		width: 0;
		height: 0;
	}

	.toggle-track {
		position: absolute;
		inset: 0;
		background: #374151;
		border-radius: 12px;
		cursor: pointer;
		transition: background 200ms;
	}

	.toggle-track::after {
		content: '';
		position: absolute;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: white;
		top: 3px;
		left: 3px;
		transition: transform 200ms;
	}

	.toggle-switch input:checked + .toggle-track {
		background: #22c55e;
	}

	.toggle-switch input:checked + .toggle-track::after {
		transform: translateX(20px);
	}

	.time-row {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 4px;
	}

	.time-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.time-label {
		font-size: 11px;
		font-weight: 700;
		color: #6b7280;
		text-transform: uppercase;
	}

	.time-input {
		padding: 8px 12px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		outline: none;
		cursor: pointer;
	}

	.time-input:focus {
		border-color: #7c3aed;
	}

	.time-sep {
		color: #6b7280;
		font-size: 16px;
		margin-top: 20px;
	}

	.save-btn {
		padding: 12px 24px;
		background: #7c3aed;
		color: white;
		border: none;
		border-radius: 10px;
		font-size: 15px;
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
