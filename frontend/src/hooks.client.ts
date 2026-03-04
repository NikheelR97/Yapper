import * as Sentry from '@sentry/sveltekit';

const dsn = import.meta.env.VITE_SENTRY_DSN as string | undefined;

if (dsn) {
	Sentry.init({
		dsn,
		environment: import.meta.env.MODE,
		// Capture 10% of traces in production to keep quota low
		tracesSampleRate: import.meta.env.DEV ? 1.0 : 0.1,
		// Only send replays on actual errors (0% ambient, 100% on error)
		replaysSessionSampleRate: 0,
		replaysOnErrorSampleRate: 1.0,
	});
}

export const handleError = dsn ? Sentry.handleErrorWithSentry() : undefined;
