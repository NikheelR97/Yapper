/**
 * Canvas store — Live Canvas state per server (music, polls, clips).
 *
 * API surface:
 *   loadCanvas(serverId)       — fetch music state + active polls
 *   loadClips(serverId)        — fetch recent clip metadata
 *   setMusic(serverId, input)  — PATCH music state
 *   createPoll(channelId, …)   — POST new poll
 *   votePoll(pollId, optionIndex) — POST vote
 *   registerCanvasHandler()    — wire WS canvas_update events
 */

import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';
import { onWsMessage } from '$stores/ws.js';

// ─── Types ────────────────────────────────────────────────────────────────────

export interface MusicState {
	artist: string;
	title: string;
	source_url: string | null;
	album_art_url: string | null;
	set_by: string | null;
	updated_at: string | null;
}

export interface PollOption {
	index: number;
	text: string;
}

export interface Poll {
	id: string;
	channel_id: string;
	question: string;
	options: PollOption[];
	vote_counts: Record<string, number>;
	ends_at: string | null;
	created_at: string;
}

export interface Clip {
	id: string;
	channel_id: string;
	sender_id: string;
	ciphertext: string | null;
	created_at: string;
}

export interface CanvasState {
	music: MusicState | null;
	polls: Poll[];
	clips: Clip[];
	loadingCanvas: boolean;
	loadingClips: boolean;
}

// ─── Per-server canvas stores ─────────────────────────────────────────────────

const canvasStores = new Map<string, ReturnType<typeof writable<CanvasState>>>();

export function getCanvasStore(serverId: string) {
	if (!canvasStores.has(serverId)) {
		canvasStores.set(
			serverId,
			writable<CanvasState>({
				music: null,
				polls: [],
				clips: [],
				loadingCanvas: false,
				loadingClips: false,
			})
		);
	}
	return canvasStores.get(serverId)!;
}

// ─── API calls ────────────────────────────────────────────────────────────────

export async function loadCanvas(serverId: string): Promise<void> {
	const store = getCanvasStore(serverId);
	store.update((s) => ({ ...s, loadingCanvas: true }));
	try {
		const res = await api.get<{ music: MusicState | null; polls: Poll[] }>(
			`/api/v1/canvas/servers/${serverId}`
		);
		store.update((s) => ({
			...s,
			music: res.music ?? null,
			polls: res.polls ?? [],
			loadingCanvas: false,
		}));
	} catch (e) {
		store.update((s) => ({ ...s, loadingCanvas: false }));
		throw e;
	}
}

export async function loadClips(serverId: string): Promise<void> {
	const store = getCanvasStore(serverId);
	store.update((s) => ({ ...s, loadingClips: true }));
	try {
		const res = await api.get<{ clips: Clip[] }>(`/api/v1/canvas/servers/${serverId}/clips`);
		store.update((s) => ({
			...s,
			clips: res.clips ?? [],
			loadingClips: false,
		}));
	} catch (e) {
		store.update((s) => ({ ...s, loadingClips: false }));
		throw e;
	}
}

export interface MusicInput {
	artist: string;
	title: string;
	source_url?: string | null;
	album_art_url?: string | null;
}

export async function setMusic(serverId: string, input: MusicInput): Promise<void> {
	await api.patch(`/api/v1/canvas/servers/${serverId}/music`, input);
	// Optimistic update
	getCanvasStore(serverId).update((s) => ({
		...s,
		music: {
			artist: input.artist,
			title: input.title,
			source_url: input.source_url ?? null,
			album_art_url: input.album_art_url ?? null,
			set_by: null,
			updated_at: new Date().toISOString(),
		},
	}));
}

export async function createPoll(
	channelId: string,
	question: string,
	options: string[],
	ends_at?: string | null
): Promise<{ id: string }> {
	return api.post<{ id: string }>(`/api/v1/canvas/channels/${channelId}/polls`, {
		question,
		options,
		ends_at: ends_at ?? null,
	});
}

export async function votePoll(
	serverId: string,
	pollId: string,
	optionIndex: number
): Promise<void> {
	const res = await api.post<{ vote_counts: Record<string, number> }>(
		`/api/v1/canvas/polls/${pollId}/vote`,
		{ option_index: optionIndex }
	);
	// Update local vote counts
	getCanvasStore(serverId).update((s) => ({
		...s,
		polls: s.polls.map((p) =>
			p.id === pollId ? { ...p, vote_counts: res.vote_counts } : p
		),
	}));
}

// ─── WS handler ───────────────────────────────────────────────────────────────

export function registerCanvasHandler(): () => void {
	return onWsMessage('canvas_update', (frame) => {
		const f = frame as { type: string; payload: Record<string, unknown> };
		const payload = f.payload;
		if (!payload?.type) return;

		// Find which server store to update by matching server_id or poll_id
		canvasStores.forEach((store) => {
			const state = get(store);

			switch (payload.type) {
				case 'music_update': {
					store.update((s) => ({
						...s,
						music: {
							artist: payload.artist as string,
							title: payload.title as string,
							source_url: (payload.source_url as string) ?? null,
							album_art_url: (payload.album_art_url as string) ?? null,
							set_by: (payload.set_by as string) ?? null,
							updated_at: new Date().toISOString(),
						},
					}));
					break;
				}

				case 'poll_created': {
					const opts = (payload.options as Array<{ index: number; text: string }>) ?? [];
					const newPoll: Poll = {
						id: payload.poll_id as string,
						channel_id: payload.channel_id as string,
						question: payload.question as string,
						options: opts,
						vote_counts: {},
						ends_at: (payload.ends_at as string) ?? null,
						created_at: new Date().toISOString(),
					};
					// Avoid duplicate if already in store
					if (!state.polls.find((p) => p.id === newPoll.id)) {
						store.update((s) => ({ ...s, polls: [newPoll, ...s.polls] }));
					}
					break;
				}

				case 'poll_vote': {
					const pollId = payload.poll_id as string;
					const voteCounts = (payload.vote_counts as Record<string, number>) ?? {};
					store.update((s) => ({
						...s,
						polls: s.polls.map((p) =>
							p.id === pollId ? { ...p, vote_counts: voteCounts } : p
						),
					}));
					break;
				}
			}
		});
	});
}
