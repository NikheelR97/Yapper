<script lang="ts">
  export let apiUrl: string = '';

  let email = '';
  let status: 'idle' | 'loading' | 'success' | 'error' = 'idle';
  let errorMsg = '';
  let count = 0;

  const workerBase = apiUrl || 'https://yapperhq.com';

  // Fetch signup count on mount
  async function fetchCount() {
    try {
      const res = await fetch(`${workerBase}/api/wishlist/count`);
      if (res.ok) {
        const data = await res.json();
        count = data.count ?? 0;
      }
    } catch {
      // Non-critical — just don't show count
    }
  }

  fetchCount();

  async function handleSubmit() {
    if (!email || status === 'loading') return;
    status = 'loading';
    errorMsg = '';

    try {
      const res = await fetch(`${workerBase}/api/wishlist`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email }),
      });

      if (res.ok) {
        status = 'success';
        count += 1;
        email = '';
      } else {
        const data = await res.json().catch(() => ({}));
        errorMsg = data.error || 'Something went wrong. Try again.';
        status = 'error';
      }
    } catch {
      errorMsg = 'Network error. Please try again.';
      status = 'error';
    }
  }
</script>

<div class="wishlist">
  {#if status === 'success'}
    <div class="success" role="status">
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none" aria-hidden="true">
        <circle cx="10" cy="10" r="9" stroke="#4ade80" stroke-width="1.5"/>
        <path d="M6 10l3 3 5-5" stroke="#4ade80" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      <span>You're on the list! Check your inbox for a confirmation.</span>
    </div>
  {:else}
    <form on:submit|preventDefault={handleSubmit} class="form" novalidate>
      <div class="input-row">
        <input
          type="email"
          bind:value={email}
          placeholder="you@example.com"
          autocomplete="email"
          required
          disabled={status === 'loading'}
          aria-label="Email address"
        />
        <button type="submit" class="btn-primary" disabled={status === 'loading' || !email}>
          {#if status === 'loading'}
            <span class="spinner" aria-hidden="true"></span>
            Joining…
          {:else}
            Join the waitlist
          {/if}
        </button>
      </div>
      {#if status === 'error'}
        <p class="error" role="alert">{errorMsg}</p>
      {/if}
    </form>
  {/if}

  {#if count > 0}
    <p class="counter" aria-live="polite">
      <span class="counter-num">{count.toLocaleString()}</span> people already on the waitlist
    </p>
  {/if}
</div>

<style>
  .wishlist {
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
    width: 100%;
    max-width: 500px;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    width: 100%;
  }

  .input-row {
    display: flex;
    gap: 0.5rem;
    width: 100%;
  }

  @media (max-width: 520px) {
    .input-row { flex-direction: column; }
  }

  input[type="email"] {
    flex: 1;
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 9999px;
    color: #f9fafb;
    font-size: 0.9375rem;
    padding: 0.8125rem 1.25rem;
    outline: none;
    transition: border-color 150ms ease;
    min-width: 0;
  }

  input[type="email"]:focus {
    border-color: rgba(167,139,250,0.5);
  }

  input[type="email"]::placeholder { color: #6b7280; }
  input[type="email"]:disabled { opacity: 0.6; cursor: not-allowed; }

  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.8125rem 1.5rem;
    background: #7c3aed;
    color: #fff;
    border: none;
    border-radius: 9999px;
    font-size: 0.9375rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 150ms ease, box-shadow 150ms ease;
  }

  .btn-primary:hover:not(:disabled) {
    background: #5b21b6;
    box-shadow: 0 0 20px rgba(124,58,237,0.4);
  }

  .btn-primary:disabled { opacity: 0.6; cursor: not-allowed; }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .error {
    font-size: 0.8125rem;
    color: #fca5a5;
    padding: 0 0.5rem;
  }

  .success {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    color: #4ade80;
    font-size: 0.9375rem;
    font-weight: 500;
    padding: 0.875rem 1.25rem;
    background: rgba(74,222,128,0.08);
    border: 1px solid rgba(74,222,128,0.2);
    border-radius: 9999px;
  }

  .counter {
    font-size: 0.8125rem;
    color: #6b7280;
  }

  .counter-num {
    color: #a78bfa;
    font-weight: 700;
  }
</style>
