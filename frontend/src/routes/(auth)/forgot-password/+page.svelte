<script lang="ts">
	import { api, ApiError } from '$api/client.js';

	let email = '';
	let error = '';
	let loading = false;
	let sent = false;

	async function handleSubmit() {
		error = '';
		loading = true;
		try {
			await api.post('/api/v1/auth/password-reset/request', { email });
			sent = true;
		} catch (e) {
			if (e instanceof ApiError) {
				error = e.message || 'Something went wrong. Please try again.';
			} else {
				error = 'Something went wrong. Please try again.';
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Forgot Password — Yapper</title>
</svelte:head>

<div class="auth-split">
	<div class="brand-panel" aria-hidden="true">
		<div class="sphere"></div>
		<p class="brand-tagline">We've Got You.</p>
	</div>

	<div class="form-panel">
		<div class="form-card">
			{#if sent}
				<div class="success-icon" aria-hidden="true">
					<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
						<polyline points="22 4 12 14.01 9 11.01" />
					</svg>
				</div>
				<h1 class="form-title">Check your inbox</h1>
				<p class="form-subtitle">
					If <strong>{email}</strong> is registered, we've sent a reset link. It expires in 1 hour.
				</p>
				<p class="form-subtitle">Don't see it? Check your spam folder.</p>
				<a href="/login" class="back-link">Back to sign in</a>
			{:else}
				<h1 class="form-title">Forgot Password?</h1>
				<p class="form-subtitle">Enter your email and we'll send a reset link.</p>

				{#if error}
					<div class="error-banner" role="alert">{error}</div>
				{/if}

				<form on:submit|preventDefault={handleSubmit} novalidate>
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

					<button type="submit" class="btn-primary" disabled={loading || !email}>
						{loading ? 'Sending…' : 'Send Reset Link'}
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

	.form-subtitle strong {
		color: var(--color-text-primary);
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
