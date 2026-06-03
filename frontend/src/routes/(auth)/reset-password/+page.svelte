<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { get } from 'svelte/store';
	import { api, ApiError } from '$api/client.js';

	let password = '';
	let confirmPassword = '';
	let error = '';
	let loading = false;
	let done = false;

	const token = get(page).url.searchParams.get('token') ?? '';

	$: strength = getStrength(password);
	$: strengthLabel = ['Very weak', 'Weak', 'Fair', 'Strong', 'Very strong'][strength];
	$: strengthColor = ['#ef4444', '#f97316', '#eab308', '#22c55e', '#16a34a'][strength];
	$: mismatch = confirmPassword.length > 0 && password !== confirmPassword;

	function getStrength(pw: string): number {
		if (!pw) return 0;
		let score = 0;
		if (pw.length >= 8) score++;
		if (pw.length >= 12) score++;
		if (/[A-Z]/.test(pw)) score++;
		if (/[0-9]/.test(pw)) score++;
		if (/[^A-Za-z0-9]/.test(pw)) score++;
		return Math.min(4, score);
	}

	async function handleSubmit() {
		if (password !== confirmPassword) {
			error = 'Passwords do not match.';
			return;
		}

		error = '';
		loading = true;
		try {
			await api.post('/api/v2/auth/password-reset/confirm', {
				token,
				new_password: password,
			});
			done = true;
		} catch (e) {
			if (e instanceof ApiError) {
				error = e.message || 'Reset failed. The link may have expired.';
			} else {
				error = 'Something went wrong. Please try again.';
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Reset Password — Yapper</title>
</svelte:head>

<div class="auth-split">
	<div class="brand-panel" aria-hidden="true">
		<div class="sphere"></div>
		<p class="brand-tagline">Fresh Start.</p>
	</div>

	<div class="form-panel">
		<div class="form-card">
			{#if !token}
				<h1 class="form-title">Invalid Link</h1>
				<p class="form-subtitle">This reset link is missing a token. Please request a new one.</p>
				<a href="/forgot-password" class="back-link">Request new link</a>

			{:else if done}
				<div class="success-icon" aria-hidden="true">
					<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
						<polyline points="22 4 12 14.01 9 11.01" />
					</svg>
				</div>
				<h1 class="form-title">Password Updated</h1>
				<p class="form-subtitle">Your password has been reset. You can now sign in.</p>
				<button class="btn-primary" on:click={() => goto('/login')}>Sign In</button>

			{:else}
				<h1 class="form-title">Reset Password</h1>
				<p class="form-subtitle">Choose a new password for your account.</p>

				{#if error}
					<div class="error-banner" role="alert">{error}</div>
				{/if}

				<form on:submit|preventDefault={handleSubmit} novalidate>
					<div class="field">
						<label for="password">New Password</label>
						<input
							id="password"
							type="password"
							bind:value={password}
							placeholder="At least 8 characters"
							autocomplete="new-password"
							minlength="8"
							required
							disabled={loading}
						/>
						{#if password}
							<div class="strength-bar">
								<div
									class="strength-fill"
									style="transform: scaleX({strength / 4}); background: {strengthColor}"
								></div>
							</div>
							<span class="strength-label" style="color: {strengthColor}">{strengthLabel}</span>
						{/if}
					</div>

					<div class="field">
						<label for="confirm-password">Confirm Password</label>
						<input
							id="confirm-password"
							type="password"
							bind:value={confirmPassword}
							placeholder="Re-enter your password"
							autocomplete="new-password"
							required
							disabled={loading}
						/>
						{#if mismatch}
							<span class="field-error">Passwords do not match</span>
						{/if}
					</div>

					<button
						type="submit"
						class="btn-primary"
						disabled={loading || !password || password.length < 8 || mismatch || !confirmPassword}
					>
						{loading ? 'Updating…' : 'Update Password'}
					</button>
				</form>

				<a href="/login" class="back-link">Back to sign in</a>
			{/if}
		</div>
	</div>
</div>

<style>
	.auth-split {
		display: flex;
		min-height: 100vh;
		min-height: 100dvh;
	}

	.brand-panel {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 2rem;
		background: linear-gradient(135deg, #1a0533 0%, #0f0f0f 100%);
		padding: 2rem;

		@media (max-width: 768px) {
			display: none;
		}
	}

	.sphere {
		width: 200px;
		height: 200px;
		border-radius: 50%;
		background: radial-gradient(circle at 35% 35%, #a78bfa, #7c3aed 40%, #3b0764 80%);
		box-shadow: 0 0 80px rgba(124, 58, 237, 0.5), 0 0 160px rgba(124, 58, 237, 0.2);
		animation: pulse 3s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { box-shadow: 0 0 80px rgba(124, 58, 237, 0.5), 0 0 160px rgba(124, 58, 237, 0.2); }
		50% { box-shadow: 0 0 100px rgba(124, 58, 237, 0.7), 0 0 200px rgba(124, 58, 237, 0.3); }
	}

	.brand-tagline {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-text-primary);
		text-align: center;
	}

	.form-panel {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		background: var(--color-bg-base);
	}

	.form-card {
		width: 100%;
		max-width: 400px;
	}

	.form-title {
		font-size: 2rem;
		font-weight: 800;
		color: var(--color-text-primary);
		margin-bottom: 0.25rem;
	}

	.form-subtitle {
		color: var(--color-text-secondary);
		margin-bottom: 1.5rem;
		line-height: 1.5;
	}

	.success-icon {
		margin-bottom: 1rem;
	}

	.error-banner {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-md);
		margin-bottom: 1rem;
		font-size: 0.875rem;
	}

	.field {
		margin-bottom: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	label {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--color-text-secondary);
	}

	input {
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text-primary);
		font-size: 1rem;
		padding: 0.625rem 0.875rem;
		transition: border-color var(--transition-fast);
		width: 100%;
	}

	input:focus {
		border-color: var(--color-brand);
		outline: none;
	}

	input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.strength-bar {
		height: 3px;
		background: var(--color-border);
		border-radius: var(--radius-full);
		overflow: hidden;
	}

	.strength-fill {
		width: 100%;
		height: 100%;
		transform-origin: left;
		/* Animate transform, not width, to keep the strength meter off the layout path. */
		transition: transform 0.3s ease, background 0.3s ease;
	}

	.strength-label {
		font-size: 0.75rem;
		font-weight: 500;
	}

	.field-error {
		font-size: 0.75rem;
		color: #fca5a5;
	}

	.btn-primary {
		width: 100%;
		background: var(--color-brand);
		color: white;
		border: none;
		border-radius: var(--radius-md);
		font-size: 1rem;
		font-weight: 600;
		padding: 0.75rem;
		cursor: pointer;
		transition: background var(--transition-fast), opacity var(--transition-fast);
	}

	.btn-primary:hover:not(:disabled) {
		background: var(--color-brand-dark);
	}

	.btn-primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.back-link {
		display: block;
		text-align: center;
		color: var(--color-text-muted);
		font-size: 0.875rem;
		margin-top: 1.5rem;
		text-decoration: none;
	}

	.back-link:hover {
		color: var(--color-brand-light);
	}
</style>
