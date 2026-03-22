# API Reference

Base URL: `https://api.yapperhq.com` (production) · `http://localhost:8080` (local)

All `/api/v1/*` endpoints require `Authorization: Bearer <access_token>` unless noted.
All state-mutating requests require `X-CSRF-Token: <csrf_token>` (value from the `csrf_token` cookie).

---

## Health

### `GET /health`
No auth required.
```json
{ "status": "ok", "db": true }
```

---

## Auth — `/auth/*`

### `POST /auth/register`
```json
// Request
{ "username": "alice", "display_name": "Alice", "email": "alice@example.com", "password": "..." }

// Response 201
{ "user_id": "uuid", "message": "Verification email sent" }
```

### `POST /auth/login`
```json
// Request
{ "email": "alice@example.com", "password": "..." }

// Response 200 — sets HttpOnly refresh_token cookie
{ "access_token": "jwt", "expires_in": 900, "user": { "id": "...", "username": "alice", ... } }
```

### `POST /auth/refresh`
No body — reads `refresh_token` cookie.
```json
// Response 200
{ "access_token": "jwt", "expires_in": 900 }
```

### `POST /auth/logout`
Invalidates the refresh token.

### `GET /auth/verify-email?token=...`
Verifies the email address from the link sent to the user.

### `POST /auth/forgot-password`
```json
{ "email": "alice@example.com" }
```

### `POST /auth/reset-password`
```json
{ "token": "...", "new_password": "..." }
```

### `GET /auth/oauth/discord` · `GET /auth/oauth/google`
Redirects to OAuth provider. No auth required.

### `GET /auth/oauth/discord/callback` · `GET /auth/oauth/google/callback`
OAuth redirect handler — sets tokens and redirects to app.

---

## Users — `/api/v1/users/*`

### `GET /api/v1/users/me`
Own full profile.
```json
{
  "id": "uuid", "username": "alice", "display_name": "Alice",
  "avatar_url": null, "banner_url": null, "about_me": null,
  "account_type": "standard", "is_premium": false,
  "parental_controls_enabled": false, "created_at": "2026-..."
}
```

### `GET /api/v1/users/:id/presence`
```json
{ "online": true, "away": false, "last_seen_at": "2026-..." }
```

### `GET /api/v1/users/by/:username`
Public profile.
```json
{
  "id": "uuid", "username": "bob", "follower_count": 42,
  "following_count": 10, "is_following": false,
  "mutual_followers": [{ "id": "...", "username": "...", "avatar_url": null }],
  "top_communities": [{ "id": "...", "name": "...", "slug": "...", "member_count": 100 }]
}
```

### `POST /api/v1/users/by/:username/follow`
Follow a user. Response: `204 No Content`

### `DELETE /api/v1/users/by/:username/follow`
Unfollow. Response: `204 No Content`

### `POST /api/v1/users/by/:username/friend-request`
If target has parental controls → `202 { "status": "pending_parental_approval" }`
Otherwise → `201 { "status": "pending" }`

### `GET /api/v1/users/me/feed`
Activity feed from followed users (hype moments).
```json
{ "items": [{ "id": "...", "type": "yap", "pinned_at": "...", "author": { ... } }] }
```

### `POST /api/v1/users/me/hype-moments`
```json
// Request
{ "message_id": "uuid", "type": "yap" }   // type: yap | clip | text
// Response 201
{ "id": "uuid" }
```

### `GET /api/v1/users/by/:username/hype-moments`
```json
{ "moments": [{ "id": "...", "message_id": "...", "type": "yap", "pinned_at": "..." }] }
```

---

## Servers — `/api/v1/servers/*`

### `GET /api/v1/servers`
List servers the authenticated user is a member of.

### `POST /api/v1/servers`
```json
// Request
{ "name": "My Server", "description": "...", "is_public": true }
// Response 201 — ServerResp
```

### `GET /api/v1/servers/:id`
```json
{
  "id": "uuid", "name": "...", "slug": "...", "owner_id": "...",
  "icon_url": null, "description": null, "is_public": true,
  "member_count": 42, "role": "member"
}
```

### `PATCH /api/v1/servers/:id`
Admin only.
```json
{ "name": "New Name", "description": "...", "is_public": false, "icon_url": "..." }
```

### `POST /api/v1/servers/:id/join`
Join a public server. Returns `{ "status": "joined" }` or `{ "status": "pending_approval" }` for child accounts.

### `DELETE /api/v1/servers/:id/leave`
Leave a server. Owners must transfer ownership first.

### `POST /api/v1/servers/:id/invites`
```json
// Request
{ "max_uses": 10, "expires_in_hours": 24 }
// Response 201
{ "code": "abc123x", "server_id": "...", "max_uses": 10, "expires_at": "..." }
```

### `POST /api/v1/servers/join/:code`
Join via invite link. Returns `{ "status": "joined" }` or `{ "status": "pending_approval" }`.

---

## Channels — `/api/v1/channels/*`

### `GET /api/v1/channels/:server_id`
List channels for a server (must be a member).

### `POST /api/v1/channels/:server_id`
Admin only.
```json
{ "name": "announcements", "type": "text" }
```

### `GET /api/v1/channels/:id/messages`
Message history (last 50, encrypted ciphertext).
```json
{ "messages": [{ "id": "...", "ciphertext": "base64...", "sender_id": "...", "created_at": "..." }] }
```

### `POST /api/v1/channels/:id/sender-key-distribution`
Distribute an encrypted sender key to a specific member.

### `GET /api/v1/channels/:id/sender-key-distributions`
Fetch pending sender key distributions for the authenticated user.

---

## Direct Messages — `/api/v1/conversations/*`

### `GET /api/v1/conversations`
List all DM conversations.

### `POST /api/v1/conversations`
```json
{ "participant_id": "uuid" }
// Response 201
{ "conversation_id": "uuid", "prekey_bundle": { ... } }
```

### `GET /api/v1/conversations/:id/messages`
Message history (encrypted).

---

## Keys — `/api/v1/keys/*`

### `GET /api/v1/keys/prekey-bundle/:user_id`
Fetch a user's prekey bundle for X3DH.
```json
{
  "identity_key": "base64", "signed_prekey": "base64",
  "signed_prekey_sig": "base64", "one_time_prekey": "base64",
  "one_time_prekey_id": 42
}
```

### `POST /api/v1/keys/identity`
Upload identity + signed prekey.

### `POST /api/v1/keys/one-time`
Upload batch of one-time prekeys.

### `GET /api/v1/keys/one-time/count`
```json
{ "count": 15 }
```

### `GET /api/v1/keys/backup`
Download encrypted key backup blob.

### `PUT /api/v1/keys/backup`
Upload encrypted key backup.
```json
{ "salt": "base64", "ciphertext": "base64" }
```

---

## Explore — `/api/v1/explore/*`

### `GET /api/v1/explore/communities?tag=gaming&limit=20&offset=0`
Paginated public server list.

### `GET /api/v1/explore/live-servers`
Servers with recent activity (last 15 minutes).

### `GET /api/v1/explore/trending-tags`
Top 20 tags by server count (5-minute in-memory cache).

### `GET /api/v1/explore/search?q=minecraft&limit=20`
Full-text search across servers and users (pg_trgm).
```json
{
  "servers": [{ "id": "...", "name": "...", "member_count": 100 }],
  "users":   [{ "id": "...", "username": "...", "display_name": "..." }]
}
```

### `GET /api/v1/explore/top-yappers`
Top 20 users by follower count.

---

## Canvas — `/api/v1/canvas/*`

### `GET /api/v1/canvas/:server_id`
Current canvas state (music + active polls).

### `GET /api/v1/canvas/:server_id/clips`
Encrypted clip list for the server.

### `PATCH /api/v1/canvas/:server_id/music`
Admin only.
```json
{ "title": "Song Name", "artist": "Artist", "album_art_url": "...", "duration_sec": 240 }
```

### `POST /api/v1/canvas/:server_id/polls`
Admin only.
```json
{ "question": "Best language?", "options": ["Rust", "Go", "Zig"], "ends_at": "2026-..." }
```

### `POST /api/v1/canvas/:server_id/polls/:poll_id/vote`
```json
{ "option_index": 0 }
```
Returns `409` if already voted.

---

## Parental Controls — `/api/v1/parental/*`

### `POST /api/v1/parental/children`
Create a child account (COPPA — DOB must be < 18 years ago).
```json
// Request
{
  "username": "kiddo", "display_name": "Junior",
  "email": "junior@example.com", "password": "...",
  "date_of_birth": "2015-06-15"
}
// Response 201
{ "child_id": "uuid", "account_type": "child", "parental_controls_enabled": true }
```

### `GET /api/v1/parental/children`
List managed children.

### `GET /api/v1/parental/children/:child_id/overview`
Pending counts + top servers.

### `GET /api/v1/parental/children/:child_id/notifications`
Pending friend requests + server join requests. Marks alerts as read.

### `PATCH /api/v1/parental/friend-requests/:id/approve`
Approve a pending friend request → creates `friendships` row.

### `PATCH /api/v1/parental/friend-requests/:id/decline`

### `PATCH /api/v1/parental/server-joins/:id/approve`
Approve a pending server join → inserts `server_memberships`.

### `PATCH /api/v1/parental/server-joins/:id/decline`

---

## Screen Time — `/api/v1/screentime/*` + parental read/update

### `POST /api/v1/screentime/report`
Child device usage ingestion (metadata only). Typically called by authenticated child clients.
```json
// Request
{
  "recordedDate": "2026-03-03",
  "platform": "ios",
  "apps": [
    { "appName": "Yapper", "durationSeconds": 3600 },
    { "appName": "YouTube", "durationSeconds": 1200 }
  ]
}

// Response 201
{
  "status": "ok",
  "recordedDate": "2026-03-03",
  "itemsUpserted": 2,
  "platform": "ios"
}
```

Validation:
- `platform` must be one of `ios | android | web | desktop`
- `apps` max length `64`
- `durationSeconds` range `0..86400`

### `GET /api/v1/parental/children/:child_id/screentime?period=today|week|month`
Parent-only route (must manage the child account).
```json
{
  "period": "week",
  "rangeStart": "2026-02-26",
  "rangeEnd": "2026-03-03",
  "totalMinutesToday": 154,
  "limitMinutes": 180,
  "appBreakdown": [
    { "appName": "Yapper", "icon": "🟣", "minutes": 72 },
    { "appName": "YouTube", "icon": "🔴", "minutes": 30 }
  ],
  "weeklyData": [
    { "day": "Mon", "yapperMinutes": 60, "otherMinutes": 40 }
  ],
  "bedtimeStart": "22:00",
  "bedtimeEnd": "07:00"
}
```

### `PATCH /api/v1/parental/children/:child_id/screentime`
Parent-only route to update daily limit and bedtime window.
```json
// Request
{
  "limitMinutes": 180,
  "bedtimeStart": "22:00",
  "bedtimeEnd": "07:00"
}

// Response 200
{
  "status": "ok",
  "childId": "uuid",
  "limitMinutes": 180,
  "bedtimeStart": "22:00",
  "bedtimeEnd": "07:00"
}
```

---

## Media — `/api/v1/media/*`

### `POST /api/v1/media/upload-url`
Get a presigned R2 PUT URL. Client encrypts the file with AES-256-GCM before uploading.
```json
// Request
{ "filename": "audio.enc", "content_type": "application/octet-stream", "size_bytes": 102400 }
// Response
{ "upload_url": "https://...", "media_id": "uuid", "expires_in": 300 }
```

### `GET /api/v1/media/:id/download-url`
Get a presigned R2 GET URL for an encrypted blob.

---

## Notifications — `/api/v1/notifications/*`

### `POST /api/v1/notifications/register-device`
Register a device for FCM push notifications.
```json
{ "fcm_token": "...", "platform": "web" }   // platform: web | ios | android
```

### `DELETE /api/v1/notifications/register-device`
Unregister the current device token.

---

## WebSocket — `wss://api.yapperhq.com/ws`

Connect, then immediately send an `Auth` frame.

### Inbound (client → server)

```json
{ "type": "auth",        "token": "jwt" }
{ "type": "reauth",      "token": "jwt" }
{ "type": "ping" }
{ "type": "send_dm",     "conversation_id": "uuid", "ciphertext": "base64",
                         "ephemeral_key": "base64", "opk_id": 42, "msg_num": 0 }
{ "type": "send_channel","channel_id": "uuid", "ciphertext": "base64",
                         "message_type": "text", "msg_num": 1 }
{ "type": "typing_start","channel_id": "uuid" }
{ "type": "read",        "message_id": "uuid", "channel_id": "uuid" }
```

### Outbound (server → client)

```json
{ "type": "pong" }
{ "type": "error", "message": "Rate limit exceeded" }
{ "type": "message", "payload": { "id": "...", "channel_id": "...", "ciphertext": "..." } }
{ "type": "typing",      "channel_id": "uuid", "user_id": "uuid" }
{ "type": "typing_stop", "channel_id": "uuid", "user_id": "uuid" }
{ "type": "read_receipt","channel_id": "uuid", "message_id": "uuid", "user_id": "uuid" }
{ "type": "presence",    "user_id": "uuid", "online": true, "away": false }
{ "type": "canvas_update","payload": { "type": "music_update", ... } }
{ "type": "parent_notification", "payload": { "type": "friend_request", "child_id": "...", ... } }
```

### Rate Limits

- WS message rate: 5 messages/second per user, burst of 20
- Exceeding the limit returns `{ "type": "error", "message": "Rate limit exceeded" }` and the message is dropped (connection stays open)

---

## Canvas (Expanded) — `/api/v1/canvas/:server_id/*`

### Music

### `GET /api/v1/canvas/:server_id/music/queue`
Get the current music queue for the server.

### `POST /api/v1/canvas/:server_id/music/queue`
Add a track to the queue.

### `DELETE /api/v1/canvas/:server_id/music/queue/:track_id`
Remove a track from the queue.

### `POST /api/v1/canvas/:server_id/music/queue/reorder`
Reorder the queue.
```json
{ "track_ids": ["uuid1", "uuid2", "uuid3"] }
```

### `POST /api/v1/canvas/:server_id/music/skip`
Skip the current track.

### `POST /api/v1/canvas/:server_id/music/dj/request`
Request the DJ role.

### `DELETE /api/v1/canvas/:server_id/music/dj`
Release the DJ role.

### `GET /api/v1/canvas/:server_id/music/history`
Get play history.

### `GET /api/v1/canvas/:server_id/music/settings`
Get music settings.

### `PATCH /api/v1/canvas/:server_id/music/settings`
Update music settings (admin only).

### Polls

### `POST /api/v1/canvas/:server_id/polls`
Create a poll (types: `binary`, `emoji`, `multiple_choice`).

### `POST /api/v1/canvas/:server_id/polls/:id/vote`
Cast a vote.

### `POST /api/v1/canvas/:server_id/polls/:id/close`
Close a poll (admin only).

### `GET /api/v1/canvas/:server_id/polls/:id/results`
Get poll results.

### Clips

### `POST /api/v1/canvas/:server_id/clips/:id/reactions`
Add a reaction to a clip.

### `DELETE /api/v1/canvas/:server_id/clips/:id/reactions/:emoji`
Remove a reaction.

### `POST /api/v1/canvas/:server_id/clips/:id/pin`
Pin a clip (admin only).

### `DELETE /api/v1/canvas/:server_id/clips/:id/pin`
Unpin a clip.

### Events

### `POST /api/v1/canvas/:server_id/events`
Create a countdown event.

### `GET /api/v1/canvas/:server_id/events`
List events.

### `PATCH /api/v1/canvas/:server_id/events/:id`
Update an event.

### `DELETE /api/v1/canvas/:server_id/events/:id`
Delete an event.

### State

### `GET /api/v1/canvas/:server_id/state`
Full canvas state hydration (music + polls + clips + events).

---

## Media (Upload) — `/api/v1/media/*`

### `POST /api/v1/media/upload-url`
Get an R2 presigned upload URL.
```json
// Request
{ "media_type": "yap", "content_length": 102400 }   // media_type: yap | clip

// Response
{ "upload_url": "https://...", "media_id": "uuid", "expires_in": 300 }
```

---

## Support — `/api/v1/support/*`

### `POST /api/v1/support/tickets`
Create a support ticket.
```json
// Request
{ "ticket_type": "bug", "subject": "...", "description": "...", "priority": "medium" }

// Response 201
{ "id": "uuid", "status": "open", "created_at": "2026-..." }
```

### `GET /api/v1/support/tickets`
List own tickets.
```json
{ "tickets": [{ "id": "...", "ticket_type": "bug", "subject": "...", "status": "open", "created_at": "..." }] }
```
