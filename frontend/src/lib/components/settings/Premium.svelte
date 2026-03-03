<script lang="ts">
	import { authStore } from '$stores/auth.js';

	$: isPremium = $authStore.user?.isPremium ?? false;

	const features = [
		{ name: 'Animated Avatars', free: false, pro: true },
		{ name: 'Custom Emoji Slots', free: '50', pro: '100' },
		{ name: 'Custom Profile Theme', free: '5 colors', pro: 'Full hex picker' },
		{ name: 'HD Video Clips', free: '720p', pro: '1080p + 4K' },
		{ name: 'Yap Recording Length', free: '5 min', pro: '30 min' },
		{ name: 'Server Boosted Quality', free: false, pro: true },
		{ name: 'Priority Support', free: false, pro: true },
		{ name: 'GoPro Badge on Profile', free: false, pro: true },
	];
</script>

<div class="premium-section">
	<h2 class="section-title">Yapper Premium</h2>

	{#if isPremium}
		<div class="current-plan">
			<div class="plan-badge pro">🚀 GoPro Active</div>
			<p class="plan-desc">You have access to all premium features. Thank you for supporting Yapper!</p>
		</div>
	{:else}
		<div class="upgrade-hero">
			<div class="hero-icon">🚀</div>
			<h3 class="hero-title">Unlock the Full Experience</h3>
			<p class="hero-desc">Go Pro and get animated avatars, 100 custom emojis, HD clips, and more.</p>
			<a href="https://yapperhq.com/gopro" target="_blank" rel="noopener" class="upgrade-btn">
				Upgrade to GoPro →
			</a>
		</div>
	{/if}

	<!-- Comparison table -->
	<div class="comparison-table">
		<div class="table-header">
			<div class="feature-col"></div>
			<div class="plan-col">
				<span class="plan-name free">Free</span>
			</div>
			<div class="plan-col">
				<span class="plan-name pro">🚀 GoPro</span>
			</div>
		</div>

		{#each features as feature}
			<div class="table-row">
				<div class="feature-col">{feature.name}</div>
				<div class="plan-col">
					{#if feature.free === false}
						<span class="x-mark">✕</span>
					{:else if feature.free === true}
						<span class="check-mark">✓</span>
					{:else}
						<span class="feature-val">{feature.free}</span>
					{/if}
				</div>
				<div class="plan-col">
					{#if feature.pro === false}
						<span class="x-mark">✕</span>
					{:else if feature.pro === true}
						<span class="check-mark">✓</span>
					{:else}
						<span class="feature-val pro-val">{feature.pro}</span>
					{/if}
				</div>
			</div>
		{/each}
	</div>

	{#if !isPremium}
		<div class="cta-footer">
			<a href="https://yapperhq.com/gopro" target="_blank" rel="noopener" class="upgrade-btn">
				Upgrade to GoPro →
			</a>
			<span class="cta-note">Cancel anytime. Billed monthly.</span>
		</div>
	{/if}
</div>

<style>
	.premium-section {
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

	/* Current plan */
	.current-plan {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 20px;
		background: rgba(124, 58, 237, 0.08);
		border: 1px solid rgba(124, 58, 237, 0.25);
		border-radius: 14px;
	}

	.plan-badge {
		padding: 6px 14px;
		border-radius: 20px;
		font-size: 14px;
		font-weight: 700;
		flex-shrink: 0;
	}

	.plan-badge.pro {
		background: linear-gradient(135deg, #7c3aed, #db2777);
		color: white;
	}

	.plan-desc {
		font-size: 14px;
		color: #d1d5db;
		margin: 0;
	}

	/* Hero */
	.upgrade-hero {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		padding: 32px;
		background: linear-gradient(135deg, rgba(124, 58, 237, 0.1), rgba(219, 39, 119, 0.05));
		border: 1px solid rgba(124, 58, 237, 0.25);
		border-radius: 16px;
		gap: 12px;
	}

	.hero-icon {
		font-size: 48px;
	}

	.hero-title {
		font-size: 22px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
	}

	.hero-desc {
		font-size: 14px;
		color: #9ca3af;
		margin: 0;
		max-width: 380px;
	}

	.upgrade-btn {
		display: inline-block;
		padding: 12px 28px;
		background: linear-gradient(135deg, #7c3aed, #db2777);
		color: white;
		border-radius: 10px;
		font-size: 15px;
		font-weight: 700;
		text-decoration: none;
		transition: opacity 150ms;
		margin-top: 4px;
	}

	.upgrade-btn:hover {
		opacity: 0.85;
	}

	/* Table */
	.comparison-table {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 14px;
		overflow: hidden;
	}

	.table-header,
	.table-row {
		display: grid;
		grid-template-columns: 1fr 140px 140px;
		align-items: center;
	}

	.table-header {
		padding: 12px 20px;
		background: rgba(255, 255, 255, 0.03);
		border-bottom: 1px solid rgba(255, 255, 255, 0.07);
	}

	.table-row {
		padding: 14px 20px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		transition: background 100ms;
	}

	.table-row:last-child {
		border-bottom: none;
	}

	.table-row:hover {
		background: rgba(255, 255, 255, 0.02);
	}

	.feature-col {
		font-size: 14px;
		color: #d1d5db;
	}

	.plan-col {
		text-align: center;
	}

	.plan-name {
		font-size: 13px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.plan-name.free {
		color: #6b7280;
	}

	.plan-name.pro {
		color: #a78bfa;
	}

	.x-mark {
		color: #4b5563;
		font-size: 14px;
	}

	.check-mark {
		color: #22c55e;
		font-size: 16px;
		font-weight: 700;
	}

	.feature-val {
		font-size: 13px;
		color: #9ca3af;
	}

	.feature-val.pro-val {
		color: #a78bfa;
		font-weight: 600;
	}

	/* CTA footer */
	.cta-footer {
		display: flex;
		align-items: center;
		gap: 16px;
	}

	.cta-note {
		font-size: 13px;
		color: #6b7280;
	}
</style>
