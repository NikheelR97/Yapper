/**
 * Clinical logging utility conforming to the Codex Security Report standard.
 *
 * Format: [HH:MM:SS.mmm] [CATEGORY] [ACTION] message
 *
 * Example:
 *   [14:02:45.012] [SECURITY] [VAULT_UNLOCK] Initiating Desktop Vault decryption seq.
 *   [14:02:45.410] [NETWORK] [WEBSOCKET] Terminating connection mid-flight to simulate MITM drop.
 *   [14:02:46.102] [VALIDATION] [UI] Fallback modal displayed within SLA limit (800ms). [PASS]
 */

export type LogCategory =
	| 'SECURITY'
	| 'ASSERTION'
	| 'NETWORK'
	| 'VALIDATION'
	| 'AUTH'
	| 'VAULT'
	| 'WEBSOCKET'
	| 'XSS';

export function log(category: LogCategory, action: string, message: string): void {
	const now = new Date();
	const pad = (n: number, len = 2) => String(n).padStart(len, '0');
	const ts = `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}.${pad(now.getMilliseconds(), 3)}`;
	console.log(`[${ts}] [${category}] [${action}] ${message}`);
}
