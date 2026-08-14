import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
	plugins: [sveltekit()],
	test: {
		include: ['src/**/*.{test,spec}.{js,ts}', 'functions/**/*.{test,spec}.{js,ts}'],
		globals: true,
		environment: 'jsdom',
		// The default `threads` (worker_threads) pool fails to initialize the test
		// context on this toolchain (all files error with "reading 'config'"). Forks
		// are marginally slower but reliable; the whole suite passes under them.
		pool: 'forks',
	},
});
