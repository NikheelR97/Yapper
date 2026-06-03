<script lang="ts">
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';

	let tokenInput = '';
	let step: 'input' | 'success' = 'input';
	let yapperToken = '';
	let botName = '';
	let importing = false;

	async function importBot() {
		if (!tokenInput.trim()) {
			toast.error('Please enter your Discord bot token.');
			return;
		}
		importing = true;
		try {
			const res = await api.post<{ token: string; botName: string }>('/api/v2/bots/import-discord', {
				discordToken: tokenInput.trim(),
			});
			yapperToken = res.token;
			botName = res.botName;
			step = 'success';
			tokenInput = '';
		} catch (e: unknown) {
			toast.error(e instanceof Error ? e.message : 'Failed to import bot');
		} finally {
			importing = false;
		}
	}

	function copyToken() {
		navigator.clipboard.writeText(yapperToken).then(() => {
			toast.success('Token copied!');
		});
	}

	const migrationGuide = [
		{ from: "client.on('messageCreate')", to: "ws.on('message')" },
		{ from: "client.channels.send()", to: "POST /api/v2/channels/.." },
		{ from: "Discord.js Client", to: "yapper-bot-sdk (coming soon)" },
	];
</script>

<div class="dev-section">
	<h2 class="section-title">Yapper for Developers</h2>

	<div class="dev-card">
		<h3 class="card-title">🤖 Discord Bot Migration</h3>
		<p class="card-desc">Migrate your Discord bot to Yapper in minutes.</p>

		{#if step === 'input'}
			<div class="import-form">
				<div class="field">
					<label class="field-label" for="botToken">DISCORD BOT TOKEN</label>
					<input
						id="botToken"
						type="password"
						class="field-input"
						placeholder="Paste your bot token here…"
						bind:value={tokenInput}
					/>
				</div>

				<div class="warning-pill">
					⚠ This token is used once and never stored.
				</div>

				<button class="import-btn" on:click={importBot} disabled={importing}>
					{importing ? 'Importing…' : 'Import Bot →'}
				</button>
			</div>
		{:else}
			<div class="success-state">
				<div class="success-icon">✅</div>
				<h4 class="success-title">Bot Imported Successfully!</h4>
				<p class="bot-name">{botName} → Yapper Bot Account</p>

				<div class="token-section">
					<div class="token-label">YOUR YAPPER BOT TOKEN (shown once)</div>
					<div class="token-row">
						<code class="token-display">{yapperToken}</code>
						<button class="copy-btn" on:click={copyToken}>Copy</button>
					</div>
					<div class="warning-pill danger">
						⚠ Save this token now. It won't be shown again.
					</div>
				</div>

				<div class="migration-guide">
					<div class="guide-title">MIGRATION GUIDE</div>
					<table class="guide-table">
						<thead>
							<tr>
								<th>Discord</th>
								<th>→</th>
								<th>Yapper</th>
							</tr>
						</thead>
						<tbody>
							{#each migrationGuide as row}
								<tr>
									<td><code>{row.from}</code></td>
									<td class="arrow">→</td>
									<td><code>{row.to}</code></td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.dev-section {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.section-title {
		font-size: 20px;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0;
	}

	.dev-card {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 14px;
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.card-title {
		font-size: 16px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
	}

	.card-desc {
		font-size: 14px;
		color: var(--color-text-secondary);
		margin: 0;
	}

	.import-form {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.field-label {
		font-size: 11px;
		font-weight: 700;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.field-input {
		padding: 10px 14px;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		color: var(--color-text-primary);
		font-size: 14px;
		font-family: inherit;
		outline: none;
		transition: border-color 150ms;
	}

	.field-input:focus {
		border-color: var(--color-brand);
	}

	.warning-pill {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 6px 14px;
		background: rgba(245, 158, 11, 0.1);
		border: 1px solid rgba(245, 158, 11, 0.25);
		border-radius: 20px;
		font-size: 12px;
		color: var(--color-warning-text);
		align-self: flex-start;
	}

	.warning-pill.danger {
		background: rgba(239, 68, 68, 0.1);
		border-color: rgba(239, 68, 68, 0.25);
		color: var(--color-error-text);
	}

	.import-btn {
		padding: 12px 24px;
		background: #7c3aed;
		color: white;
		border: none;
		border-radius: 10px;
		font-size: 15px;
		font-weight: 700;
		cursor: pointer;
		align-self: flex-start;
		transition: opacity 150ms;
	}

	.import-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.import-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Success state */
	.success-state {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.success-icon {
		font-size: 40px;
	}

	.success-title {
		font-size: 18px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
	}

	.bot-name {
		font-size: 14px;
		color: var(--color-text-secondary);
		margin: 0;
	}

	.token-section {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.token-label {
		font-size: 11px;
		font-weight: 700;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.token-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 14px;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-left: 3px solid #f59e0b;
		border-radius: 8px;
	}

	.token-display {
		flex: 1;
		font-family: 'Courier New', monospace;
		font-size: 13px;
		color: var(--color-text-primary);
		word-break: break-all;
	}

	.copy-btn {
		padding: 6px 14px;
		background: rgba(124, 58, 237, 0.15);
		border: 1px solid rgba(124, 58, 237, 0.3);
		border-radius: 6px;
		color: var(--color-brand-text);
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		flex-shrink: 0;
		transition: background 150ms;
	}

	.copy-btn:hover {
		background: rgba(124, 58, 237, 0.3);
	}

	/* Migration guide */
	.migration-guide {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.guide-title {
		font-size: 11px;
		font-weight: 700;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.guide-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 13px;
		font-family: 'Courier New', monospace;
	}

	.guide-table th {
		text-align: left;
		padding: 6px 8px;
		color: var(--color-text-muted);
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.guide-table td {
		padding: 8px;
		color: var(--color-text-secondary);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.guide-table .arrow {
		color: var(--color-text-muted);
		text-align: center;
		width: 24px;
	}

	.guide-table td code {
		font-family: inherit;
	}
</style>
