/**
 * Yapper Wishlist Worker
 *
 * Routes:
 *   POST /api/wishlist        -- register an email address
 *   GET  /api/wishlist/count  -- return current signup count (for the live counter)
 *
 * Bindings (set in wrangler.toml):
 *   DB  -- D1 database (yapper-wishlist)
 *   KV  -- KV namespace (yapper-counters)
 *
 * Secrets (set via `wrangler secret put`):
 *   RESEND_API_KEY
 *   FROM_EMAIL        e.g. "Yapper <noreply@yapperhq.com>"
 *   SITE_ORIGIN       e.g. "https://yapperhq.com"  (used for CORS)
 */

const CORS_HEADERS = (origin, allowedOrigin) => ({
  'Access-Control-Allow-Origin': origin === allowedOrigin ? origin : allowedOrigin,
  'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type',
  'Access-Control-Max-Age': '86400',
});

const KV_COUNTER_KEY = 'wishlist_count';

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const origin = request.headers.get('Origin') ?? '';
    const allowedOrigin = env.SITE_ORIGIN ?? '*';

    // Preflight
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        status: 204,
        headers: CORS_HEADERS(origin, allowedOrigin),
      });
    }

    let response;
    if (url.pathname === '/api/wishlist' && request.method === 'POST') {
      response = await handleSignup(request, env);
    } else if (url.pathname === '/api/wishlist/count' && request.method === 'GET') {
      response = await handleCount(env);
    } else {
      response = json({ error: 'Not found' }, 404);
    }

    // Attach CORS headers to every response
    const headers = new Headers(response.headers);
    for (const [k, v] of Object.entries(CORS_HEADERS(origin, allowedOrigin))) {
      headers.set(k, v);
    }
    return new Response(response.body, { status: response.status, headers });
  },
};

// ---------------------------------------------------------------------------
// POST /api/wishlist
// ---------------------------------------------------------------------------

async function handleSignup(request, env) {
  let body;
  try {
    body = await request.json();
  } catch {
    return json({ error: 'Invalid JSON' }, 400);
  }

  const email = (body?.email ?? '').trim().toLowerCase();
  if (!isValidEmail(email)) {
    return json({ error: 'Invalid email address' }, 400);
  }

  // Idempotent insert -- silently succeed if already registered
  try {
    const existing = await env.DB.prepare(
      'SELECT id FROM wishlist WHERE email = ?'
    ).bind(email).first();

    if (!existing) {
      await env.DB.prepare(
        'INSERT INTO wishlist (email) VALUES (?)'
      ).bind(email).run();

      // Increment KV counter
      const current = parseInt((await env.KV.get(KV_COUNTER_KEY)) ?? '0', 10);
      await env.KV.put(KV_COUNTER_KEY, String(current + 1));

      // Send confirmation email -- await so errors are logged, but don't fail signup
      try {
        await sendConfirmationEmail(email, env);
      } catch (err) {
        console.error('Email error:', err);
      }
    }
  } catch (err) {
    console.error('DB error:', err);
    return json({ error: 'Server error' }, 500);
  }

  return json({ ok: true }, 200);
}

// ---------------------------------------------------------------------------
// GET /api/wishlist/count
// ---------------------------------------------------------------------------

async function handleCount(env) {
  const count = parseInt((await env.KV.get(KV_COUNTER_KEY)) ?? '0', 10);
  return json({ count }, 200);
}

// ---------------------------------------------------------------------------
// Resend email
// ---------------------------------------------------------------------------

async function sendConfirmationEmail(email, env) {
  if (!env.RESEND_API_KEY) return;

  const from = env.FROM_EMAIL ?? 'Yapper <noreply@yapperhq.com>';

  const res = await fetch('https://api.resend.com/emails', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${env.RESEND_API_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      from,
      to: [email],
      subject: "You're on the Yapper waitlist",
      html: confirmationHtml(email),
      text: confirmationText(email),
    }),
  });

  if (!res.ok) {
    const body = await res.text();
    console.error('Resend API error:', res.status, body);
  }
}

function confirmationHtml(email) {
  return `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head>
<body style="margin:0;padding:0;background:#0f0f0f;font-family:'Segoe UI',system-ui,sans-serif;color:#e5e7eb;">
  <table width="100%" cellpadding="0" cellspacing="0" style="padding:48px 16px;">
    <tr><td align="center">
      <table width="100%" style="max-width:480px;background:#1a1a2e;border-radius:16px;overflow:hidden;">
        <tr><td style="padding:40px 40px 32px;text-align:center;">
          <div style="width:64px;height:64px;border-radius:50%;background:radial-gradient(circle at 35% 35%,#a78bfa,#7c3aed 40%,#3b0764 80%);margin:0 auto 24px;"></div>
          <h1 style="margin:0 0 8px;font-size:24px;font-weight:800;color:#f9fafb;">You're in.</h1>
          <p style="margin:0 0 24px;color:#9ca3af;line-height:1.6;">
            Thanks for joining the Yapper waitlist. We'll let you know the moment we launch.
          </p>
          <p style="margin:0;font-size:12px;color:#6b7280;">
            You signed up with <strong style="color:#a78bfa;">${email}</strong>
          </p>
        </td></tr>
      </table>
    </td></tr>
  </table>
</body>
</html>`;
}

function confirmationText(email) {
  return `You're on the Yapper waitlist!\n\nThanks for signing up. We'll reach out when we launch.\n\nYou signed up with: ${email}`;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function isValidEmail(email) {
  // RFC 5322 simplified -- good enough for a waitlist
  return /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(email) && email.length <= 254;
}
