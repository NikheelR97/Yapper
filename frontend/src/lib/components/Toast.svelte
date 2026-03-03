<script lang="ts">
	import { toast, type Toast } from '$stores/toast.js';

	const icons: Record<string, string> = {
		success: '✓',
		error: '✕',
		warning: '⚠',
		info: 'ℹ',
	};

	const borderColors: Record<string, string> = {
		success: '#22c55e',
		error: '#ef4444',
		warning: '#f59e0b',
		info: '#3b82f6',
	};

	function dismiss(id: string) {
		toast.remove(id);
	}
</script>

<div class="toast-container" role="region" aria-label="Notifications">
	{#each $toast as t (t.id)}
		<div
			class="toast toast-{t.type}"
			style="border-left-color: {borderColors[t.type]}"
			role="alert"
			aria-live={t.type === 'error' ? 'assertive' : 'polite'}
		>
			<span class="toast-icon" style="color: {borderColors[t.type]}">{icons[t.type]}</span>
			<span class="toast-message">{t.message}</span>
			<button class="toast-close" on:click={() => dismiss(t.id)} aria-label="Dismiss">✕</button>
		</div>
	{/each}
</div>

<style>
	.toast-container {
		position: fixed;
		bottom: 24px;
		right: 24px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		z-index: 9999;
		pointer-events: none;
	}

	.toast {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 14px 18px;
		background: #16171e;
		border-left: 3px solid transparent;
		border-radius: 8px;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
		max-width: 360px;
		pointer-events: all;
		animation: slide-in 200ms ease-out;
	}

	.toast-icon {
		font-size: 15px;
		flex-shrink: 0;
		font-weight: 700;
	}

	.toast-message {
		flex: 1;
		font-size: 14px;
		color: #f9fafb;
		line-height: 1.4;
	}

	.toast-close {
		background: none;
		border: none;
		color: #6b7280;
		cursor: pointer;
		font-size: 12px;
		padding: 2px 4px;
		flex-shrink: 0;
		transition: color 150ms;
	}

	.toast-close:hover {
		color: #f9fafb;
	}

	@keyframes slide-in {
		from {
			opacity: 0;
			transform: translateX(100%);
		}
		to {
			opacity: 1;
			transform: translateX(0);
		}
	}
</style>
