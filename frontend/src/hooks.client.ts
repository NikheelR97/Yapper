import * as Sentry from "@sentry/sveltekit";

const dsn = import.meta.env.VITE_SENTRY_DSN as string | undefined;

const SENSITIVE_KEY_RE =
  /(authorization|cookie|set-cookie|token|password|secret|email|message|description|body)/i;

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
