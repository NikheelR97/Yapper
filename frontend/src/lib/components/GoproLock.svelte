<script lang="ts">
	import { goto } from '$app/navigation';

	export let description: string = 'This is a GoPro exclusive feature.';
	export let feature: string = 'GoPro Feature';
</script>

<div class="gopro-overlay" role="region" aria-label="Premium feature locked">
	<div class="overlay-blur"></div>
	<div class="lock-card">
		<span class="rocket-icon">🚀</span>
		<h3 class="lock-title">{feature}</h3>
		<p class="lock-desc">{description}</p>
		<button class="learn-more-btn" on:click={() => goto('/settings?tab=premium')}>
			Learn More
		</button>
	</div>
</div>

<style>
	.gopro-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.overlay-blur {
		position: absolute;
		inset: 0;
		backdrop-filter: blur(4px) brightness(0.5);
		background: rgba(10, 10, 15, 0.3);
	}

	.lock-card {
		position: relative;
		z-index: 1;
		background: rgba(22, 23, 30, 0.9);
		border: 1px solid rgba(255, 255, 255, 0.1);
		backdrop-filter: blur(12px);
		border-radius: 16px;
		padding: 24px;
		max-width: 280px;
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
	}

	.rocket-icon {
		font-size: 32px;
	}

	.lock-title {
		font-size: 18px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0;
	}

	.lock-desc {
		font-size: 14px;
		color: #9ca3af;
		margin: 0;
		line-height: 1.5;
	}

	.learn-more-btn {
		padding: 9px 20px;
		background: rgba(124, 58, 237, 0.15);
		color: #a78bfa;
		border: 1px solid rgba(124, 58, 237, 0.3);
		border-radius: 8px;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		margin-top: 4px;
		transition: background 150ms;
	}

	.learn-more-btn:hover {
		background: rgba(124, 58, 237, 0.3);
	}
</style>
