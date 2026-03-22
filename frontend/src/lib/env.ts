/** Centralised environment configuration. Import from here instead of
 *  repeating `import.meta.env.VITE_*` across modules. */

export const API_URL: string =
	import.meta.env.VITE_API_URL ?? 'http://localhost:8080';

export const WS_URL: string = API_URL.replace(/^http/, 'ws') + '/ws';

export const R2_PUBLIC_URL: string =
	import.meta.env.VITE_R2_PUBLIC_URL ?? '';
