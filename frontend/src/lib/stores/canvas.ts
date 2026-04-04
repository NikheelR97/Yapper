/**
 * Canvas store — Live Canvas state per server.
 *
 * Manages music (now playing + queue + skip votes), polls, clips (with
 * reactions + pinning), and live events (countdown). Hydrated via
 * GET /state endpoint; kept in sync via canvas_update WS events.
 */

import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';
import { onWsMessage } from '$stores/ws.js';
import { registerSessionResetter } from '$stores/auth.js';

// ─── Types ────────────────────────────────────────────────────────────────────

export interface MusicNowPlaying {
	track_id: string | null;
	artist: string;
	title: string;
	duration_secs: number | null;
	source_url: string | null;
	album_art_url: string | null;
	started_at: string;
	set_by: string;
}

export interface MusicQueueEntry {
	id: string;
	artist: string;
	title: string;
	duration_secs: number;
	source_url: string | null;
	album_art_url: string | null;
	added_by: string;
	position: number;
}

export interface MusicState {
	now_playing: MusicNowPlaying | null;
	queue: MusicQueueEntry[];
	skip_votes: number;
	skip_threshold_pct: number;
	online_members: number;
}

export interface PollOption {
	index: number;
	text: string;
}

export interface Poll {
	id: string;
	channel_id: string;
	poll_type: 'binary' | 'multiple_choice' | 'emoji_reaction';
	question: string;
	options: PollOption[];
	vote_counts: Record<string, number>;
	anonymous: boolean;
	status: 'active' | 'closed';
	ends_at: string | null;
	created_at: string;
	my_vote: number | null;
}

export interface ClipV2 {
	id: string;
	channel_id: string;
	sender_id: string;
	ciphertext: string | null;
	reactions: Record<string, number>;
	my_reactions: string[];
	pinned_at: string | null;
	created_at: string;
}

export interface CanvasEvent {
	id: string;
	title: string;
	description: string | null;
	event_at: string;
	created_by: string;
	created_at: string;
}

export interface CanvasState {
	music: MusicState;
	polls: Poll[];
	clips: ClipV2[];
	pinned_clips: ClipV2[];
	event: CanvasEvent | null;
	loading: boolean;
}

// Legacy types kept for backward compat with existing components during migration
export type { MusicNowPlaying as MusicStateLegacy };
export type Clip = ClipV2;

// ─── Per-server canvas stores ─────────────────────────────────────────────────

const canvasStores = new Map<string, ReturnType<typeof writable<CanvasState>>>();

const emptyState: CanvasState = {
	music: {
		now_playing: null,
		queue: [],
		skip_votes: 0,
		skip_threshold_pct: 50,
		online_members: 0,
	},
	polls: [],
	clips: [],
	pinned_clips: [],
	event: null,
	loading: false,
};

function resetCanvasState(): void {
	for (const store of canvasStores.values()) {
		store.set({ ...emptyState });
	}
	canvasStores.clear();
}

registerSessionResetter(resetCanvasState);

export function getCanvasStore(serverId: string) {
	if (!canvasStores.has(serverId)) {
		canvasStores.set(serverId, writable<CanvasState>({ ...emptyState }));
	}
	return canvasStores.get(serverId)!;
}

// ─── Hydration ────────────────────────────────────────────────────────────────

export async function loadCanvasState(
	serverId: string,
	channelId: string
): Promise<void> {
	const store = getCanvasStore(serverId);
	store.update((s) => ({ ...s, loading: true }));
	try {
		const res = await api.get<{
			music: MusicState;
			polls: Poll[];
			clips: ClipV2[];
			pinned_clips: ClipV2[];
			event: CanvasEvent | null;
		}>(`/api/v2/canvas/servers/${serverId}/state?channel_id=${channelId}`);
		store.set({
			music: res.music ?? emptyState.music,
			polls: res.polls ?? [],
			clips: res.clips ?? [],
			pinned_clips: res.pinned_clips ?? [],
			event: res.event ?? null,
			loading: false,
		});
	} catch {
		store.update((s) => ({ ...s, loading: false }));
	}
}

/** Legacy: load canvas state without channel_id (backward compat). */
export async function loadCanvas(serverId: string): Promise<void> {
	const store = getCanvasStore(serverId);
	store.update((s) => ({ ...s, loading: true }));
	try {
		const res = await api.get<{
			music: MusicNowPlaying | null;
			polls: Poll[];
			clips: ClipV2[];
		}>(`/api/v2/canvas/servers/${serverId}`);
		store.update((s) => ({
			...s,
			music: {
				...s.music,
				now_playing: res.music ?? null,
			},
			polls: res.polls ?? [],
			clips: res.clips ?? [],
			loading: false,
		}));
	} catch {
		store.update((s) => ({ ...s, loading: false }));
	}
}

export async function loadClips(serverId: string): Promise<void> {
	try {
		const res = await api.get<{ clips: ClipV2[] }>(
			`/api/v2/canvas/servers/${serverId}/clips`
		);
		getCanvasStore(serverId).update((s) => ({
			...s,
			clips: res.clips ?? [],
		}));
	} catch {
		// Non-critical — clips will load on next hydration
	}
}

// ─── Music API ────────────────────────────────────────────────────────────────

export interface MusicInput {
	artist: string;
	title: string;
	source_url?: string | null;
	album_art_url?: string | null;
	duration_secs?: number | null;
}

export async function setMusic(serverId: string, input: MusicInput): Promise<void> {
	await api.patch(`/api/v2/canvas/servers/${serverId}/music`, input);
}

export interface EnqueueTrackInput {
	artist: string;
	title: string;
	duration_secs: number;
	source_url?: string | null;
	album_art_url?: string | null;
}

export async function enqueueTrack(
	serverId: string,
	input: EnqueueTrackInput
): Promise<{ id: string; position: number }> {
	return api.post(`/api/v2/canvas/servers/${serverId}/music/queue`, input);
}

export async function removeFromQueue(
	serverId: string,
	trackId: string
): Promise<void> {
	await api.delete(`/api/v2/canvas/servers/${serverId}/music/queue/${trackId}`);
}

export async function reorderQueue(
	serverId: string,
	trackIds: string[]
): Promise<void> {
	await api.post(`/api/v2/canvas/servers/${serverId}/music/queue/reorder`, {
		track_ids: trackIds,
	});
}

export interface SkipVoteResult {
	skip_votes: number;
	online_members: number;
	threshold_pct: number;
	skipped: boolean;
	now_playing?: MusicNowPlaying | null;
}

export async function skipVote(serverId: string): Promise<SkipVoteResult> {
	return api.post(`/api/v2/canvas/servers/${serverId}/music/skip-vote`, {});
}

export async function advanceTrack(
	serverId: string,
	force = false
): Promise<{ now_playing: MusicNowPlaying | null; queue_remaining: number }> {
	return api.post(`/api/v2/canvas/servers/${serverId}/music/advance`, { force });
}

export interface HistoryTrack {
	id: string;
	artist: string;
	title: string;
	duration_secs: number;
	source_url: string | null;
	album_art_url: string | null;
	added_by: string;
	started_at: string | null;
	finished_at: string | null;
}

export async function getMusicHistory(
	serverId: string
): Promise<HistoryTrack[]> {
	const res = await api.get<{ tracks: HistoryTrack[] }>(
		`/api/v2/canvas/servers/${serverId}/music/history`
	);
	return res.tracks;
}

export async function updateMusicSettings(
	serverId: string,
	settings: { skip_threshold_pct?: number; queue_enabled?: boolean }
): Promise<void> {
	await api.patch(
		`/api/v2/canvas/servers/${serverId}/music/settings`,
		settings
	);
}

// ─── Poll API ─────────────────────────────────────────────────────────────────

export async function createPoll(
	channelId: string,
	question: string,
	options: string[],
	opts?: {
		poll_type?: string;
		ends_at?: string | null;
		anonymous?: boolean;
	}
): Promise<{ id: string }> {
	return api.post(`/api/v2/canvas/channels/${channelId}/polls`, {
		poll_type: opts?.poll_type ?? 'multiple_choice',
		question,
		options,
		ends_at: opts?.ends_at ?? null,
		anonymous: opts?.anonymous ?? false,
	});
}

export async function votePoll(
	serverId: string,
	pollId: string,
	optionIndex: number
): Promise<void> {
	const res = await api.post<{ vote_counts: Record<string, number> }>(
		`/api/v2/canvas/polls/${pollId}/vote`,
		{ option_index: optionIndex }
	);
	getCanvasStore(serverId).update((s) => ({
		...s,
		polls: s.polls.map((p) =>
			p.id === pollId
				? { ...p, vote_counts: res.vote_counts, my_vote: optionIndex }
				: p
		),
	}));
}

export async function closePoll(pollId: string): Promise<void> {
	await api.post(`/api/v2/canvas/polls/${pollId}/close`, {});
}

export interface PollResults {
	poll_id: string;
	poll_type: string;
	question: string;
	anonymous: boolean;
	status: string;
	total_votes: number;
	options: Array<{
		index: number;
		text: string;
		votes: number;
		voters: string[] | null;
	}>;
}

export async function getPollResults(pollId: string): Promise<PollResults> {
	return api.get(`/api/v2/canvas/polls/${pollId}/results`);
}

// ─── Clips API ────────────────────────────────────────────────────────────────

export async function addClipReaction(
	clipId: string,
	emoji: string
): Promise<void> {
	await api.put(`/api/v2/canvas/clips/${clipId}/reactions`, { emoji });
}

export async function removeClipReaction(
	clipId: string,
	emoji: string
): Promise<void> {
	await api.delete(
		`/api/v2/canvas/clips/${clipId}/reactions?emoji=${encodeURIComponent(emoji)}`
	);
}

export async function pinClip(
	serverId: string,
	clipId: string
): Promise<void> {
	await api.post(`/api/v2/canvas/servers/${serverId}/pinned-clips`, {
		clip_id: clipId,
	});
}

export async function unpinClip(
	serverId: string,
	clipId: string
): Promise<void> {
	await api.delete(
		`/api/v2/canvas/servers/${serverId}/pinned-clips/${clipId}`
	);
}

// ─── Events API ───────────────────────────────────────────────────────────────

export interface CreateEventInput {
	title: string;
	description?: string | null;
	event_at: string;
}

export async function createCanvasEvent(
	serverId: string,
	input: CreateEventInput
): Promise<CanvasEvent> {
	return api.post(`/api/v2/canvas/servers/${serverId}/events`, input);
}

export async function updateCanvasEvent(
	eventId: string,
	input: Partial<CreateEventInput>
): Promise<CanvasEvent> {
	return api.patch(`/api/v2/canvas/events/${eventId}`, input);
}

export async function deleteCanvasEvent(eventId: string): Promise<void> {
	await api.delete(`/api/v2/canvas/events/${eventId}`);
}

// ─── WS handler ───────────────────────────────────────────────────────────────

export function registerCanvasHandler(): () => void {
	return onWsMessage('canvas_update', (frame) => {
		const f = frame as { type: string; payload: Record<string, unknown> };
		const payload = f.payload;
		if (!payload?.type) return;

		const serverId = payload.server_id as string | undefined;

		// Look up only the targeted store instead of iterating all stores.
		// Falls back to forEach for events missing server_id (shouldn't happen
		// but keeps backward compatibility).
		const targetedStores: [string, ReturnType<typeof writable<CanvasState>>][] = [];
		if (serverId && canvasStores.has(serverId)) {
			targetedStores.push([serverId, canvasStores.get(serverId)!]);
		} else if (!serverId) {
			canvasStores.forEach((s, id) => targetedStores.push([id, s]));
		} else {
			return; // server_id present but no store subscribed — skip
		}

		for (const [, store] of targetedStores) {
			const state = get(store);

			switch (payload.type) {
				// ─── Music events ─────────────────────────────────────
				case 'music_track_started': {
					const np = payload.now_playing as MusicNowPlaying | undefined;
					store.update((s) => ({
						...s,
						music: {
							...s.music,
							now_playing: np ?? {
								track_id: (payload.track_id as string) ?? null,
								artist: payload.artist as string,
								title: payload.title as string,
								duration_secs: (payload.duration_secs as number) ?? null,
								source_url: (payload.source_url as string) ?? null,
								album_art_url: (payload.album_art_url as string) ?? null,
								started_at: (payload.started_at as string) ?? new Date().toISOString(),
								set_by: (payload.set_by as string) ?? '',
							},
							skip_votes: 0,
						},
					}));
					break;
				}

				case 'music_stopped': {
					store.update((s) => ({
						...s,
						music: { ...s.music, now_playing: null, skip_votes: 0 },
					}));
					break;
				}

				case 'music_queue_updated': {
					const queue = (payload.queue as MusicQueueEntry[]) ?? [];
					store.update((s) => ({
						...s,
						music: { ...s.music, queue },
					}));
					break;
				}

				case 'music_skip_vote_update': {
					store.update((s) => ({
						...s,
						music: {
							...s.music,
							skip_votes: (payload.skip_votes as number) ?? s.music.skip_votes,
							online_members:
								(payload.online_members as number) ?? s.music.online_members,
						},
					}));
					break;
				}

				case 'music_track_skipped': {
					const np = payload.now_playing as MusicNowPlaying | null | undefined;
					store.update((s) => ({
						...s,
						music: {
							...s.music,
							now_playing: np ?? null,
							skip_votes: 0,
						},
					}));
					break;
				}

				// Legacy compat
				case 'music_update': {
					store.update((s) => ({
						...s,
						music: {
							...s.music,
							now_playing: {
								track_id: null,
								artist: payload.artist as string,
								title: payload.title as string,
								duration_secs: null,
								source_url: (payload.source_url as string) ?? null,
								album_art_url: (payload.album_art_url as string) ?? null,
								started_at: new Date().toISOString(),
								set_by: (payload.set_by as string) ?? '',
							},
						},
					}));
					break;
				}

				// ─── Poll events ──────────────────────────────────────
				case 'poll_created': {
					const opts =
						(payload.options as Array<{ index: number; text: string }>) ?? [];
					const newPoll: Poll = {
						id: payload.poll_id as string,
						channel_id: payload.channel_id as string,
						poll_type:
							(payload.poll_type as Poll['poll_type']) ?? 'multiple_choice',
						question: payload.question as string,
						options: opts,
						vote_counts: {},
						anonymous: (payload.anonymous as boolean) ?? false,
						status: 'active',
						ends_at: (payload.ends_at as string) ?? null,
						created_at: new Date().toISOString(),
						my_vote: null,
					};
					if (!state.polls.find((p) => p.id === newPoll.id)) {
						store.update((s) => ({
							...s,
							polls: [newPoll, ...s.polls].slice(0, 5),
						}));
					}
					break;
				}

				case 'poll_vote': {
					const pollId = payload.poll_id as string;
					const vc = (payload.vote_counts as Record<string, number>) ?? {};
					store.update((s) => ({
						...s,
						polls: s.polls.map((p) =>
							p.id === pollId ? { ...p, vote_counts: vc } : p
						),
					}));
					break;
				}

				case 'poll_closed': {
					const pollId = payload.poll_id as string;
					const finalCounts =
						(payload.final_vote_counts as Record<string, number>) ?? {};
					store.update((s) => ({
						...s,
						polls: s.polls.map((p) =>
							p.id === pollId
								? {
										...p,
										status: 'closed' as const,
										vote_counts: finalCounts,
									}
								: p
						),
					}));
					break;
				}

				// ─── Clip events ──────────────────────────────────────
				case 'clip_reaction_added':
				case 'clip_reaction_removed': {
					const clipId = payload.clip_id as string;
					const counts =
						(payload.reaction_counts as Record<string, number>) ?? {};
					const updateClipReactions = (c: ClipV2) =>
						c.id === clipId ? { ...c, reactions: counts } : c;
					store.update((s) => ({
						...s,
						clips: s.clips.map(updateClipReactions),
						pinned_clips: s.pinned_clips.map(updateClipReactions),
					}));
					break;
				}

				case 'clip_pinned': {
					const clipId = payload.clip_id as string;
					store.update((s) => {
						const clip = s.clips.find((c) => c.id === clipId);
						if (!clip) return s;
						return {
							...s,
							clips: s.clips.filter((c) => c.id !== clipId),
							pinned_clips: [
								{ ...clip, pinned_at: new Date().toISOString() },
								...s.pinned_clips,
							].slice(0, 3),
						};
					});
					break;
				}

				case 'clip_unpinned': {
					const clipId = payload.clip_id as string;
					store.update((s) => {
						const clip = s.pinned_clips.find((c) => c.id === clipId);
						if (!clip) return s;
						return {
							...s,
							pinned_clips: s.pinned_clips.filter((c) => c.id !== clipId),
							clips: [{ ...clip, pinned_at: null }, ...s.clips],
						};
					});
					break;
				}

				// ─── Event events ─────────────────────────────────────
				case 'canvas_event_created':
				case 'canvas_event_updated': {
					const evt = payload.event as CanvasEvent | undefined;
					if (evt) {
						store.update((s) => ({ ...s, event: evt }));
					}
					break;
				}

				case 'canvas_event_deleted': {
					store.update((s) => ({ ...s, event: null }));
					break;
				}
			}
		}
	});
}
