# API Reference

Base URLs: `https://api.yapperhq.com` (production) · `https://staging-api.yapperhq.com` (staging) · `http://localhost:8080` (local)

The only documented HTTP API surface is `/api/v2/*`. Canonical device-aware auth lives under `/api/v2/auth/*`. OAuth browser redirects remain versionless under `/auth/oauth/*`.
All state-mutating requests require `X-CSRF-Token: <csrf_token>` (value from the `csrf_token` cookie).

---

## WebSocket

### `GET /ws`

Real-time bidirectional event stream. No query parameters — the access token is **never** placed in the URL (it would appear in proxy logs and browser history).

**Connection flow:**
1. Open `wss://api.yapperhq.com/ws` with no query parameters.
2. Immediately send the auth frame as the first message:
   ```json
   { "type": "auth", "token": "<access_token>" }
   ```
3. Server responds with `{ "type": "authenticated" }` on success, or closes the socket with code `4001` on failure.
4. Proactively re-authenticate before the access token expires: when the server sends `{ "type": "re_auth_required" }`, respond with `{ "type": "reauth", "token": "<new_access_token>" }` within 30 seconds.

**Limits:** max 5 concurrent connections per user; max frame size 64 KB; 5 messages/sec per user (burst 20).

---

## Health

### `GET /health`
No auth required.

```json
{ "status": "ok", "db": true }
```

---

## Auth

### `POST /api/v2/auth/register`
Device bootstrap registration. Returns access token, CSRF token, user, and device metadata.

### `POST /api/v2/auth/login`
Device bootstrap login. Returns access token, CSRF token, user, and device metadata.

### `POST /api/v2/auth/oauth/exchange`
Exchange an OAuth code for a device-aware session.

### `POST /api/v2/auth/attach-device`
Attach the current installation to an already signed-in account.

### `POST /api/v2/auth/refresh`
Refresh the current device-bound session.

### `DELETE /api/v2/auth/logout`
Invalidate the current refresh-token family.

### `GET /auth/oauth/discord` · `GET /auth/oauth/google` · `GET /auth/oauth/apple`
Redirect to the OAuth provider. No auth required.

### `GET /auth/oauth/discord/callback` · `GET /auth/oauth/google/callback` · `POST /auth/oauth/apple/callback`
OAuth callback handler. Sets tokens and redirects back into the app.

---

## Devices

### `GET /api/v2/devices`
List the authenticated user's devices.

### `POST /api/v2/devices/trust-requests`
Create a trust request for a pending device.

### `GET /api/v2/devices/sync-events`
Fetch queued sync events for the authenticated device.

### `POST /api/v2/devices/sync-events`
Create a device sync event.

### `POST /api/v2/devices/:id/approve`
Approve a pending device from a trusted device.

### `DELETE /api/v2/devices/:id`
Revoke a device.

---

## Keys

### `POST /api/v2/keys/identity`
Upload identity and signed prekey for the trusted device.

### `POST /api/v2/keys/signed-prekey`
Upload a signed prekey for the trusted device.

### `POST /api/v2/keys/one-time-prekeys`
Upload one-time prekeys for the trusted device.

### `GET /api/v2/keys/one-time-prekey-count`
Get the remaining one-time-prekey count for the trusted device.

### `GET /api/v2/keys/backup` · `PUT /api/v2/keys/backup`
Download or replace the encrypted key backup blob for the current user.

### `POST /api/v2/keys/backup/restore`
Restore a key backup into the current device vault.

### `GET /api/v2/keys/:user_id/bundles`
Fetch a user's device-aware key bundles.

---

## Conversations

### `POST /api/v2/conversations`
Create or get a trusted DM conversation.

### `GET /api/v2/conversations`
List device-aware DM conversations.

### `GET /api/v2/conversations/:id/messages`
Fetch encrypted DM history.

### `POST /api/v2/conversations/:id/messages`
Send a device-aware DM message.

---

## Users

### `GET /api/v2/users/me`
Own full profile.

```json
{
  "id": "uuid",
  "username": "alice",
  "display_name": "Alice",
  "avatar_url": null,
  "banner_url": null,
  "about_me": null,
  "account_type": "standard",
  "is_premium": false,
  "parental_controls_enabled": false,
  "created_at": "2026-..."
}
```

### `GET /api/v2/users/:id/presence`
```json
{ "online": true, "away": false, "last_seen_at": "2026-..." }
```

### `GET /api/v2/users/by/:username`
Public profile.

### `POST /api/v2/users/by/:username/follow`
Follow a user. Response: `204 No Content`

### `DELETE /api/v2/users/by/:username/follow`
Unfollow a user. Response: `204 No Content`

### `POST /api/v2/users/by/:username/friend-request`
If the target has parental controls, returns `202 { "status": "pending_parental_approval" }`.
Otherwise returns `201 { "status": "pending" }`.

### `GET /api/v2/users/me/feed`
Activity feed from followed users.

### `POST /api/v2/users/me/hype-moments`
Pin a message to the user's profile.

### `GET /api/v2/users/by/:username/hype-moments`
Fetch pinned profile moments for a user.

### `PATCH /api/v2/users/me`
Update profile fields.

### `POST /api/v2/users/me/avatar`
Upload a profile avatar.

### `POST /api/v2/users/me/banner`
Upload a profile banner.

### `PATCH /api/v2/users/me/username`
Change username with cooldown enforcement.

### `GET /api/v2/users/me/privacy`
Read privacy preferences.

### `PATCH /api/v2/users/me/privacy`
Update privacy preferences, including `show_last_seen`.

### `GET /api/v2/users/me/appearance`
Read appearance preferences.

### `PATCH /api/v2/users/me/appearance`
Update appearance preferences.

### `GET /api/v2/users/me/notifications`
Read notification preferences.

### `PATCH /api/v2/users/me/notifications`
Update notification preferences.

### `DELETE /api/v2/users/me/connections/:provider`
Unlink a connected account.

---

## Servers

### `GET /api/v2/servers`
List servers the authenticated user belongs to.

### `POST /api/v2/servers`
Create a server.

### `GET /api/v2/servers/:id`
Get server metadata.

### `PATCH /api/v2/servers/:id`
Update server metadata.

### `POST /api/v2/servers/:id/join`
Join a public server.

### `DELETE /api/v2/servers/:id/leave`
Leave a server.

### `POST /api/v2/servers/:id/invites`
Create an invite code.

### `POST /api/v2/servers/join/:code`
Join a server via invite code.

### `GET /api/v2/servers/:server_id/channels`
List channels for a server.

### `POST /api/v2/servers/:server_id/channels`
Create a channel in a server.

---

## Channels

### `GET /api/v2/channels/:id/messages`
Fetch encrypted channel history.

### `POST /api/v2/channels/:id/messages`
Send an encrypted channel message.

### `POST /api/v2/channels/:id/sender-key-distribution`
Store channel sender-key distribution payloads.

### `GET /api/v2/channels/:id/sender-key-distributions`
Fetch pending sender-key distribution payloads.

---

## Explore

### `GET /api/v2/explore/communities?tag=gaming&limit=20&offset=0`
Browse public communities.

### `GET /api/v2/explore/live-servers`
Browse recently active public servers.

### `GET /api/v2/explore/trending-tags`
Fetch the trending tag list.

### `GET /api/v2/explore/search?q=minecraft&limit=20`
Search servers and users.

### `GET /api/v2/explore/top-yappers`
Fetch the top creators by follower count.

### `GET /api/v2/search?q=...`
Unified discovery search endpoint.

---

## Canvas

### `GET /api/v2/canvas/:server_id`
Fetch the server canvas snapshot.

### `GET /api/v2/canvas/:server_id/clips`
Fetch canvas clips.

### `PATCH /api/v2/canvas/:server_id/music`
Update music state.

### `POST /api/v2/canvas/:server_id/polls`
Create a poll.

### `POST /api/v2/canvas/:server_id/polls/:poll_id/vote`
Vote in a poll.

### `GET /api/v2/canvas/:server_id/music/queue`
Fetch the music queue.

### `POST /api/v2/canvas/:server_id/music/queue`
Add a track to the music queue.

### `DELETE /api/v2/canvas/:server_id/music/queue/:track_id`
Remove a queued track.

### `POST /api/v2/canvas/:server_id/music/queue/reorder`
Reorder queued tracks.

### `POST /api/v2/canvas/:server_id/music/skip`
Skip the current track.

### `GET /api/v2/canvas/:server_id/music/history`
Fetch music history.

### `GET /api/v2/canvas/:server_id/music/settings`
Read music settings.

### `PATCH /api/v2/canvas/:server_id/music/settings`
Update music settings.

### `POST /api/v2/canvas/:server_id/polls/:id/close`
Close a poll.

### `GET /api/v2/canvas/:server_id/polls/:id/results`
Fetch poll results.

### `POST /api/v2/canvas/:server_id/clips/:id/reactions`
Add a clip reaction.

### `DELETE /api/v2/canvas/:server_id/clips/:id/reactions/:emoji`
Remove a clip reaction.

### `POST /api/v2/canvas/:server_id/clips/:id/pin`
Pin a clip.

### `DELETE /api/v2/canvas/:server_id/clips/:id/pin`
Unpin a clip.

### `POST /api/v2/canvas/:server_id/events`
Create a canvas event.

### `GET /api/v2/canvas/:server_id/events`
List canvas events.

### `PATCH /api/v2/canvas/:server_id/events/:id`
Update a canvas event.

### `DELETE /api/v2/canvas/:server_id/events/:id`
Delete a canvas event.

### `GET /api/v2/canvas/:server_id/state`
Read the hydrated canvas state.

---

## Parental Controls

### `POST /api/v2/parental/children`
Create a child account.

### `GET /api/v2/parental/children`
List managed children.

### `GET /api/v2/parental/children/:child_id/overview`
Read a child's metadata-only overview.

### `GET /api/v2/parental/children/:child_id/notifications`
Fetch pending parental notifications.

### `PATCH /api/v2/parental/friend-requests/:id/approve`
Approve a friend request.

### `PATCH /api/v2/parental/friend-requests/:id/decline`
Decline a friend request.

### `PATCH /api/v2/parental/server-joins/:id/approve`
Approve a server join request.

### `PATCH /api/v2/parental/server-joins/:id/decline`
Decline a server join request.

### `GET /api/v2/parental/children/:child_id/screentime?period=today|week|month`
Read screen-time summaries.

### `PATCH /api/v2/parental/children/:child_id/screentime`
Update screen-time settings.

---

## Media

### `POST /api/v2/media/upload-url`
Create a presigned upload URL.

### `GET /api/v2/media/:id/download-url`
Create a presigned download URL.

---

## Notifications

### `PUT /api/v2/notifications/push-token`
Register or update a push token for the current device.

### `DELETE /api/v2/notifications/push-token`
Remove the current device's push token.

---

## Bots

### `GET /api/v2/bots`
List bots.

### `POST /api/v2/bots/import-discord`
Import a Discord bot into Yapper.

### `DELETE /api/v2/bots/:id`
Delete a bot.

---

## Discord

### `GET /api/v2/discord/import-profile`
Start the Discord profile import flow.

### `GET /api/v2/discord/import-profile/callback`
Handle the Discord import callback.

---

## Premium

### `GET /api/v2/premium`
Read premium status.

### `POST /api/v2/premium/activate`
Activate premium.

### `DELETE /api/v2/premium`
Deactivate premium.

---

## Support

### `POST /api/v2/support/tickets`
Create a support ticket.

### `GET /api/v2/support/tickets`
List the authenticated user's support tickets.

### `POST /api/v2/support/webhooks/hubspot`
HubSpot status webhook.

---
