<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';

	type TicketType = 'bug' | 'idea' | 'improvement';
	type Priority = 'low' | 'medium' | 'high' | 'urgent';

	interface Ticket {
		id: string;
		ticket_type: TicketType;
		subject: string;
		priority: Priority;
		status: string;
		hubspot_id: string | null;
		created_at: string;
	}

	let ticketType: TicketType = 'bug';
	let subject = '';
	let description = '';
	let priority: Priority = 'medium';
	let submitting = false;
	let tickets: Ticket[] = [];
	let loadingTickets = true;

	const typeOptions: { value: TicketType; label: string; desc: string; icon: string }[] = [
		{ value: 'bug', label: 'Bug Report', desc: 'Something is broken or not working as expected', icon: '🐛' },
		{ value: 'idea', label: 'Feature Idea', desc: 'A new feature you would like to see in Yapper', icon: '💡' },
		{ value: 'improvement', label: 'Improvement', desc: 'An existing feature that could work better', icon: '✨' },
	];

	const priorityOptions: { value: Priority; label: string; color: string }[] = [
		{ value: 'low',    label: 'Low',    color: '#22c55e' },
		{ value: 'medium', label: 'Medium', color: '#f59e0b' },
		{ value: 'high',   label: 'High',   color: '#f97316' },
		{ value: 'urgent', label: 'Urgent', color: '#ef4444' },
	];

	const statusLabel: Record<string, string> = {
		open:        'Open',
		in_progress: 'In Progress',
		resolved:    'Resolved',
		closed:      'Closed',
	};

	const typeLabel: Record<TicketType, string> = {
		bug:         'Bug',
		idea:        'Idea',
		improvement: 'Improvement',
	};

	$: subjectLen = subject.length;
	$: descLen = description.length;
	$: canSubmit = subject.trim().length > 0 && description.trim().length > 0 && !submitting;

	onMount(async () => {
		await loadTickets();
	});

	async function loadTickets() {
		loadingTickets = true;
		try {
			const res = await api.get<{ tickets: Ticket[] }>('/api/v1/support/tickets');
			tickets = res.tickets;
		} catch {
			tickets = [];
		} finally {
			loadingTickets = false;
		}
	}

	async function submit() {
		if (!canSubmit) return;
		submitting = true;
		try {
			await api.post('/api/v1/support/tickets', {
				ticket_type: ticketType,
				subject: subject.trim(),
				description: description.trim(),
				priority,
			});
			toast.success('Ticket submitted! Our team will review it shortly.');
			subject = '';
			description = '';
			priority = 'medium';
			ticketType = 'bug';
			await loadTickets();
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to submit ticket');
		} finally {
			submitting = false;
		}
	}
</script>

<div class="support-section">
	<h2 class="section-title">Support</h2>
	<p class="section-desc">
		Report a bug, suggest a feature, or let us know how we can improve Yapper.
		Our team reviews every submission.
	</p>

	<!-- Ticket type selector -->
	<div class="setting-block">
		<h3 class="block-title">What type of ticket is this?</h3>
		<div class="type-grid">
			{#each typeOptions as opt}
				<button
					class="type-card"
					class:selected={ticketType === opt.value}
					on:click={() => (ticketType = opt.value)}
					type="button"
				>
					<span class="type-icon">{opt.icon}</span>
					<span class="type-label">{opt.label}</span>
					<span class="type-desc">{opt.desc}</span>
				</button>
			{/each}
		</div>
	</div>

	<!-- Subject -->
	<div class="setting-block">
		<div class="field-header">
			<h3 class="block-title">Subject</h3>
			<span class="char-count" class:warn={subjectLen > 180}>{subjectLen}/200</span>
		</div>
		<input
			class="text-input"
			type="text"
			placeholder={ticketType === 'bug'
				? 'e.g. Messages not loading in server channels'
				: ticketType === 'idea'
				? 'e.g. Add reactions to Yap voice messages'
				: 'e.g. Make the explore page search faster'}
			bind:value={subject}
			maxlength="200"
		/>
	</div>

	<!-- Priority (bugs only) -->
	{#if ticketType === 'bug'}
		<div class="setting-block">
			<h3 class="block-title">Priority</h3>
			<p class="block-desc">How severely does this affect your use of Yapper?</p>
			<div class="priority-row">
				{#each priorityOptions as opt}
					<button
						class="priority-chip"
						class:selected={priority === opt.value}
						style="--chip-color: {opt.color}"
						on:click={() => (priority = opt.value)}
						type="button"
					>
						{opt.label}
					</button>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Description -->
	<div class="setting-block">
		<div class="field-header">
			<h3 class="block-title">
				{#if ticketType === 'bug'}Steps to reproduce / description{:else}Description{/if}
			</h3>
			<span class="char-count" class:warn={descLen > 1800}>{descLen}/2000</span>
		</div>
		<textarea
			class="text-area"
			placeholder={ticketType === 'bug'
				? '1. Go to a server channel\n2. Send a message\n3. Expected: message appears\n4. Actual: spinner hangs forever\n\nPlatform: Web / Desktop / Mobile\nVersion: ...'
				: ticketType === 'idea'
				? 'Describe the feature and why it would be useful…'
				: 'Describe what could be improved and how…'}
			bind:value={description}
			maxlength="2000"
			rows="7"
		></textarea>
	</div>

	<button class="submit-btn" on:click={submit} disabled={!canSubmit}>
		{#if submitting}
			Submitting…
		{:else}
			Submit Ticket
		{/if}
	</button>

	<!-- Past tickets -->
	<div class="history-section">
		<h3 class="history-title">Your Previous Tickets</h3>

		{#if loadingTickets}
			<div class="tickets-empty">Loading your tickets…</div>
		{:else if tickets.length === 0}
			<div class="tickets-empty">No tickets submitted yet.</div>
		{:else}
			<div class="ticket-list">
				{#each tickets as ticket}
					<div class="ticket-row">
						<div class="ticket-left">
							<span class="ticket-type-badge ticket-type-{ticket.ticket_type}">
								{typeLabel[ticket.ticket_type]}
							</span>
							<div class="ticket-info">
								<span class="ticket-subject">{ticket.subject}</span>
								<span class="ticket-meta">
									{new Date(ticket.created_at).toLocaleDateString(undefined, {
										year: 'numeric', month: 'short', day: 'numeric'
									})}
									{#if ticket.ticket_type === 'bug'}
										· {ticket.priority}
									{/if}
								</span>
							</div>
						</div>
						<span class="ticket-status ticket-status-{ticket.status}">
							{statusLabel[ticket.status] ?? ticket.status}
						</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.support-section {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.section-title {
		font-size: 20px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
	}

	.section-desc {
		font-size: 14px;
		color: #6b7280;
		margin: -16px 0 0;
		line-height: 1.6;
	}

	.setting-block {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 12px;
		padding: 18px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.block-title {
		font-size: 13px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.block-desc {
		font-size: 13px;
		color: #6b7280;
		margin: -8px 0 0;
	}

	.field-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.char-count {
		font-size: 12px;
		color: #6b7280;
	}

	.char-count.warn {
		color: #f59e0b;
	}

	/* Type selector */
	.type-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 10px;
	}

	.type-card {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 6px;
		padding: 14px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 10px;
		cursor: pointer;
		text-align: left;
		transition: background 100ms, border-color 100ms;
	}

	.type-card:hover {
		background: rgba(255, 255, 255, 0.06);
	}

	.type-card.selected {
		background: rgba(124, 58, 237, 0.12);
		border-color: rgba(124, 58, 237, 0.4);
	}

	.type-icon {
		font-size: 22px;
	}

	.type-label {
		font-size: 13px;
		font-weight: 700;
		color: #f9fafb;
	}

	.type-card.selected .type-label {
		color: #a78bfa;
	}

	.type-desc {
		font-size: 11px;
		color: #6b7280;
		line-height: 1.4;
	}

	/* Priority */
	.priority-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.priority-chip {
		padding: 6px 16px;
		border-radius: 20px;
		border: 1px solid rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.04);
		color: #9ca3af;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: background 100ms, border-color 100ms, color 100ms;
	}

	.priority-chip.selected {
		background: color-mix(in srgb, var(--chip-color) 15%, transparent);
		border-color: color-mix(in srgb, var(--chip-color) 50%, transparent);
		color: var(--chip-color);
	}

	/* Inputs */
	.text-input {
		width: 100%;
		padding: 10px 14px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		outline: none;
		box-sizing: border-box;
		transition: border-color 150ms;
	}

	.text-input:focus {
		border-color: #7c3aed;
	}

	.text-input::placeholder {
		color: #4b5563;
	}

	.text-area {
		width: 100%;
		padding: 10px 14px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		outline: none;
		resize: vertical;
		min-height: 140px;
		box-sizing: border-box;
		line-height: 1.6;
		transition: border-color 150ms;
	}

	.text-area:focus {
		border-color: #7c3aed;
	}

	.text-area::placeholder {
		color: #4b5563;
	}

	/* Submit */
	.submit-btn {
		padding: 12px 28px;
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

	.submit-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.submit-btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	/* Ticket history */
	.history-section {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding-top: 8px;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.history-title {
		font-size: 13px;
		font-weight: 700;
		color: #9ca3af;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.tickets-empty {
		font-size: 13px;
		color: #4b5563;
		padding: 12px 0;
	}

	.ticket-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.ticket-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 12px 14px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 10px;
	}

	.ticket-left {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	.ticket-type-badge {
		flex-shrink: 0;
		font-size: 10px;
		font-weight: 700;
		padding: 3px 8px;
		border-radius: 20px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.ticket-type-bug {
		background: rgba(239, 68, 68, 0.15);
		color: #fca5a5;
	}

	.ticket-type-idea {
		background: rgba(234, 179, 8, 0.15);
		color: #fde047;
	}

	.ticket-type-improvement {
		background: rgba(124, 58, 237, 0.15);
		color: #a78bfa;
	}

	.ticket-info {
		display: flex;
		flex-direction: column;
		gap: 3px;
		min-width: 0;
	}

	.ticket-subject {
		font-size: 13px;
		font-weight: 600;
		color: #e5e7eb;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.ticket-meta {
		font-size: 11px;
		color: #6b7280;
		text-transform: capitalize;
	}

	.ticket-status {
		flex-shrink: 0;
		font-size: 11px;
		font-weight: 700;
		padding: 4px 10px;
		border-radius: 20px;
	}

	.ticket-status-open {
		background: rgba(59, 130, 246, 0.12);
		color: #93c5fd;
	}

	.ticket-status-in_progress {
		background: rgba(234, 179, 8, 0.12);
		color: #fde047;
	}

	.ticket-status-resolved {
		background: rgba(34, 197, 94, 0.12);
		color: #86efac;
	}

	.ticket-status-closed {
		background: rgba(255, 255, 255, 0.06);
		color: #6b7280;
	}

	@media (max-width: 600px) {
		.type-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
