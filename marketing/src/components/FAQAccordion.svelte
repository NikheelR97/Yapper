<script lang="ts">
  const items = [
    {
      q: 'Is Yapper really end-to-end encrypted?',
      a: 'Yes. Every direct message and group channel uses the Signal Protocol — the same cryptography used by Signal, WhatsApp, and Google Messages. Messages are encrypted on your device before they leave it. Not even Yapper employees can read your chats.',
    },
    {
      q: 'How is Yapper different from Discord?',
      a: 'Discord is not end-to-end encrypted — their team can read your messages and comply with data requests. Yapper encrypts everything by default. We also ship parental controls that respect E2EE — parents manage who their child connects with, not what they say.',
    },
    {
      q: 'When will Yapper launch?',
      a: "We're in active development following a 6-month sprint plan. Sign up for the waitlist and you'll be among the first to know — and the first to get access.",
    },
    {
      q: 'Is it free?',
      a: 'The core Yapper experience — DMs, servers, voice Yaps, video Clips, live canvas — is free forever. A Premium tier ($4.99/month) adds larger storage, unlimited servers, HD video, and custom badges. We will never paywall core features.',
    },
    {
      q: 'How do parental controls work with E2EE?',
      a: "E2EE is inviolable — parents can never read message content and that's by design. What parents control is the social graph: friend requests to children require parent approval before any encrypted session is established. Server joins work the same way. Screen time limits use iOS DeviceActivity and Android UsageStatsManager APIs at the OS level.",
    },
    {
      q: 'Will it be available on iOS and Android at launch?',
      a: 'Yes. Web, iOS, Android, macOS, and Windows are all planned for launch.',
    },
  ];

  let openIndex: number | null = null;

  function toggle(i: number) {
    openIndex = openIndex === i ? null : i;
  }
</script>

<section class="faq-section" id="faq">
  <div class="container">
    <div class="section-header">
      <div class="badge">FAQ</div>
      <h2 class="section-title">Questions, <span class="gradient-text">answered.</span></h2>
    </div>

    <div class="accordion" role="list">
      {#each items as item, i}
        <div class="item" role="listitem">
          <button
            class="question"
            class:open={openIndex === i}
            on:click={() => toggle(i)}
            aria-expanded={openIndex === i}
            aria-controls={`faq-answer-${i}`}
            id={`faq-q-${i}`}
          >
            <span>{item.q}</span>
            <span class="chevron" aria-hidden="true">
              <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
                <path d="M4.5 6.75L9 11.25L13.5 6.75" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </span>
          </button>
          {#if openIndex === i}
            <div
              class="answer"
              id={`faq-answer-${i}`}
              role="region"
              aria-labelledby={`faq-q-${i}`}
            >
              <p>{item.a}</p>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
</section>

<style>
  .faq-section {
    padding: var(--section-gap) 0;
  }

  .section-header {
    text-align: center;
    margin-bottom: 3rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .section-title {
    font-size: clamp(2rem, 4vw, 3rem);
    font-weight: 900;
    letter-spacing: -0.02em;
  }

  .accordion {
    max-width: 720px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .item {
    background: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: border-color 150ms ease;
  }

  .item:has(.open) {
    border-color: rgba(167,139,250,0.25);
  }

  .question {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 1.125rem 1.5rem;
    background: none;
    border: none;
    color: var(--color-text-primary);
    font-size: 0.9375rem;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
    transition: color 150ms ease;
  }

  .question:hover { color: var(--color-brand-light); }
  .question.open  { color: var(--color-brand-light); }

  .chevron {
    flex-shrink: 0;
    color: var(--color-text-muted);
    transition: transform 250ms ease;
  }

  .question.open .chevron { transform: rotate(180deg); }

  .answer {
    padding: 0 1.5rem 1.25rem;
    animation: slide-in 200ms ease;
  }

  .answer p {
    font-size: 0.9375rem;
    color: var(--color-text-secondary);
    line-height: 1.7;
  }

  @keyframes slide-in {
    from { opacity: 0; transform: translateY(-6px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  /* Make section-title and badge work without global CSS in svelte */
  :global(.section-title .gradient-text) {
    background: linear-gradient(135deg, #c4b5fd 0%, #7c3aed 50%, #a78bfa 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
</style>
