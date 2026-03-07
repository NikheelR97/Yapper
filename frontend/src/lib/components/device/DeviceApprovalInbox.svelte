<script lang="ts">
	import type { AuthDevice } from '$stores/auth.js';

	export let pendingDevices: AuthDevice[] = [];
	export let approving = new Set<string>();
	export let onApprove: (deviceId: string) => Promise<void>;
</script>

{#if pendingDevices.length > 0}
	<div class="inbox">
		<div>
			<p class="title">Pending Device Approvals</p>
			<p class="caption">Approve these devices to unlock encrypted chat on them.</p>
		</div>
		<div class="items">
			{#each pendingDevices as device (device.id)}
				<div class="item">
					<div>
						<p class="name">{device.label}</p>
						<p class="meta">{device.platform} · #{device.signalDeviceId}</p>
					</div>
					<button
						type="button"
						class="approve-btn"
						on:click={() => void onApprove(device.id)}
						disabled={approving.has(device.id)}
					>
						{approving.has(device.id) ? 'Approving…' : 'Approve'}
					</button>
				</div>
			{/each}
		</div>
	</div>
{/if}

<style>
	.inbox {
		display: grid;
		gap: 0.75rem;
		padding: 0.9rem 1rem;
		border-bottom: 1px solid rgba(249, 115, 22, 0.18);
		background: linear-gradient(90deg, rgba(249, 115, 22, 0.12), rgba(168, 85, 247, 0.08));
	}

	.title {
		margin: 0;
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--color-text-primary);
	}

	.caption {
		margin: 0.2rem 0 0;
		font-size: 0.78rem;
		color: var(--color-text-secondary);
	}

	.items {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.item {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.65rem 0.8rem;
		border-radius: 14px;
		background: rgba(10, 10, 16, 0.55);
		border: 1px solid rgba(255, 255, 255, 0.06);
	}

	.name,
	.meta {
		margin: 0;
	}

	.name {
		font-size: 0.86rem;
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.meta {
		font-size: 0.76rem;
		color: var(--color-text-muted);
	}

	.approve-btn {
		border: none;
		border-radius: 999px;
		padding: 0.55rem 0.8rem;
		background: #7c3aed;
		color: #fff;
		font-size: 0.8rem;
		font-weight: 700;
		cursor: pointer;
	}

	.approve-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
