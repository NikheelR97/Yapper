import { sveltekit } from '@sveltejs/kit/vite';
import { VitePWA } from 'vite-plugin-pwa';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [
		sveltekit(),
		VitePWA({
			registerType: 'autoUpdate',
			includeAssets: ['favicon.svg', 'robots.txt'],
			manifest: {
				name: 'Yapper',
				short_name: 'Yapper',
				description: 'End-to-end encrypted messaging with voice Yaps, video Clips, and live canvases',
				theme_color: '#7C3AED',
				background_color: '#0F0F0F',
				display: 'standalone',
				icons: [
					{ src: '/icons/icon-192.png', sizes: '192x192', type: 'image/png' },
					{ src: '/icons/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any maskable' },
				],
			},
		}),
	],

	// Tauri expects a fixed port during dev
	server: {
		port: 5173,
		strictPort: true,
	},

	// Allow libsignal WASM
	optimizeDeps: {
		exclude: ['@signalapp/libsignal-client'],
	},

	build: {
		// Tauri uses a static webview - no SSR
		target: 'esnext',
	},

	test: {
		environment: 'jsdom',
	},
});
