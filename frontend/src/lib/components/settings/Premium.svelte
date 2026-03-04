<script lang="ts">
	import { onMount } from 'svelte';
	import { authStore, setPremiumStatus } from '$stores/auth.js';
	import { api, ApiError } from '$api/client.js';

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

	// Promo code activation
	let promoCode = '';
	let promoError = '';
	let promoSuccess = '';
	let promoLoading = false;
	let showPromoForm = false;

	// Cancel
	let cancelLoading = false;
	let cancelError = '';
	let showCancelConfirm = false;

	// Premium-since display
	let premiumSince: string | null = null;

	onMount(async () => {
		try {
			const status = await api.get<{ is_premium: boolean; premium_since: string | null }>(
				'/api/v1/premium'
			);
			premiumSince = status.premium_since;
			// Sync store in case auth token is stale
			if (status.is_premium !== isPremium) {
				setPremiumStatus(status.is_premium);
			}
		} catch {
			// Non-critical — falls back to auth store value
		}
	});

	async function handleActivate() {
		if (!promoCode.trim()) return;
		promoError = '';
		promoSuccess = '';
		promoLoading = true;
		try {
			await api.post('/api/v1/premium/activate', { promo_code: promoCode.trim() });
			setPremiumStatus(true);
			promoSuccess = 'GoPro activated! Welcome to the hype 🚀';
			promoCode = '';
			showPromoForm = false;
		} catch (e) {
			promoError = e instanceof ApiError ? e.message : 'Activation failed. Try again.';
		} finally {
			promoLoading = false;
		}
	}

	async function handleCancel() {
		cancelError = '';
		cancelLoading = true;
		try {
			await api.delete('/api/v1/premium');
			setPremiumStatus(false);
			premiumSince = null;
			showCancelConfirm = false;
		} catch (e) {
			cancelError = e instanceof ApiError ? e.message : 'Cancellation failed. Try again.';
		} finally {
			cancelLoading = false;
		}
	}

	function formatDate(iso: string | null): string {
		if (!iso) return '';
		return new Date(iso).toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'long',
			day: 'numeric',
		});
	}
</script>

<div class="premium-section">
	<h2 class="section-title">Yapper Premium</h2>

	{#if isPremium}
		<div class="current-plan">
			<div class="plan-badge pro">🚀 GoPro Active</div>
			<div>
				<p class="plan-desc">You have access to all premium features. Thank you for supporting Yapper!</p>
				{#if premiumSince}
					<p class="plan-since">GoPro since {formatDate(premiumSince)}</p>
				{/if}
			</div>
		</div>

		{#if !showCancelConfirm}
			<button class="cancel-link" on:click={() => (showCancelConfirm = true)}>
				Cancel GoPro subscription
			</button>
		{:else}
			<div class="cancel-confirm">
				<p>Are you sure? You'll lose access to all GoPro features immediately.</p>
				{#if cancelError}<p class="error-text">{cancelError}</p>{/if}
				<div class="confirm-actions">
					<button class="btn-danger" on:click={handleCancel} disabled={cancelLoading}>
						{cancelLoading ? 'Cancelling…' : 'Yes, cancel GoPro'}
					</button>
					<button class="btn-ghost" on:click={() => (showCancelConfirm = false)}>
						Keep GoPro
					</button>
				</div>
			</div>
		{/if}
	{:else}
		<div class="upgrade-hero">
			<div class="hero-icon">🚀</div>
			<h3 class="hero-title">Unlock the Full Experience</h3>
			<p class="hero-desc">Go Pro and get animated avatars, 100 custom emojis, HD clips, and more.</p>
			<a href="https://yapperhq.com/gopro" target="_blank" rel="noopener" class="upgrade-btn">
				Upgrade to GoPro →
			</a>
		</div>

		<!-- Promo code -->
		{#if promoSuccess}
			<div class="success-banner">{promoSuccess}</div>
		{:else}
			<div class="promo-section">
				{#if !showPromoForm}
					<button class="promo-toggle" on:click={() => (showPromoForm = true)}>
						Have a promo code?
					</button>
				{:else}
					<form class="promo-form" on:submit|preventDefault={handleActivate}>
						<input
							type="text"
							class="promo-input"
							bind:value={promoCode}
							placeholder="Enter promo code"
							disabled={promoLoading}
						/>
						<button type="submit" class="btn-promo" disabled={promoLoading || !promoCode.trim()}>
							{promoLoading ? 'Activating…' : 'Activate'}
						</button>
						<button
							type="button"
							class="btn-ghost"
							on:click={() => {
								showPromoForm = false;
								promoError = '';
							}}
						>
							Cancel
						</button>
					</form>
					{#if promoError}<p class="error-text">{promoError}</p>{/if}
				{/if}
			</div>
		{/if}
	{/if}

	<!-- Comparison table -->
	<div class="comparison-table">
		<div class="table-header">
			<div class="feature-col"></div>
			<div class="plan-col"><span class="plan-name free">Free</span></div>
			<div class="plan-col"><span class="plan-name pro">🚀 GoPro</span></div>
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

	.plan-since {
		font-size: 12px;
		color: #6b7280;
		margin: 4px 0 0;
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

	.hero-icon { font-size: 48px; }

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

	.upgrade-btn:hover { opacity: 0.85; }

	/* Promo code */
	.promo-section { display: flex; flex-direction: column; gap: 8px; }

	.promo-toggle {
		background: none;
		border: none;
		color: #a78bfa;
		font-size: 13px;
		cursor: pointer;
		padding: 0;
		text-decoration: underline;
		align-self: flex-start;
	}

	.promo-form {
		display: flex;
		gap: 8px;
		align-items: center;
		flex-wrap: wrap;
	}

	.promo-input {
		flex: 1;
		min-width: 160px;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 14px;
		padding: 8px 12px;
		outline: none;
	}

	.promo-input:focus { border-color: #7c3aed; }

	.btn-promo {
		padding: 8px 16px;
		background: linear-gradient(135deg, #7c3aed, #db2777);
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.btn-promo:disabled { opacity: 0.5; cursor: not-allowed; }

	/* Cancel */
	.cancel-link {
		background: none;
		border: none;
		color: #6b7280;
		font-size: 13px;
		cursor: pointer;
		padding: 0;
		text-decoration: underline;
		align-self: flex-start;
	}

	.cancel-link:hover { color: #ef4444; }

	.cancel-confirm {
		padding: 16px;
		background: rgba(239, 68, 68, 0.06);
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: 12px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.cancel-confirm p {
		font-size: 14px;
		color: #d1d5db;
		margin: 0;
	}

	.confirm-actions { display: flex; gap: 8px; }

	.btn-danger {
		padding: 8px 16px;
		background: #ef4444;
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.btn-danger:disabled { opacity: 0.5; cursor: not-allowed; }

	.btn-ghost {
		padding: 8px 16px;
		background: rgba(255, 255, 255, 0.06);
		color: #d1d5db;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		font-size: 14px;
		cursor: pointer;
		transition: background 150ms;
	}

	.btn-ghost:hover { background: rgba(255, 255, 255, 0.1); }

	/* Feedback */
	.success-banner {
		padding: 12px 16px;
		background: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.25);
		border-radius: 10px;
		font-size: 14px;
		color: #86efac;
	}

	.error-text {
		font-size: 13px;
		color: #fca5a5;
		margin: 0;
	}

	/* Comparison table */
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

	.table-row:last-child { border-bottom: none; }
	.table-row:hover { background: rgba(255, 255, 255, 0.02); }

	.feature-col { font-size: 14px; color: #d1d5db; }
	.plan-col { text-align: center; }

	.plan-name {
		font-size: 13px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.plan-name.free { color: #6b7280; }
	.plan-name.pro  { color: #a78bfa; }

	.x-mark     { color: #4b5563; font-size: 14px; }
	.check-mark { color: #22c55e; font-size: 16px; font-weight: 700; }

	.feature-val { font-size: 13px; color: #9ca3af; }
	.feature-val.pro-val { color: #a78bfa; font-weight: 600; }

	/* CTA footer */
	.cta-footer { display: flex; align-items: center; gap: 16px; }
	.cta-note { font-size: 13px; color: #6b7280; }
</style>
