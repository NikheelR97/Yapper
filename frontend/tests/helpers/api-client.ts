/**
 * E2EApiClient — consolidated API helper.
 *
 * Deduplicates the createSession / apiHeaders / createServerViaApi /
 * createInvite / joinByInvite helpers that were previously copy-pasted
 * across servers.spec.ts, social.spec.ts, and channel-e2ee.spec.ts.
 */

import { loginViaApi, type AuthData } from '../auth-helper.js';

export const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

export interface Session {
	accessToken: string;
	csrfToken: string;
	userId: string;
	username: string;
	displayName: string;
}

export class E2EApiClient {
	constructor(private readonly baseUrl: string = API_URL) {}

	// ─── Auth ─────────────────────────────────────────────────────────────────

	async login(email: string, password: string, installationId: string, label: string): Promise<Session> {
		const data: AuthData = await loginViaApi(email, password, { installationId, label });
		return {
			accessToken: data.accessToken,
			csrfToken: data.csrfToken,
			userId: String(data.user.id),
			username: String(data.user.username),
			displayName: String(data.user.displayName ?? data.user.display_name ?? data.user.username),
		};
	}

	// ─── Headers ──────────────────────────────────────────────────────────────

	authedHeaders(session: Pick<Session, 'accessToken' | 'csrfToken'>): Record<string, string> {
		return {
			'Content-Type': 'application/json',
			Authorization: `Bearer ${session.accessToken}`,
			Cookie: `csrf_token=${session.csrfToken}`,
			'X-CSRF-Token': session.csrfToken,
		};
	}

	// ─── Servers ──────────────────────────────────────────────────────────────

	async createServer(session: Session, name: string): Promise<{ serverId: string; channelId: string }> {
		const res = await fetch(`${this.baseUrl}/api/v1/servers`, {
			method: 'POST',
			headers: this.authedHeaders(session),
			body: JSON.stringify({ name }),
		});
		if (!res.ok) throw new Error(`createServer failed: ${res.status} ${await res.text()}`);
		const server = (await res.json()) as { id?: string };
		if (!server.id) throw new Error('createServer: missing server id');

		const chanRes = await fetch(`${this.baseUrl}/api/v1/servers/${server.id}/channels`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		if (!chanRes.ok) throw new Error(`listChannels failed: ${chanRes.status}`);
		const channels = (await chanRes.json()) as Array<{ id?: string }>;
		const channelId = channels[0]?.id;
		if (!channelId) throw new Error('createServer: no default channel');

		return { serverId: server.id, channelId };
	}

	async createInvite(session: Session, serverId: string): Promise<string> {
		const res = await fetch(`${this.baseUrl}/api/v1/servers/${serverId}/invite`, {
			method: 'POST',
			headers: this.authedHeaders(session),
			body: JSON.stringify({}),
		});
		if (!res.ok) throw new Error(`createInvite failed: ${res.status} ${await res.text()}`);
		const body = (await res.json()) as { code?: string; invite?: { code?: string } };
		const code = body.code ?? body.invite?.code;
		if (!code) throw new Error('createInvite: missing invite code');
		return code;
	}

	async joinByInvite(session: Session, code: string): Promise<void> {
		const res = await fetch(`${this.baseUrl}/api/v1/servers/join/${code}`, {
			method: 'POST',
			headers: this.authedHeaders(session),
			body: '{}',
		});
		if (!res.ok && res.status !== 409) throw new Error(`joinByInvite failed: ${res.status}`);
	}

	// ─── DMs ──────────────────────────────────────────────────────────────────

	async createDmConversation(session: Session, peerId: string): Promise<string> {
		const res = await fetch(`${this.baseUrl}/api/v1/conversations`, {
			method: 'POST',
			headers: this.authedHeaders(session),
			body: JSON.stringify({ peer_id: peerId }),
		});
		if (!res.ok) throw new Error(`createDmConversation failed: ${res.status}`);
		const body = (await res.json()) as { id?: string; conversation_id?: string };
		const id = body.id ?? body.conversation_id;
		if (!id) throw new Error('createDmConversation: missing id');
		return id;
	}

	// ─── Social ───────────────────────────────────────────────────────────────

	async followUser(session: Session, username: string): Promise<void> {
		const res = await fetch(`${this.baseUrl}/api/v1/users/by/${username}/follow`, {
			method: 'POST',
			headers: this.authedHeaders(session),
		});
		if (!res.ok && res.status !== 409) throw new Error(`follow failed: ${res.status}`);
	}

	async unfollowUser(session: Session, username: string): Promise<void> {
		const res = await fetch(`${this.baseUrl}/api/v1/users/by/${username}/follow`, {
			method: 'DELETE',
			headers: this.authedHeaders(session),
		});
		if (!res.ok && res.status !== 404) throw new Error(`unfollow failed: ${res.status}`);
	}

	async sendFriendRequest(session: Session, username: string): Promise<void> {
		const res = await fetch(`${this.baseUrl}/api/v1/users/by/${username}/friend-request`, {
			method: 'POST',
			headers: this.authedHeaders(session),
			body: '{}',
		});
		if (!res.ok && res.status !== 409) throw new Error(`sendFriendRequest failed: ${res.status}`);
	}

	async getProfile(session: Session, username: string): Promise<Record<string, unknown>> {
		const res = await fetch(`${this.baseUrl}/api/v1/users/by/${username}`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		if (!res.ok) throw new Error(`getProfile failed: ${res.status}`);
		return res.json() as Promise<Record<string, unknown>>;
	}

	// ─── Devices ──────────────────────────────────────────────────────────────

	async listDevices(session: Session): Promise<unknown[]> {
		const res = await fetch(`${this.baseUrl}/api/v2/devices`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		if (!res.ok) throw new Error(`listDevices failed: ${res.status}`);
		return res.json() as Promise<unknown[]>;
	}
}

/** Convenience singleton using the default API_URL. */
export const apiClient = new E2EApiClient();
