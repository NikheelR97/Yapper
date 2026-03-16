# API Reference

Base URL: `https://api.yapperhq.com`

All authenticated endpoints require:
- `Authorization: Bearer <access_token>` header
- `X-CSRF-Token: <csrf_token>` header on mutating requests (POST, PUT, PATCH, DELETE)

---

## Health

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | No | Returns `{"status":"ok","db":true}` |

---

## WebSocket

| Path | Description |
|------|-------------|
| `wss://api.yapperhq.com/ws?token=<access_token>` | Real-time event stream |

---

## Auth v1

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/auth/register` | No | Create account |
| POST | `/api/v1/auth/login` | No | Login (legacy single-device) |
| POST | `/api/v1/auth/refresh` | Cookie | Refresh access token |
| POST | `/api/v1/auth/logout` | Yes | Invalidate refresh token |
| POST | `/api/v1/auth/verify-email` | No | Verify email with token |
| POST | `/api/v1/auth/forgot-password` | No | Send reset email |
| POST | `/api/v1/auth/reset-password` | No | Reset password with token |

## Auth v2 (multi-device)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v2/auth/login` | No | Login with device bootstrap |
| POST | `/api/v2/auth/refresh` | Cookie | Refresh with device context |
| DELETE | `/api/v2/auth/logout` | Yes | Logout current device |

## OAuth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/auth/oauth/discord` | No | Redirect to Discord OAuth |
| GET | `/auth/oauth/discord/callback` | No | Discord OAuth callback |
| GET | `/auth/oauth/google` | No | Redirect to Google OAuth |
| GET | `/auth/oauth/google/callback` | No | Google OAuth callback |

---

## Users

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/users/me` | Yes | Current user profile |
| PATCH | `/api/v1/users/me` | Yes | Update profile |
| POST | `/api/v1/users/me/avatar` | Yes | Upload avatar (WebP 256×256) |
| POST | `/api/v1/users/me/banner` | Yes | Upload banner (WebP 1500×500) |
| GET | `/api/v1/users/:id` | Yes | Public user profile |
| GET | `/api/v1/users/:id/presence` | Yes | User presence status |
| POST | `/api/v1/users/:id/follow` | Yes | Follow user |
| DELETE | `/api/v1/users/:id/follow` | Yes | Unfollow user |
| POST | `/api/v1/users/:id/friend-request` | Yes | Send friend request |
| GET | `/api/v1/users/me/friend-requests` | Yes | List incoming friend requests |
| PATCH | `/api/v1/users/me/friend-requests/:id` | Yes | Accept/decline friend request |
| DELETE | `/api/v1/users/me/connections/:provider` | Yes | Unlink OAuth provider |

## Account

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/account/data-export` | Yes | Download GDPR data export (ZIP) |
| DELETE | `/api/v1/account` | Yes | Soft-delete account |
| GET | `/api/v1/account/settings/privacy` | Yes | Get privacy settings |
| PATCH | `/api/v1/account/settings/privacy` | Yes | Update privacy settings |
| GET | `/api/v1/account/settings/appearance` | Yes | Get appearance settings |
| PATCH | `/api/v1/account/settings/appearance` | Yes | Update appearance settings |
| GET | `/api/v1/account/settings/notifications` | Yes | Get notification preferences |
| PATCH | `/api/v1/account/settings/notifications` | Yes | Update notification preferences |

---

## Devices

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v2/devices` | Yes | List user's devices |
| DELETE | `/api/v2/devices/:id` | Yes | Revoke device |
| PATCH | `/api/v2/devices/:id/trust` | Yes | Approve pending device |

---

## Signal Keys

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/keys/identity` | Yes | Upload identity + prekeys |
| GET | `/api/v1/keys/bundle/:user_id` | Yes | Fetch key bundle for X3DH |
| POST | `/api/v1/keys/one-time` | Yes | Upload one-time prekeys |
| GET | `/api/v1/keys/opk-count` | Yes | Remaining OPK count |
| GET | `/api/v1/keys/backup` | Yes | Fetch PIN-encrypted key backup |
| PUT | `/api/v1/keys/backup` | Yes | Store PIN-encrypted key backup |
| POST | `/api/v2/keys/device` | Yes | Upload device Signal keys |
| GET | `/api/v2/keys/bundle/:user_id/:device_id` | Yes | Per-device key bundle |

---

## Servers

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/servers` | Yes | List joined servers |
| POST | `/api/v1/servers` | Yes | Create server |
| GET | `/api/v1/servers/:id` | Yes | Get server details |
| PATCH | `/api/v1/servers/:id` | Yes | Update server (owner only) |
| DELETE | `/api/v1/servers/:id` | Yes | Delete server (owner only) |
| POST | `/api/v1/servers/:id/invites` | Yes | Generate invite link |
| POST | `/api/v1/servers/join/:code` | Yes | Join via invite code |
| DELETE | `/api/v1/servers/:id/members/:user_id` | Yes | Kick member / leave |

## Channels

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/channels/:id` | Yes | Get channel |
| POST | `/api/v1/servers/:id/channels` | Yes | Create channel |
| PATCH | `/api/v1/channels/:id` | Yes | Update channel |
| DELETE | `/api/v1/channels/:id` | Yes | Delete channel |

---

## Messages

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/channels/:id/messages` | Yes | Channel message history |
| POST | `/api/v1/channels/:id/messages` | Yes | Send channel message (v1) |
| POST | `/api/v2/conversations/:id/messages` | Yes | Send DM (v2 envelope) |
| GET | `/api/v1/conversations` | Yes | List DM conversations |
| GET | `/api/v1/conversations/:id/messages` | Yes | DM history |
| POST | `/api/v1/conversations` | Yes | Create DM conversation |
| POST | `/api/v1/channels/:id/typing` | Yes | Send typing start |
| POST | `/api/v1/channels/:id/read` | Yes | Mark messages read |

---

## Media

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/media/upload-url` | Yes | Get presigned R2 upload URL |

---

## Explore

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/explore/servers` | Yes | Search public servers |
| GET | `/api/v1/explore/trending-tags` | Yes | Trending server tags (5-min cache) |
| GET | `/api/v1/explore/communities` | Yes | Featured communities |
| GET | `/api/v1/explore/live-servers` | Yes | Active live servers |

---

## Canvas

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/servers/:id/canvas` | Yes | Get canvas state + clips |
| PATCH | `/api/v1/servers/:id/canvas/music` | Yes | Update music widget |
| POST | `/api/v1/servers/:id/canvas/polls` | Yes | Create poll |
| POST | `/api/v1/canvas/polls/:id/vote` | Yes | Vote on poll |

---

## Premium

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/premium/status` | Yes | Subscription status |
| POST | `/api/v1/premium/promo` | Yes | Redeem promo code |
| POST | `/api/v1/premium/webhook` | No | Stripe webhook endpoint |

---

## Parental Controls

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/parental/children` | Yes | Create child account (COPPA) |
| GET | `/api/v1/parental/children` | Yes | List managed children |
| GET | `/api/v1/parental/children/:id/overview` | Yes | Child activity snapshot |
| GET | `/api/v1/parental/children/:id/notifications` | Yes | Pending approval alerts |
| PATCH | `/api/v1/parental/friend-requests/:id/approve` | Yes | Approve friend request |
| PATCH | `/api/v1/parental/friend-requests/:id/decline` | Yes | Decline friend request |
| PATCH | `/api/v1/parental/server-joins/:id/approve` | Yes | Approve server join |
| PATCH | `/api/v1/parental/server-joins/:id/decline` | Yes | Decline server join |

---

## Support

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/support/tickets` | Yes | Submit support ticket |
| GET | `/api/v1/support/tickets` | Yes | List own tickets |

### POST /api/v1/support/tickets

```json
{
  "ticket_type": "bug",          // "bug" | "idea" | "improvement"
  "subject": "Login fails",      // 1–200 chars
  "description": "Steps to…",   // 1–2000 chars
  "priority": "high"             // "low" | "medium" | "high" | "urgent"
}
```

---

## Notifications

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/notifications/device-token` | Yes | Register FCM push token |
| DELETE | `/api/v1/notifications/device-token` | Yes | Unregister FCM token |
