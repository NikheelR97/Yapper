<script lang="ts">
	import { goto } from '$app/navigation';
	import { createChild, type ChildSettings } from '$stores/parental.js';
	import { toast } from '$stores/toast.js';
	import { authStore, setAuth } from '$stores/auth.js';
	import { get } from 'svelte/store';

	let step = 1;
	let totalSteps = 3;

	// Step 1
	let displayName = '';
	let username = '';
	let email = '';
	let password = '';
	let selectedVibe = 0;

	// Step 2
	let dobDay = '';
	let dobMonth = '';
	let dobYear = '';
	let showCoppaWarning = false;

	// Step 3 — Safety gates
	let approveFriendRequests = true;
	let approveServerJoins = true;
	let screenTimeLimitEnabled = false;
	let screenTimeLimitMinutes = 180; // 3h default
	let contentFilterEnabled = true;

	let submitting = false;

	const vibes = [
		{ emoji: '🎮', label: 'Gamer' },
		{ emoji: '🎨', label: 'Creator' },
		{ emoji: '🎵', label: 'Music' },
		{ emoji: '🏆', label: 'Sports' },
	];

	function checkCoppa() {
		const year = parseInt(dobYear);
		const month = parseInt(dobMonth);
		const day = parseInt(dobDay);
		if (!year || !month || !day) {
			showCoppaWarning = false;
			return;
		}
		const dob = new Date(year, month - 1, day);
		const age = Math.floor((Date.now() - dob.getTime()) / (365.25 * 24 * 3600000));
		showCoppaWarning = age < 13;
	}

	$: { dobYear; dobMonth; dobDay; checkCoppa(); }

	function nextStep() {
		if (step === 1) {
			if (!displayName.trim()) { toast.error('Please enter a display name.'); return; }
			if (!username.trim()) { toast.error('Please enter a username.'); return; }
			if (!/^[a-z0-9_]{3,32}$/.test(username.trim())) {
				toast.error('Username must be 3–32 characters: lowercase letters, numbers, underscores only.');
				return;
			}
			if (!email.trim() || !email.includes('@')) { toast.error('Please enter a valid email.'); return; }
			if (password.length < 8) { toast.error('Password must be at least 8 characters.'); return; }
		}
		if (step === 2 && (!dobDay || !dobMonth || !dobYear)) {
			toast.error('Please enter a valid date of birth.');
			return;
		}
		if (step === 2) {
			const d = new Date(parseInt(dobYear), parseInt(dobMonth) - 1, parseInt(dobDay));
			if (d.getDate() !== parseInt(dobDay)) {
				toast.error('Invalid date — please check day and month.');
				return;
			}
		}
		step = Math.min(step + 1, totalSteps);
	}

	function prevStep() {
		step = Math.max(step - 1, 1);
	}

	async function handleSubmit() {
		if (submitting) return;
		submitting = true;

		const dobStr = `${dobYear.padStart(4, '0')}-${dobMonth.padStart(2, '0')}-${dobDay.padStart(2, '0')}`;

		const settings: ChildSettings = {
			approveFriendRequests,
			approveServerJoins,
			screenTimeLimitMinutes: screenTimeLimitEnabled ? screenTimeLimitMinutes : null,
			contentFilterEnabled,
			bedtimeStart: null,
			bedtimeEnd: null,
		};

		try {
			await createChild({
				username: username.trim(),
				displayName: displayName.trim(),
				email: email.trim(),
				password,
				dateOfBirth: dobStr,
				settings,
			});
			// Update authStore so accountType reflects 'parent' immediately
			const s = get(authStore);
			if (s.user && s.user.accountType !== 'parent') {
				setAuth({ ...s.user, accountType: 'parent' }, s.accessToken!, s.csrfToken, s.device);
			}
			toast.success('Child account created!');
			await goto('/parent/dashboard');
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to create account');
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>Set Up Child Account — Yapper</title>
</svelte:head>

<div class="setup-page">
	<div class="setup-card">
		<!-- Progress pills -->
		<div class="progress-pills">
			{#each Array(totalSteps) as _, i}
				<div class="pill" class:active={i + 1 === step} class:done={i + 1 < step}></div>
			{/each}
		</div>

		<!-- Step 1: Profile -->
		{#if step === 1}
			<div class="step">
				<div class="step-icon">🛡</div>
				<h1 class="step-title">Secure Their Space</h1>
				<p class="step-desc">Set up your child's Yapper profile.</p>

				<div class="field">
					<label class="field-label" for="displayName">Display Name</label>
					<input
						id="displayName"
						type="text"
						class="field-input"
						placeholder="e.g. Alex"
						bind:value={displayName}
						maxlength={32}
					/>
				</div>

				<div class="field">
					<label class="field-label" for="username">Username</label>
					<input
						id="username"
						type="text"
						class="field-input"
						placeholder="e.g. alex_gamer (letters, numbers, _)"
						bind:value={username}
						maxlength={32}
					/>
				</div>

				<div class="field">
					<label class="field-label" for="email">Email Address</label>
					<input
						id="email"
						type="email"
						class="field-input"
						placeholder="e.g. alex@example.com"
						bind:value={email}
					/>
				</div>

				<div class="field">
					<label class="field-label" for="password">Password</label>
					<input
						id="password"
						type="password"
						class="field-input"
						placeholder="At least 8 characters"
						bind:value={password}
						minlength={8}
					/>
				</div>

				<fieldset class="field vibe-field">
					<legend class="field-label">Choose a Vibe</legend>
					<div class="vibe-grid">
						{#each vibes as vibe, i}
							<button
								type="button"
								class="vibe-card"
								class:selected={selectedVibe === i}
								aria-pressed={selectedVibe === i}
								on:click={() => (selectedVibe = i)}
							>
								<span class="vibe-emoji">{vibe.emoji}</span>
								<span class="vibe-label">{vibe.label}</span>
								{#if selectedVibe === i}
									<span class="vibe-check">✓</span>
								{/if}
							</button>
						{/each}
					</div>
				</fieldset>
			</div>

		<!-- Step 2: Date of birth -->
		{:else if step === 2}
			<div class="step">
				<div class="step-icon">🎂</div>
				<h1 class="step-title">Date of Birth</h1>
				<p class="step-desc">This helps us apply the right safety settings.</p>

				<div class="dob-row">
					<div class="field">
						<label class="field-label" for="dobDay">Day</label>
						<input
							id="dobDay"
							type="number"
							class="field-input"
							placeholder="DD"
							min="1"
							max="31"
							bind:value={dobDay}
						/>
					</div>
					<div class="field">
						<label class="field-label" for="dobMonth">Month</label>
						<input
							id="dobMonth"
							type="number"
							class="field-input"
							placeholder="MM"
							min="1"
							max="12"
							bind:value={dobMonth}
						/>
					</div>
					<div class="field">
						<label class="field-label" for="dobYear">Year</label>
						<input
							id="dobYear"
							type="number"
							class="field-input"
							placeholder="YYYY"
							min="2000"
							max={new Date().getFullYear()}
							bind:value={dobYear}
						/>
					</div>
				</div>

				{#if showCoppaWarning}
					<div class="coppa-banner">
						<span>⚠</span>
						<div>
							<strong>Under 13 — Extra Protection Active</strong>
							<p>All DMs and server joins require your approval before going through.</p>
						</div>
					</div>
				{/if}
			</div>

		<!-- Step 3: Safety gates -->
		{:else if step === 3}
			<div class="step step-safety">
				<div class="step-left">
					<div class="step-icon">🔒</div>
					<h1 class="step-title">Safety Gates</h1>
					<p class="step-desc">Choose what requires your approval.</p>

					<div class="toggle-list">
						<div class="toggle-row">
							<div class="toggle-info">
								<span class="toggle-label">Approve Friend Requests</span>
								<span class="toggle-desc">You review before {displayName || 'them'} accepts any friend request</span>
							</div>
							<label class="toggle-switch">
								<input type="checkbox" bind:checked={approveFriendRequests} />
								<span class="toggle-track"></span>
							</label>
						</div>

						<div class="toggle-row">
							<div class="toggle-info">
								<span class="toggle-label">Approve Server Joins</span>
								<span class="toggle-desc">You review all server join requests</span>
							</div>
							<label class="toggle-switch">
								<input type="checkbox" bind:checked={approveServerJoins} />
								<span class="toggle-track"></span>
							</label>
						</div>

						<div class="toggle-row">
							<div class="toggle-info">
								<span class="toggle-label">Screen Time Limit</span>
								<span class="toggle-desc">Set a daily Yapper usage cap</span>
							</div>
							<label class="toggle-switch">
								<input type="checkbox" bind:checked={screenTimeLimitEnabled} />
								<span class="toggle-track"></span>
							</label>
						</div>

						{#if screenTimeLimitEnabled}
							<div class="screen-time-slider">
								<label class="field-label" for="stSlider">
									Daily limit: {Math.floor(screenTimeLimitMinutes / 60)}h {screenTimeLimitMinutes % 60}m
								</label>
								<input
									id="stSlider"
									type="range"
									min="30"
									max="480"
									step="30"
									bind:value={screenTimeLimitMinutes}
									class="slider"
								/>
							</div>
						{/if}

						<div class="toggle-row">
							<div class="toggle-info">
								<span class="toggle-label">Content Filter</span>
								<span class="toggle-desc">Filter age-inappropriate content automatically</span>
							</div>
							<label class="toggle-switch">
								<input type="checkbox" bind:checked={contentFilterEnabled} />
								<span class="toggle-track"></span>
							</label>
						</div>
					</div>
				</div>

				<div class="step-right">
					<div class="tip-card">
						<div class="tip-title">💡 Quick Tip</div>
						<p class="tip-body">
							You can change these settings any time from the Parent Dashboard.
							E2EE protects message content — only metadata is visible to you.
						</p>
					</div>

					<div class="override-card">
						<div class="override-title">🛡 Parental Override</div>
						<p class="override-body">
							As a parent, you can always review, approve, or decline activity — even after the account is created.
						</p>
					</div>
				</div>
			</div>
		{/if}

		<!-- Navigation -->
		<div class="step-nav">
			{#if step > 1}
				<button class="btn-back" on:click={prevStep}>← Back</button>
			{:else}
				<div></div>
			{/if}

			{#if step < totalSteps}
				<button class="btn-next" on:click={nextStep}>Next →</button>
			{:else}
				<button class="btn-create" on:click={handleSubmit} disabled={submitting}>
					{submitting ? 'Creating…' : 'Create Yapper Account'}
				</button>
			{/if}
		</div>
	</div>
</div>

<style>
	.setup-page {
		min-height: 100vh;
		/* One Glow Rule: a single rationed violet glow over the theme canvas —
		   reads as a lit-room glow in dark and a soft brand tint in light. */
		background: radial-gradient(circle at 50% -10%, rgba(124, 58, 237, 0.18), transparent 55%),
			var(--color-bg-base);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
	}

	.setup-card {
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		box-shadow: 0 8px 40px rgba(0, 0, 0, 0.12);
		border-radius: 20px;
		padding: 40px;
		width: 100%;
		max-width: 680px;
	}

	.progress-pills {
		display: flex;
		gap: 8px;
		margin-bottom: 32px;
	}

	.pill {
		height: 4px;
		flex: 1;
		border-radius: 2px;
		background: var(--color-border);
		transition: background 300ms;
	}

	.pill.active {
		background: var(--color-brand);
	}

	.pill.done {
		background: var(--color-success);
	}

	.step {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.step-safety {
		display: grid;
		grid-template-columns: 1fr 240px;
		gap: 24px;
	}

	.step-left {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.step-right {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.step-icon {
		font-size: 40px;
	}

	.step-title {
		font-size: 26px;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0;
	}

	.step-desc {
		font-size: 14px;
		color: var(--color-text-secondary);
		margin: -12px 0 0;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.field-label {
		font-size: 12px;
		font-weight: 700;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.field-input {
		padding: 12px 16px;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		color: var(--color-text-primary);
		font-size: 15px;
		font-family: inherit;
		outline: none;
		transition: border-color 150ms;
	}

	.field-input:focus {
		border-color: var(--color-brand);
	}

	.vibe-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 10px;
	}

	.vibe-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 16px 8px;
		background: var(--color-bg-elevated);
		border: 2px solid transparent;
		border-radius: 12px;
		cursor: pointer;
		position: relative;
		transition: border-color 150ms, background 150ms;
	}

	.vibe-card.selected {
		border-color: var(--color-brand);
		background: rgba(124, 58, 237, 0.1);
	}

	.vibe-emoji {
		font-size: 28px;
	}

	.vibe-label {
		font-size: 12px;
		color: var(--color-text-secondary);
		font-weight: 500;
	}

	.vibe-check {
		position: absolute;
		top: 6px;
		right: 8px;
		font-size: 12px;
		color: var(--color-brand);
		font-weight: 700;
	}

	/* DOB */
	.dob-row {
		display: grid;
		grid-template-columns: 1fr 1fr 2fr;
		gap: 10px;
	}

	.coppa-banner {
		display: flex;
		gap: 12px;
		padding: 14px 16px;
		background: rgba(245, 158, 11, 0.08);
		border: 1px solid rgba(245, 158, 11, 0.3);
		border-radius: 10px;
		font-size: 13px;
		color: var(--color-warning-text);
	}

	.coppa-banner strong {
		display: block;
		margin-bottom: 4px;
	}

	.coppa-banner p {
		margin: 0;
		color: var(--color-text-secondary);
	}

	/* Toggles */
	.toggle-list {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 14px 16px;
		background: var(--color-bg-base);
		border-radius: 10px;
	}

	.toggle-info {
		display: flex;
		flex-direction: column;
		gap: 3px;
		flex: 1;
	}

	.toggle-label {
		font-size: 14px;
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.toggle-desc {
		font-size: 12px;
		color: var(--color-text-secondary);
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
		background: var(--color-text-muted);
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
		background: var(--color-success);
	}

	.toggle-switch input:checked + .toggle-track::after {
		transform: translateX(20px);
	}

	.screen-time-slider {
		padding: 0 16px 4px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.slider {
		width: 100%;
		accent-color: var(--color-brand);
		cursor: pointer;
	}

	/* Side cards */
	.tip-card,
	.override-card {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		padding: 16px;
	}

	.tip-title,
	.override-title {
		font-size: 13px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin-bottom: 8px;
	}

	.tip-body,
	.override-body {
		font-size: 12px;
		color: var(--color-text-secondary);
		margin: 0;
		line-height: 1.5;
	}

	/* Nav */
	.step-nav {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 32px;
	}

	.btn-back {
		background: none;
		border: none;
		color: var(--color-text-secondary);
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		padding: 0;
		transition: color 150ms;
	}

	.btn-back:hover {
		color: var(--color-text-primary);
	}

	.btn-next {
		padding: 12px 28px;
		background: rgba(124, 58, 237, 0.15);
		color: var(--color-brand-text);
		border: 1px solid rgba(124, 58, 237, 0.3);
		border-radius: 10px;
		font-size: 15px;
		font-weight: 700;
		cursor: pointer;
		transition: background 150ms;
	}

	.btn-next:hover {
		background: rgba(124, 58, 237, 0.3);
	}

	.btn-create {
		padding: 14px 32px;
		background: var(--color-brand);
		color: white;
		border: none;
		border-radius: 10px;
		font-size: 15px;
		font-weight: 700;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.btn-create:hover:not(:disabled) {
		opacity: 0.85;
	}

	.btn-create:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 600px) {
		.setup-card {
			padding: 24px;
		}

		.step-safety {
			grid-template-columns: 1fr;
		}

		.vibe-grid {
			grid-template-columns: repeat(2, 1fr);
		}

		.dob-row {
			grid-template-columns: 1fr 1fr 1fr;
		}
	}
</style>
