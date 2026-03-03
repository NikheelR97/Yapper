<script lang="ts">
	import type { MusicState } from '$stores/canvas.js';

	export let music: MusicState;
</script>

<div class="music-widget">
	<div class="album-art-wrap">
		{#if music.album_art_url}
			<img class="album-art" src={music.album_art_url} alt="Album art" />
		{:else}
			<div class="album-art-placeholder" aria-hidden="true">
				<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
					<circle cx="12" cy="12" r="10"/>
					<circle cx="12" cy="12" r="3"/>
					<line x1="12" y1="2" x2="12" y2="9"/>
					<line x1="12" y1="15" x2="12" y2="22"/>
				</svg>
			</div>
		{/if}
		<!-- Vinyl spin animation overlay -->
		<div class="pulse-ring" aria-hidden="true"></div>
	</div>

	<div class="track-info">
		<p class="track-title" title={music.title}>{music.title}</p>
		<p class="track-artist" title={music.artist}>{music.artist}</p>
	</div>

	<div class="eq-bars" aria-label="Now playing" aria-hidden="true">
		<span></span>
		<span></span>
		<span></span>
		<span></span>
	</div>
</div>

<style>
	.music-widget {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		background: linear-gradient(135deg, rgba(124, 58, 237, 0.15), rgba(167, 139, 250, 0.05));
		border: 1px solid rgba(124, 58, 237, 0.25);
		border-radius: var(--radius-md);
	}

	.album-art-wrap {
		position: relative;
		flex-shrink: 0;
	}

	.album-art {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		object-fit: cover;
		animation: spin 8s linear infinite;
	}

	.album-art-placeholder {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: rgba(124, 58, 237, 0.2);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-brand-light);
		animation: spin 8s linear infinite;
	}

	.pulse-ring {
		position: absolute;
		inset: -4px;
		border-radius: 50%;
		border: 2px solid rgba(124, 58, 237, 0.4);
		animation: pulse-ring 2s ease-out infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	@keyframes pulse-ring {
		0%   { transform: scale(1); opacity: 0.6; }
		70%  { transform: scale(1.2); opacity: 0; }
		100% { transform: scale(1.2); opacity: 0; }
	}

	.track-info {
		flex: 1;
		min-width: 0;
	}

	.track-title {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.track-artist {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	/* Animated equalizer bars */
	.eq-bars {
		display: flex;
		align-items: flex-end;
		gap: 2px;
		height: 20px;
		flex-shrink: 0;
	}

	.eq-bars span {
		display: block;
		width: 3px;
		background: var(--color-brand-light);
		border-radius: 2px;
		animation: eq-bounce var(--dur, 0.8s) ease-in-out infinite alternate;
	}

	.eq-bars span:nth-child(1) { --dur: 0.7s; animation-delay: 0s; }
	.eq-bars span:nth-child(2) { --dur: 0.9s; animation-delay: 0.15s; }
	.eq-bars span:nth-child(3) { --dur: 0.6s; animation-delay: 0.3s; }
	.eq-bars span:nth-child(4) { --dur: 1.1s; animation-delay: 0.45s; }

	@keyframes eq-bounce {
		from { height: 4px; }
		to   { height: 18px; }
	}
</style>
