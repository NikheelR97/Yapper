<script lang="ts">
	import type { AuthDevice } from '$stores/auth.js';

	export let label: string;
	export let refreshing = false;
	export let onRefresh: () => Promise<void>;
	export let restoreDevices: AuthDevice[] = [];
	export let restoring = false;
	export let onRestore: (sourceDeviceId: string, pin: string) => Promise<void>;

	let sourceDeviceId = '';
	let restorePin = '';

	$: if (!sourceDeviceId && restoreDevices.length > 0) {
		sourceDeviceId = restoreDevices[0].id;
	}
</script>

<div class="gate-shell">
	<div class="gate-card">
		<p class="eyebrow">Device Approval Required</p>
		<h1>{label}</h1>
		<p class="body">
			This device is signed in, but encrypted chat stays locked until one of your trusted devices
			approves it or you restore an encrypted backup.
		</p>
		<button class="refresh-btn" type="button" on:click={() => void onRefresh()} disabled={refreshing}>
			{refreshing ? 'Refreshing...' : 'Refresh Status'}
		</button>

		{#if restoreDevices.length > 0}
			<div class="restore-panel">
				<p class="restore-title">Restore from encrypted backup</p>
				<label class="field">
					<span>Source device</span>
					<select bind:value={sourceDeviceId} disabled={restoring}>
						{#each restoreDevices as device (device.id)}
							<option value={device.id}>{device.label} · #{device.signalDeviceId}</option>
						{/each}
					</select>
				</label>
				<label class="field">
					<span>Backup PIN</span>
					<input
						bind:value={restorePin}
						type="password"
						autocomplete="current-password"
						placeholder="Enter backup PIN"
						disabled={restoring}
						on:keydown={(event) =>
							event.key === 'Enter' &&
							sourceDeviceId &&
							restorePin.trim() &&
							void onRestore(sourceDeviceId, restorePin)}
					/>
				</label>
				<button
					class="restore-btn"
					type="button"
					on:click={() => void onRestore(sourceDeviceId, restorePin)}
					disabled={restoring || !sourceDeviceId || !restorePin.trim()}
				>
					{restoring ? 'Restoring...' : 'Restore Backup'}
				</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.gate-shell {
		display: grid;
		place-items: center;
		flex: 1;
		padding: 2rem;
		background:
			radial-gradient(circle at top, rgba(168, 85, 247, 0.16), transparent 40%),
			linear-gradient(180deg, rgba(13, 13, 18, 0.98), rgba(8, 8, 12, 0.98));
	}

	.gate-card {
		width: min(100%, 560px);
		border: 1px solid rgba(239, 68, 68, 0.28);
		border-radius: 20px;
		padding: 2rem;
		background: rgba(17, 17, 24, 0.94);
		box-shadow: 0 32px 80px rgba(0, 0, 0, 0.35);
	}

	.eyebrow {
		margin: 0 0 0.5rem;
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: #f97316;
	}

	h1 {
		margin: 0 0 0.75rem;
		font-size: 1.8rem;
		font-weight: 800;
		color: var(--color-text-primary);
	}

	.body {
		margin: 0 0 1.5rem;
		color: var(--color-text-secondary);
		line-height: 1.6;
	}

	.refresh-btn {
		border: none;
		border-radius: 999px;
		padding: 0.75rem 1.1rem;
		font-size: 0.95rem;
		font-weight: 700;
		color: #fff;
		background: linear-gradient(135deg, #f97316, #ef4444);
		cursor: pointer;
	}

	.refresh-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.restore-panel {
		margin-top: 1.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.08);
		display: grid;
		gap: 0.85rem;
	}

	.restore-title {
		margin: 0;
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--color-text-primary);
	}

	.field {
		display: grid;
		gap: 0.4rem;
	}

	.field span {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--color-text-secondary);
	}

	.field select,
	.field input {
		width: 100%;
		border-radius: 12px;
		border: 1px solid rgba(255, 255, 255, 0.08);
		background: rgba(8, 8, 12, 0.82);
		color: var(--color-text-primary);
		padding: 0.75rem 0.85rem;
		font-size: 0.92rem;
	}

	.restore-btn {
		justify-self: start;
		border: none;
		border-radius: 999px;
		padding: 0.75rem 1rem;
		font-size: 0.9rem;
		font-weight: 700;
		color: #04110a;
		background: linear-gradient(135deg, #22c55e, #84cc16);
		cursor: pointer;
	}

	.restore-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
