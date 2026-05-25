# API Reference

Base URL: `https://api.yapperhq.com`

The documented HTTP API surface is `/api/v2/*`. OAuth browser redirects remain versionless under `/auth/oauth/*`.

All authenticated endpoints require:
- `Authorization: Bearer <access_token>` header
- `X-CSRF-Token: <csrf_token>` header on mutating requests

---

## Health

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | No | Returns `{"status":"ok","db":true}` |

---

## WebSocket

| Path | Description |
|------|-------------|
| `wss://api.yapperhq.com/ws` | Real-time event stream. First client frame must be `{"type":"auth","token":"<access_token>"}` |

---

## Auth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v2/auth/register` | No | Create account and device-bound session |
| POST | `/api/v2/auth/login` | No | Login with device bootstrap |
| POST | `/api/v2/auth/oauth/exchange` | No | Exchange OAuth code for a device-aware session |
| POST | `/api/v2/auth/refresh` | Cookie | Refresh access token |
| DELETE | `/api/v2/auth/logout` | Yes | Invalidate current refresh-token family |

## OAuth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/auth/oauth/discord` | No | Redirect to Discord OAuth |
| GET | `/auth/oauth/discord/callback` | No | Discord OAuth callback |
| GET | `/auth/oauth/google` | No | Redirect to Google OAuth |
| GET | `/auth/oauth/google/callback` | No | Google OAuth callback |

---

## Devices

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v2/devices` | Yes | List user's devices |
| DELETE | `/api/v2/devices/:id` | Yes | Revoke device |
| PATCH | `/api/v2/devices/:id/trust` | Yes | Approve pending device |

## Users and Account

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v2/users/me` | Yes | Current user profile |
| PATCH | `/api/v2/users/me` | Yes | Update profile |
| POST | `/api/v2/users/me/avatar` | Yes | Upload avatar |
| POST | `/api/v2/users/me/banner` | Yes | Upload banner |
| GET | `/api/v2/users/:id` | Yes | Public user profile |
| GET | `/api/v2/users/:id/presence` | Yes | User presence status |
| POST | `/api/v2/users/:id/follow` | Yes | Follow user |
| DELETE | `/api/v2/users/:id/follow` | Yes | Unfollow user |
| GET | `/api/v2/account/data-export` | Yes | Download data export |
| DELETE | `/api/v2/account` | Yes | Delete account |
| GET | `/api/v2/account/settings/privacy` | Yes | Get privacy settings |
| PATCH | `/api/v2/account/settings/privacy` | Yes | Update privacy settings |
| GET | `/api/v2/account/settings/appearance` | Yes | Get appearance settings |
| PATCH | `/api/v2/account/settings/appearance` | Yes | Update appearance settings |
| GET | `/api/v2/account/settings/notifications` | Yes | Get notification preferences |
| PATCH | `/api/v2/account/settings/notifications` | Yes | Update notification preferences |

---

## Signal Keys

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v2/keys/device` | Yes | Upload device Signal keys |
| GET | `/api/v2/keys/bundle/:user_id/:device_id` | Yes | Fetch per-device key bundle |
| POST | `/api/v2/keys/backup` | Yes | Store PIN-encrypted key backup |
| GET | `/api/v2/keys/backup` | Yes | Fetch PIN-encrypted key backup |

---

## Messaging, Servers, Media

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v2/servers` | Yes | List joined servers |
| POST | `/api/v2/servers` | Yes | Create server |
| GET | `/api/v2/servers/:id` | Yes | Get server details |
| PATCH | `/api/v2/servers/:id` | Yes | Update server |
| DELETE | `/api/v2/servers/:id` | Yes | Delete server |
| POST | `/api/v2/servers/:id/invites` | Yes | Generate invite link |
| POST | `/api/v2/servers/join/:code` | Yes | Join via invite code |
| GET | `/api/v2/channels/:id/messages` | Yes | Channel message history |
| POST | `/api/v2/channels/:id/messages` | Yes | Send channel message |
| GET | `/api/v2/conversations` | Yes | List DM conversations |
| GET | `/api/v2/conversations/:id/messages` | Yes | DM history |
| POST | `/api/v2/conversations` | Yes | Create DM conversation |
| POST | `/api/v2/media/upload-url` | Yes | Get presigned R2 upload URL |

---

## Discovery, Canvas, Premium, Parental, Support, Notifications

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v2/explore/servers` | Yes | Search public servers |
| GET | `/api/v2/explore/trending-tags` | Yes | Trending server tags |
| GET | `/api/v2/explore/communities` | Yes | Featured communities |
| GET | `/api/v2/explore/live-servers` | Yes | Active live servers |
| GET | `/api/v2/servers/:id/canvas` | Yes | Get canvas state |
| PATCH | `/api/v2/servers/:id/canvas/music` | Yes | Update music widget |
| POST | `/api/v2/servers/:id/canvas/polls` | Yes | Create poll |
| POST | `/api/v2/canvas/polls/:id/vote` | Yes | Vote on poll |
| GET | `/api/v2/premium/status` | Yes | Subscription status |
| POST | `/api/v2/premium/promo` | Yes | Redeem promo code |
| POST | `/api/v2/parental/children` | Yes | Create child account |
| GET | `/api/v2/parental/children` | Yes | List managed children |
| GET | `/api/v2/parental/children/:id/overview` | Yes | Child activity snapshot |
| GET | `/api/v2/parental/children/:id/notifications` | Yes | Pending approval alerts |
| PATCH | `/api/v2/parental/friend-requests/:id/approve` | Yes | Approve friend request |
| PATCH | `/api/v2/parental/friend-requests/:id/decline` | Yes | Decline friend request |
| PATCH | `/api/v2/parental/server-joins/:id/approve` | Yes | Approve server join |
| PATCH | `/api/v2/parental/server-joins/:id/decline` | Yes | Decline server join |
| POST | `/api/v2/support/tickets` | Yes | Submit support ticket |
| GET | `/api/v2/support/tickets` | Yes | List own tickets |
| POST | `/api/v2/support/webhooks/hubspot` | No | HubSpot status webhook |
| PUT | `/api/v2/notifications/push-token` | Yes | Register or update push token |
| DELETE | `/api/v2/notifications/push-token` | Yes | Unregister push token |
