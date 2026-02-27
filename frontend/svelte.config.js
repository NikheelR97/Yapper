import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),

	kit: {
		// Static adapter — outputs to /build for Tauri, Capacitor, and Cloudflare Pages
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html',  // SPA fallback for client-side routing
			precompress: false,
			strict: false,
		}),

		alias: {
			$lib: './src/lib',
			$components: './src/lib/components',
			$stores: './src/lib/stores',
			$api: './src/lib/api',
			$signal: './src/lib/signal',
			$plugins: './src/lib/plugins',
		},
	},
};

export default config;
