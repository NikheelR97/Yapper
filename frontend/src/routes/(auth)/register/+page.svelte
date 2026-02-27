<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, ApiError } from '$api/client.js';
	import { setAuth } from '$stores/auth.js';
	import type { User } from '$stores/auth.js';

	let username = '';
	let email = '';
	let password = '';
	let displayName = '';
	let error = '';
	let loading = false;

	// Password strength (0–4)
	$: strength = getStrength(password);
	$: strengthLabel = ['Very weak', 'Weak', 'Fair', 'Strong', 'Very strong'][strength];
	$: strengthColor = ['#ef4444', '#f97316', '#eab308', '#22c55e', '#16a34a'][strength];

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

	async function handleRegister() {
		error = '';
		loading = true;
		try {
			const res = await api.post<{ access_token: string; user: User }>(
				'/api/v1/auth/register',
				{ username, email, password, display_name: displayName || undefined }
			);
			setAuth(res.user, res.access_token);
			await goto('/onboarding');
		} catch (e) {
			if (e instanceof ApiError) {
				if (e.status === 409) {
					error = 'Username or email is already taken.';
				} else {
					error = e.message || 'Registration failed. Please try again.';
				}
			} else {
				error = 'Something went wrong. Please try again.';
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Join Yapper — Create Account</title>
</svelte:head>

<div class="auth-split">
	<div class="brand-panel" aria-hidden="true">
		<div class="sphere"></div>
		<p class="brand-tagline">Join the Hype.</p>
	</div>

	<div class="form-panel">
		<div class="form-card">
			<h1 class="form-title">Join the Hype</h1>
			<p class="form-subtitle">Create your Yapper account</p>

			{#if error}
				<div class="error-banner" role="alert">{error}</div>
			{/if}

			<form on:submit|preventDefault={handleRegister} novalidate>
				<div class="field">
					<label for="username">Username</label>
					<input
						id="username"
						type="text"
						bind:value={username}
						placeholder="your_handle"
						autocomplete="username"
						pattern="[a-zA-Z0-9_]+"
						minlength="2"
						maxlength="32"
						required
						disabled={loading}
					/>
					<span class="field-hint">Letters, numbers and underscores only</span>
				</div>

				<div class="field">
					<label for="display-name">Display Name <span class="optional">(optional)</span></label>
					<input
						id="display-name"
						type="text"
						bind:value={displayName}
						placeholder="Your Name"
						maxlength="64"
						disabled={loading}
					/>
				</div>

				<div class="field">
					<label for="email">Email</label>
					<input
						id="email"
						type="email"
						bind:value={email}
						placeholder="you@example.com"
						autocomplete="email"
						required
						disabled={loading}
					/>
				</div>

				<div class="field">
					<label for="password">Password</label>
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
								style="width: {(strength / 4) * 100}%; background: {strengthColor}"
							></div>
						</div>
						<span class="strength-label" style="color: {strengthColor}">{strengthLabel}</span>
					{/if}
				</div>

				<button
					type="submit"
					class="btn-primary"
					disabled={loading || !username || !email || !password || password.length < 8}
				>
					{loading ? 'Creating account…' : 'Create Account'}
				</button>
			</form>

			<div class="divider"><span>or sign up with</span></div>

			<div class="social-buttons">
				<a href="/api/v1/auth/oauth/discord" class="btn-social btn-discord">
					<svg width="18" height="14" viewBox="0 0 18 14" fill="currentColor" aria-hidden="true">
						<path d="M15.245 1.187A14.76 14.76 0 0 0 11.58 0c-.166.3-.36.703-.492 1.023a13.67 13.67 0 0 0-4.178 0C6.78.703 6.58.3 6.415 0A14.745 14.745 0 0 0 2.747 1.19C.395 4.787-.242 8.29.076 11.74c1.595 1.202 3.14 1.932 4.658 2.41a11.38 11.38 0 0 0 .978-1.625 9.633 9.633 0 0 1-1.54-.756c.13-.097.256-.198.378-.302 2.974 1.4 6.2 1.4 9.14 0 .123.104.25.205.377.302-.492.297-1.012.553-1.54.757.283.578.61 1.12.978 1.625 1.52-.478 3.067-1.208 4.663-2.41.382-4.047-.657-7.522-2.922-10.554ZM6.007 9.613c-.9 0-1.64-.844-1.64-1.877 0-1.034.722-1.88 1.64-1.88.916 0 1.652.846 1.638 1.88 0 1.033-.72 1.877-1.638 1.877Zm6.063 0c-.9 0-1.638-.844-1.638-1.877 0-1.034.72-1.88 1.638-1.88.92 0 1.65.846 1.64 1.88 0 1.033-.72 1.877-1.64 1.877Z"/>
					</svg>
					Discord
				</a>
				<a href="/api/v1/auth/oauth/google" class="btn-social btn-google">
					<svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
						<path fill="#4285F4" d="M17.64 9.2c0-.637-.057-1.251-.164-1.84H9v3.481h4.844a4.14 4.14 0 0 1-1.796 2.716v2.259h2.908c1.702-1.567 2.684-3.875 2.684-6.616Z"/>
						<path fill="#34A853" d="M9 18c2.43 0 4.467-.806 5.956-2.184l-2.908-2.259c-.806.54-1.837.86-3.048.86-2.344 0-4.328-1.584-5.036-3.711H.957v2.332A8.997 8.997 0 0 0 9 18Z"/>
						<path fill="#FBBC05" d="M3.964 10.706A5.41 5.41 0 0 1 3.682 9c0-.593.102-1.17.282-1.706V4.962H.957A8.996 8.996 0 0 0 0 9c0 1.452.348 2.827.957 4.038l3.007-2.332Z"/>
						<path fill="#EA4335" d="M9 3.58c1.321 0 2.508.454 3.44 1.345l2.582-2.58C13.463.891 11.426 0 9 0A8.997 8.997 0 0 0 .957 4.964L3.964 7.3C4.672 5.163 6.656 3.58 9 3.58Z"/>
					</svg>
					Google
				</a>
			</div>

			<p class="auth-switch">
				Already have an account? <a href="/login">Sign in</a>
			</p>

			<p class="legal-note">
				By creating an account you agree to our
				<a href="/privacy">Privacy Policy</a> and <a href="/terms">Terms of Service</a>.
			</p>
		</div>
	</div>
</div>

<style>
	/* Reuse auth-split layout from login — shared via global CSS in a real app */
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
		box-shadow: 0 0 80px rgba(124, 58, 237, 0.5);
		animation: pulse 3s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { box-shadow: 0 0 80px rgba(124, 58, 237, 0.5); }
		50% { box-shadow: 0 0 100px rgba(124, 58, 237, 0.7); }
	}

	.brand-tagline {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-text-primary);
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

	.optional {
		color: var(--color-text-muted);
		font-weight: 400;
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

	.field-hint {
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	.strength-bar {
		height: 3px;
		background: var(--color-border);
		border-radius: var(--radius-full);
		overflow: hidden;
	}

	.strength-fill {
		height: 100%;
		transition: width 0.3s ease, background 0.3s ease;
	}

	.strength-label {
		font-size: 0.75rem;
		font-weight: 500;
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
		margin-top: 0.5rem;
	}

	.btn-primary:hover:not(:disabled) { background: var(--color-brand-dark); }
	.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

	.divider {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin: 1.5rem 0;
		color: var(--color-text-muted);
		font-size: 0.8125rem;
	}

	.divider::before,
	.divider::after {
		content: '';
		flex: 1;
		height: 1px;
		background: var(--color-border);
	}

	.social-buttons {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.btn-social {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.625rem;
		padding: 0.625rem;
		border-radius: var(--radius-md);
		font-weight: 500;
		font-size: 0.9375rem;
		text-decoration: none;
		transition: opacity var(--transition-fast);
	}

	.btn-social:hover { opacity: 0.85; text-decoration: none; }
	.btn-discord { background: #5865F2; color: white; }
	.btn-google { background: var(--color-bg-surface); color: var(--color-text-primary); border: 1px solid var(--color-border); }

	.auth-switch {
		text-align: center;
		color: var(--color-text-muted);
		font-size: 0.875rem;
		margin-top: 1.5rem;
	}

	.auth-switch a { color: var(--color-brand-light); font-weight: 500; }

	.legal-note {
		text-align: center;
		color: var(--color-text-muted);
		font-size: 0.75rem;
		margin-top: 1rem;
		line-height: 1.6;
	}
</style>
