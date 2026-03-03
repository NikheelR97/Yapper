<script lang="ts">
	export let width: string = '100%';
	export let height: string = '16px';
	export let circle: boolean = false;
	export let lines: number = 1;
	export let gap: string = '8px';
</script>

{#if lines > 1}
	<div class="skeleton-lines" style="gap: {gap}">
		{#each Array(lines) as _, i}
			<div
				class="skeleton"
				class:circle
				style="
					width: {i === lines - 1 ? '60%' : width};
					height: {height};
					border-radius: {circle ? '50%' : '4px'};
				"
			></div>
		{/each}
	</div>
{:else}
	<div
		class="skeleton"
		class:circle
		style="
			width: {width};
			height: {height};
			border-radius: {circle ? '50%' : '4px'};
		"
	></div>
{/if}

<style>
	.skeleton-lines {
		display: flex;
		flex-direction: column;
	}

	.skeleton {
		background: linear-gradient(
			90deg,
			rgba(255, 255, 255, 0.04) 0%,
			rgba(255, 255, 255, 0.08) 50%,
			rgba(255, 255, 255, 0.04) 100%
		);
		background-size: 200% 100%;
		animation: shimmer 1.5s linear infinite;
	}

	.circle {
		flex-shrink: 0;
	}

	@keyframes shimmer {
		0% {
			background-position: -200% 0;
		}
		100% {
			background-position: 200% 0;
		}
	}
</style>
