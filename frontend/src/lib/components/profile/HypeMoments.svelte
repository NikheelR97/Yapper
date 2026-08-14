<script lang="ts">
	import Skeleton from '$components/Skeleton.svelte';
	import type { HypeMoment } from '$stores/profile.js';

	export let moments: HypeMoment[] = [];
	export let loading: boolean = false;

	const brightGradients = [
		'linear-gradient(135deg, #7c3aed, #db2777)',
		'linear-gradient(135deg, #059669, #0891b2)',
		'linear-gradient(135deg, #d97706, #dc2626)',
		'linear-gradient(135deg, #2563eb, #7c3aed)',
	];

	function getGradient(moment: HypeMoment, index: number): string {
		if (moment.gradientStart && moment.gradientEnd) {
			return `linear-gradient(135deg, ${moment.gradientStart}, ${moment.gradientEnd})`;
		}
		return brightGradients[index % brightGradients.length];
	}
</script>

<div class="hype-moments">
	<h2 class="section-title">Hype Moments</h2>

	{#if loading}
		<div class="moments-grid">
			{#each Array(6) as _}
				<div class="moment-skeleton">
					<Skeleton height="180px" />
				</div>
			{/each}
		</div>
	{:else if moments.length === 0}
		<div class="empty-state">
			<span class="empty-icon">⚡</span>
			<p>No Hype Moments yet.</p>
		</div>
	{:else}
		<div class="moments-grid">
			{#each moments as moment, i}
				{#if moment.type === 'media'}
					<div class="moment-card media-card">
						{#if moment.mediaUrl}
							<img src={moment.mediaUrl} alt="Hype moment" class="media-img" />
						{:else}
							<div class="media-placeholder"></div>
						{/if}
						<div class="media-overlay">
							<p class="media-caption">{moment.content}</p>
						</div>
					</div>
				{:else if moment.type === 'text'}
					<div class="moment-card text-card" style="background: {getGradient(moment, i)}">
						<span class="quote-mark">"</span>
						<p class="quote-text">{moment.content}</p>
					</div>
				{:else if moment.type === 'clip'}
					<div class="moment-card clip-card">
						{#if moment.mediaUrl}
							<video src={moment.mediaUrl} class="clip-thumb" muted preload="metadata"></video>
						{:else}
							<div class="media-placeholder"></div>
						{/if}
						<div class="play-overlay">
							<div class="play-btn">▶</div>
						</div>
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>

<style>
	.hype-moments {
		margin-top: 24px;
	}

	.section-title {
		font-size: 16px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0 0 16px;
	}

	.moments-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
	}

	.moment-skeleton {
		border-radius: 12px;
		overflow: hidden;
	}

	.moment-card {
		border-radius: 12px;
		overflow: hidden;
		aspect-ratio: 1;
		position: relative;
		cursor: pointer;
		transition: transform 150ms, box-shadow 150ms;
	}

	.moment-card:hover {
		transform: scale(1.02);
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
	}

	/* Media card */
	.media-card {
		background: var(--color-bg-elevated);
	}

	.media-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.media-placeholder {
		width: 100%;
		height: 100%;
		background: linear-gradient(135deg, #1a0533, #2e1065);
	}

	.media-overlay {
		position: absolute;
		inset: 0;
		background: linear-gradient(to top, rgba(0, 0, 0, 0.7), transparent);
		display: flex;
		align-items: flex-end;
		padding: 12px;
	}

	.media-caption {
		font-size: 13px;
		color: #f9fafb;
		margin: 0;
		line-height: 1.4;
	}

	/* Text/quote card */
	.text-card {
		display: flex;
		flex-direction: column;
		justify-content: center;
		padding: 20px;
	}

	.quote-mark {
		font-size: 48px;
		font-weight: 900;
		color: rgba(255, 255, 255, 0.3);
		line-height: 1;
		margin-bottom: 4px;
	}

	.quote-text {
		font-size: 15px;
		font-weight: 600;
		color: white;
		margin: 0;
		line-height: 1.5;
	}

	/* Clip card */
	.clip-card {
		background: #000;
	}

	.clip-thumb {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.play-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.3);
	}

	.play-btn {
		width: 44px;
		height: 44px;
		background: rgba(255, 255, 255, 0.9);
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 16px;
		color: #0a0a0f;
		padding-left: 3px;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding: 40px;
		color: var(--color-text-secondary);
		font-size: 14px;
	}

	.empty-icon {
		font-size: 32px;
	}

	@media (max-width: 600px) {
		.moments-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
</style>
