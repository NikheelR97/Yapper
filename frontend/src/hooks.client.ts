import * as Sentry from "@sentry/sveltekit";

const dsn = import.meta.env.VITE_SENTRY_DSN as string | undefined;

const SENSITIVE_KEY_RE =
  /(authorization|cookie|set-cookie|csrf|token|password|password_hash|argon2|secret|email|message|description|body|plaintext|ciphertext|identity_dh_key|identity_sig_key|signed_prekey|one_time_prekey|private_key|jwt|key)/i;
const SENSITIVE_TEXT_RE =
  /(@.*\.)|bearer\s+|eyJ[\w-]*\.[\w-]*\.[\w-]*|\$argon2|refresh_token|csrf_token|set-cookie|authorization|password_hash|identity_dh_key|identity_sig_key|signed_prekey|one_time_prekey|private_key|jwt/i;

function scrubValue(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(scrubValue);
  }
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>).map(
      ([key, entryValue]) => [
        key,
        SENSITIVE_KEY_RE.test(key) ? "[redacted]" : scrubValue(entryValue),
      ],
    );
    return Object.fromEntries(entries);
  }
  if (typeof value === "string" && SENSITIVE_TEXT_RE.test(value)) {
    return "[redacted]";
  }
  return value;
}

export function sanitizeSentryEvent<T>(event: T): T {
  return scrubValue(event) as T;
}

export function init() {
  if (!dsn) {
    return;
  }

  Sentry.init({
    dsn,
    environment: import.meta.env.MODE,
    sendDefaultPii: false,
    // Capture 10% of traces in production to keep quota low
    tracesSampleRate: import.meta.env.DEV ? 1.0 : 0.1,
    // Disable replay capture until DOM masking and payload scrubbing are reviewed.
    replaysSessionSampleRate: 0,
    replaysOnErrorSampleRate: 0,
    beforeSend(event) {
      return sanitizeSentryEvent(event);
    },
  });
}

export const handleError = dsn ? Sentry.handleErrorWithSentry() : undefined;
