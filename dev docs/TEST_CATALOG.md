# Yapper Test Catalog

> Last updated: 2026-05-27
>
> **217 backend unit tests** | **191 frontend E2E tests** across **49 spec files**

---

## Table of Contents

- [Backend Unit Tests (Rust)](#backend-unit-tests-rust)
- [Frontend E2E Tests (Playwright)](#frontend-e2e-tests-playwright)
- [Test Infrastructure](#test-infrastructure)
- [Latest Run Results](#latest-run-results)

---

## Backend Unit Tests (Rust)

Run: `cargo test --lib --bins` from `backend/`

### auth/handlers.rs (2 tests)

| Test | Purpose |
|------|---------|
| `refresh_cookie_header_omits_secure_flag_for_local_http` | Cookie omits `Secure` on HTTP |
| `refresh_cookie_header_includes_secure_flag_for_https` | Cookie includes `Secure` on HTTPS |

### auth/middleware.rs (7 tests)

| Test | Purpose |
|------|---------|
| `trusted_auth_user_device_is_allowed` | Trusted device passes auth |
| `pending_auth_user_device_is_forbidden` | Pending device gets 403 |
| `revoked_auth_user_device_is_unauthorized` | Revoked device gets 401 |
| `standard_accounts_require_device_binding` | Non-bot accounts must bind a device |
| `bot_accounts_can_remain_deviceless` | Bot accounts skip device binding |
| `test_login_rate_limiter_locks_after_5_failures` | Rate limiter locks after 5 bad logins |
| `test_login_rate_limiter_clears_on_success` | Rate limiter resets on success |

### auth/oauth.rs (2 tests)

| Test | Purpose |
|------|---------|
| `linked_identity_allows_same_subject_reuse` | Same OAuth subject can re-link |
| `linked_identity_rejects_subject_mismatch_for_same_provider` | Different subject blocked for same provider |

### auth/service.rs (4 tests)

| Test | Purpose |
|------|---------|
| `test_password_hash_and_verify` | Argon2id hash round-trip |
| `test_password_too_long_rejected` | Rejects password > MAX_PASSWORD_LENGTH |
| `test_email_token_is_64_hex_chars` | Email verification token format |
| `test_email_tokens_are_unique` | Successive tokens differ |

### canvas/types.rs (10 tests)

| Test | Purpose |
|------|---------|
| `music_input_deserializes_with_all_fields` | Full music input deserialization |
| `music_input_deserializes_with_required_fields_only` | Minimal music input |
| `music_input_rejects_unknown_fields` | Strict deserialization |
| `enqueue_track_deserializes_valid` | Valid track enqueue |
| `enqueue_track_rejects_missing_duration` | Duration required |
| `create_poll_defaults_to_multiple_choice` | Poll type defaults |
| `create_poll_binary_without_options` | Binary poll without options |
| `vote_req_rejects_unknown_fields` | Vote strict deserialization |
| `add_reaction_deserializes` | Reaction deserialization |
| `create_event_deserializes` | Event deserialization |
| `constants_are_sensible` | Canvas constants sanity |

### channels/service.rs (1 test)

| Test | Purpose |
|------|---------|
| `key_dist_member_lookup_uses_correct_table` | Key distribution queries correct table |

### csrf.rs (24 tests)

| Test | Purpose |
|------|---------|
| `exempt_auth_login` | /auth/login exempt from CSRF |
| `exempt_auth_register` | /auth/register exempt |
| `exempt_auth_verify_email` | /auth/verify-email exempt |
| `exempt_auth_password_reset_request` | /auth/password-reset-request exempt |
| `exempt_auth_password_reset_confirm` | /auth/password-reset-confirm exempt |
| `exempt_auth_refresh` | /auth/refresh exempt |
| `exempt_auth_oauth_exchange` | /auth/oauth/*/callback exempt |
| `exempt_premium_webhook` | /premium/webhook exempt |
| `exempt_support_webhooks_hubspot` | /support/webhooks/hubspot exempt |
| `exempt_list_is_exactly_nine_entries` | Allowlist size guard |
| `non_exempt_random_api_path` | Random path not exempt |
| `non_exempt_partial_match` | Partial path not exempt |
| `non_exempt_auth_logout` | /auth/logout not exempt |
| `non_exempt_root` | Root path not exempt |
| `non_exempt_empty` | Empty path not exempt |
| `non_exempt_premium_without_webhook` | /premium not exempt |
| `non_exempt_support_tickets` | /support/tickets not exempt |
| `non_exempt_auth_prefix_only` | /auth not exempt |
| `normalize_strips_trailing_slash` | Path normalization |
| `normalize_no_trailing_slash_unchanged` | No-op normalization |
| `normalize_root_slash_stays` | Root slash preserved |
| `normalize_multiple_trailing_slashes` | Multi-slash normalization |
| `cookie_header_contains_token_name` | CSRF cookie format |
| `clear_cookie_sets_max_age_zero` | Cookie clearing |

### devices/mod.rs (2 tests)

| Test | Purpose |
|------|---------|
| `validate_sync_public_key_accepts_32_byte_key` | Valid 32-byte sync key |
| `validate_sync_public_key_rejects_wrong_length` | Invalid sync key length |

### emojis/mod.rs (4 tests)

| Test | Purpose |
|------|---------|
| `emoji_name_accepts_valid_names` | Valid emoji name patterns |
| `emoji_name_rejects_invalid_names` | Invalid emoji name patterns |
| `emoji_limits_differ_by_tier` | Free vs premium emoji limits |
| `webp_conversion_produces_correct_dimensions` | WebP 64x64 output |

### explore/mod.rs (19 tests)

| Test | Purpose |
|------|---------|
| `search_query_deserializes_valid` | Valid search query |
| `search_query_deserializes_empty_string` | Empty search query |
| `search_query_rejects_missing_q` | Missing `q` param |
| `search_query_rejects_unknown_fields` | Strict query deserialization |
| `search_query_rejects_non_string_q` | Non-string `q` rejected |
| `search_trim_removes_whitespace` | Whitespace trimming |
| `search_trim_all_whitespace_is_empty` | All-whitespace = empty |
| `search_query_length_at_boundary` | 255-char boundary |
| `search_query_unicode_length_is_byte_len` | Unicode byte-length check |
| `tag_cache_ttl_is_five_minutes` | Cache TTL = 5 min |
| `tag_cache_fresh_entry_is_not_expired` | Fresh cache not expired |
| `tag_cache_data_preserves_values` | Cache value preservation |
| `tag_cache_empty_data_is_valid` | Empty cache valid |
| `tag_cache_mutex_starts_none` | Cache starts empty |
| `tag_cache_mutex_can_store_and_retrieve` | Cache store/retrieve |
| `tag_cache_mutex_can_be_cleared` | Cache clearable |
| `community_json_shape` | Community JSON schema |
| `search_empty_response_shape` | Empty search response |
| `top_yapper_json_shape` | Top yapper JSON schema |
| `trending_tag_json_shape` | Trending tag JSON schema |
| `live_server_json_shape_with_last_active` | Live server JSON schema |
| `user_search_result_json_shape` | User search result schema |
| `user_search_result_friend_permission_variants` | Friend permission enum |

### hub.rs (46 tests)

| Test | Purpose |
|------|---------|
| `test_hub_register_unregister` | Connection lifecycle |
| `test_hub_max_connections_per_user` | Max 5 connections per user |
| `test_hub_send_to_offline_user_is_noop` | Offline send is no-op |
| `test_hub_try_send_to_user_returns_false_when_queue_full` | Backpressure on full queue |
| `test_hub_try_send_to_device_returns_false_when_queue_full` | Device queue backpressure |
| `test_dm_v2_replay_payload_includes_ratchet_metadata` | Ratchet metadata in DM replay |
| `test_mark_dm_delivered_advances_state_after_success` | Delivery state machine |
| `test_pending_sync_events_keep_state_on_queue_full` | Sync events preserved on backpressure |
| `test_pending_sync_events_require_ack_after_successful_ws_handoff` | Sync ACK pattern |
| `test_pending_key_dists_keep_state_on_queue_full` | Key dist preserved on backpressure |
| `test_check_msg_rate_allows_burst` | Rate limiter allows burst (20 msgs) |
| `test_check_msg_rate_rejects_after_burst` | Rate limiter rejects over burst |
| `test_check_msg_rate_independent_per_user` | Per-user rate limiting |
| `test_rate_limiter_cleaned_up_on_unregister` | Rate limiter cleanup |
| `test_ws_inbound_ping` | WS ping parsing |
| `test_ws_inbound_auth` | WS auth parsing |
| `test_ws_inbound_reauth` | WS reauth parsing |
| `test_ws_inbound_send_dm_full` | WS DM full payload |
| `test_ws_inbound_send_dm_optional_fields_null` | WS DM optional nulls |
| `test_ws_inbound_send_channel` | WS channel message |
| `test_ws_inbound_send_channel_optional_fields_missing` | WS channel optional fields |
| `test_ws_inbound_typing_start` | WS typing start |
| `test_ws_inbound_read` | WS read receipt |
| `test_ws_inbound_unknown_type_fails` | Unknown WS type rejected |
| `test_ws_inbound_missing_required_field_fails` | Missing required field |
| `test_ws_outbound_pong` | Outbound pong serialization |
| `test_ws_outbound_ready` | Outbound ready serialization |
| `test_ws_outbound_presence_online` | Online presence JSON |
| `test_ws_outbound_presence_offline_with_last_seen` | Offline presence JSON |
| `test_ws_outbound_presence_away` | Away presence JSON |
| `test_redact_last_seen_for_broadcast_hides_when_disabled` | last_seen redacted when disabled |
| `test_redact_last_seen_for_broadcast_keeps_when_enabled` | last_seen kept when enabled |
| `test_ws_outbound_message` | Outbound message JSON |
| `test_ws_outbound_error` | Outbound error JSON |
| `test_ws_outbound_re_auth_required` | Outbound reauth JSON |
| `test_ws_outbound_typing` | Outbound typing JSON |
| `test_ws_outbound_typing_stop` | Outbound typing stop JSON |
| `test_ws_outbound_read_receipt` | Outbound read receipt JSON |
| `test_ws_outbound_canvas_update` | Outbound canvas update JSON |
| `test_ws_outbound_parent_notification` | Outbound parent notification |
| `test_ws_outbound_serializes_to_valid_json_string` | All outbound variants valid JSON |
| `test_hub_broadcast_sends_to_registered_users` | Broadcast delivery |
| `test_hub_broadcast_respects_fanout_limit` | Fanout limit enforced |
| `test_hub_send_to_user_delivers_to_all_connections` | Multi-connection delivery |
| `test_hub_device_register_and_send` | Device-targeted send |
| `test_hub_device_offline_after_unregister` | Device offline after unregister |
| `test_hub_away_state` | Away state tracking |
| `test_hub_register_non_trusted_device_not_in_user_connections` | Untrusted device excluded |
| `test_send_ws_error_sends_error_variant` | Error variant delivery |
| `test_connection_id_unique` | ConnectionId uniqueness |
| `test_connection_id_clone_eq` | ConnectionId clone + eq |
| `ws_auth_rejects_pending_trust_device` | Pending trust device rejected |

### keys/mod.rs (5 tests)

| Test | Purpose |
|------|---------|
| `test_base64_roundtrip` | Base64 encode/decode |
| `parse_device_ids_filter_accepts_uuid_lists` | UUID list parsing |
| `parse_device_ids_filter_rejects_invalid_uuid` | Invalid UUID rejected |
| `restore_backup_request_defaults_to_non_destructive_mode` | Restore defaults to non-destructive |
| `restore_backup_request_allows_explicit_source_replacement` | Restore allows source replacement |

### media/handlers.rs (2 tests)

| Test | Purpose |
|------|---------|
| `test_allowed_types_include_yap_clip` | yap + clip in allowed types |
| `test_unknown_media_type_rejected` | Unknown media type rejected |

### media/r2.rs (3 tests)

| Test | Purpose |
|------|---------|
| `test_allowed_media_types_are_non_empty` | Allowed types non-empty |
| `test_allowed_media_types_contain_yap_and_clip` | yap + clip present |
| `test_object_key_format` | R2 object key format |

### parental/mod.rs (32 tests)

| Test | Purpose |
|------|---------|
| `constants_are_sensible` | Parental constants sanity |
| `deserialize_valid_input` | Valid child registration |
| `deserialize_rejects_unknown_fields` | Strict deserialization |
| `deserialize_rejects_missing_username` | Username required |
| `deserialize_rejects_missing_display_name` | Display name required |
| `deserialize_rejects_missing_email` | Email required |
| `deserialize_rejects_missing_password` | Password required |
| `deserialize_rejects_missing_date_of_birth` | DOB required |
| `deserialize_rejects_empty_json_object` | Empty object rejected |
| `deserialize_rejects_null` | Null rejected |
| `deserialize_rejects_array` | Array rejected |
| `deserialize_rejects_wrong_type_for_username` | Type validation (username) |
| `deserialize_rejects_wrong_type_for_date_of_birth` | Type validation (DOB) |
| `deserialize_accepts_empty_string_values` | Empty strings allowed |
| `deserialize_preserves_whitespace_in_fields` | Whitespace preserved |
| `deserialize_accepts_unicode_values` | Unicode values accepted |
| `coppa_rejects_adult_dob` | COPPA: adult rejected |
| `coppa_rejects_exactly_18` | COPPA: exactly 18 rejected |
| `coppa_accepts_under_18` | COPPA: under 18 accepted |
| `coppa_accepts_just_under_18` | COPPA: just under 18 |
| `coppa_accepts_newborn` | COPPA: newborn accepted |
| `coppa_rejects_invalid_date_format` | COPPA: bad date format |
| `coppa_rejects_future_date` | COPPA: future date rejected |
| `password_too_short` | Password too short |
| `password_exactly_min` | Password at minimum |
| `password_valid_length` | Password valid length |
| `password_too_long` | Password too long |
| `password_exactly_max` | Password at maximum |
| `username_too_short` | Username too short |
| `username_trims_to_too_short` | Trimmed username too short |
| `username_exactly_min` | Username at minimum |
| `username_valid` | Username valid |
| `username_too_long` | Username too long |
| `username_exactly_max` | Username at maximum |
| `display_name_empty` | Display name empty |
| `display_name_whitespace_only` | Display name whitespace |
| `display_name_valid` | Display name valid |
| `display_name_too_long` | Display name too long |
| `display_name_exactly_max` | Display name at maximum |
| `email_empty` | Email empty |
| `email_missing_at` | Email missing @ |
| `email_valid` | Email valid |
| `email_too_long` | Email too long |
| `email_whitespace_only` | Email whitespace only |
| `email_at_only` | Email @ only |
| `uid_pair_ordering_is_deterministic` | UID pair ordering |
| `uid_pair_ordering_same_id` | Same-ID UID pair |

### premium/service.rs (3 tests)

| Test | Purpose |
|------|---------|
| `stripe_signature_valid_for_recent_timestamp` | Stripe signature validation |
| `stripe_signature_rejects_replayed_timestamp` | Replay attack rejected |
| `subscription_status_only_grants_for_active_or_trialing` | Status gating |

### screentime/mod.rs (5 tests)

| Test | Purpose |
|------|---------|
| `parse_platform_rejects_unknown` | Unknown platform rejected |
| `period_range_rejects_invalid` | Invalid period range |
| `require_parent_of_pool_allows_linked_parent` | Linked parent allowed |
| `require_parent_of_pool_denies_unlinked_parent` | Unlinked parent denied |
| `refresh_daily_summary_aggregates_by_app_type` | Daily summary aggregation |
| `upsert_screentime_settings_updates_existing_row` | Settings upsert |

### servers/service.rs (4 tests)

| Test | Purpose |
|------|---------|
| `invite_expiry_none_means_never_expires` | No expiry = permanent |
| `invite_expiry_accepts_bounded_positive_hours` | Valid expiry hours |
| `invite_expiry_rejects_zero_and_negative_hours` | Invalid expiry hours |
| `invite_expiry_rejects_excessive_hours` | Excessive expiry rejected |

### support/mod.rs (4 tests)

| Test | Purpose |
|------|---------|
| `redact_support_text_scrubs_common_pii_and_secrets` | PII redaction |
| `verify_hubspot_signature_accepts_valid_and_rejects_invalid` | HubSpot signature verification |
| `map_hubspot_stage_maps_known_stages` | HubSpot stage mapping |
| `build_hubspot_content_excludes_direct_identifiers_by_default` | PII excluded from HubSpot |

### users/mod.rs (10 tests)

| Test | Purpose |
|------|---------|
| `parse_image_upload_success_path` | Image upload parsing |
| `parse_image_upload_rejects_invalid_content_type` | Invalid content type |
| `parse_image_upload_rejects_oversize_file` | Oversize file rejected |
| `parse_image_upload_rejects_missing_file_field` | Missing file field |
| `validate_image_dimensions_accepts_small_png` | Small PNG accepted |
| `validate_image_dimensions_rejects_oversized_dimension` | Oversized dimension rejected |
| `validate_image_dimensions_rejects_pixel_count_bomb` | Pixel bomb rejected |
| `validate_image_dimensions_rejects_nonsense_bytes` | Invalid bytes rejected |
| `visible_last_seen_allows_self_view_even_when_disabled` | Self-view last_seen |
| `visible_last_seen_redacts_peer_when_disabled` | Peer last_seen redacted |
| `build_data_export_zip_contains_expected_json_file` | GDPR export ZIP contents |

---

## Frontend E2E Tests (Playwright)

Run: `npx dotenv -e .env.test -- npx playwright test --project=chromium` from `frontend/`

### auth.spec.ts (11 tests)

- **Login page**
  - `renders form elements` @smoke
  - `submit button disabled when fields are empty` @smoke
  - `shows error banner on wrong credentials` @smoke
  - `navigates to /register via link` @smoke
  - `navigates to /forgot-password via link` @smoke
  - `OAuth buttons are present` @smoke
- **Register page**
  - `renders all form fields`
  - `submit button disabled until required fields are filled`
  - `password strength indicator appears after typing`
  - `navigates to /login via sign-in link`
- **Authenticated navigation**
  - `successful login redirects to /explore`

### auth-shell.spec.ts (1 test)

- **Authenticated shell**
  - `boots explore, covers settings sections, survives reload, and logs out` @smoke

### brute-force-auth.spec.ts (1 test)

- **Brute-force auth** @security @auth @mobile-layout
  - `successive authentication failures trigger rate-limit lockdown` @smoke

### canvas-auth.spec.ts (1 test)

- **Canvas auth** @security @regression
  - `non-admin members cannot enqueue tracks through the canvas API directly`

### channel-e2ee.spec.ts (2 tests)

- **Channel E2EE -- cross-user message decryption**
  - `User B can read a channel message sent by User A (Sender Key decryption regression)`
  - `Both users can send and receive messages in the same channel (bidirectional)`

### decryption-integrity.spec.ts (3 tests)

- **DM decryption integrity**
  - `Bidirectional DM -- both users decrypt each other without errors`
- **Channel decryption integrity**
  - `Cross-user channel messages decrypt without errors (Sender Key distribution)`
  - `Late joiner decrypts messages sent after joining (key_dist_request regression)`

### discord-import.spec.ts (3 tests)

- **Discord Import -- settings page** @smoke
  - `shows Discord as connected when user has linked account`
  - `shows Discord as not connected when no link exists`
- **Bot message display in channel** @smoke
  - `bot plaintext messages render without decryption errors`

### dm.spec.ts (4 tests)

- **DM index - authenticated**
  - `/dm page renders`
  - `Direct Messages nav link is present in sidebar`
  - `sidebar shows DM section when on /dm`
- **Seeded DM flow**
  - `User A can open the seeded DM with User B and send a message`

### emoji-rendering.spec.ts (3 tests)

- **Custom emoji rendering** @smoke
  - `renders :emoji_name: as <img> with safe URL`
  - `blocks javascript: protocol in emoji URLs (XSS prevention)`
- **Emoji picker integration** @smoke
  - `emoji picker button is visible in message input area`

### error-states.spec.ts (4 tests)

- **Error states -- 404**
  - `navigating to a non-existent route shows an error page` @smoke
  - `non-existent profile route shows error state`
- **Error states -- network failure**
  - `API failure on explore page shows error state`
- **Error states -- loading skeletons**
  - `profile page shows loading skeleton before data arrives` @smoke

### explore.spec.ts (2 tests)

- **Explore - unauthenticated**
  - `redirects to /login`
- **Explore - authenticated**
  - `renders core explore controls and handles join attempts`

### explore-advanced.spec.ts (8 tests)

- **Explore -- debounced search**
  - `search box is present and accepts input` @smoke
  - `search results render after user input` @smoke
- **Explore -- tag filtering**
  - `trending tags are visible` @smoke
  - `clicking a tag filters or highlights results`
- **Explore -- grid/list view toggle**
  - `communities render in the default view` @smoke
  - `list view toggle changes layout class or structure`
- **Explore -- user search results**
  - `searching for a username shows user row with display name`
  - `user row includes Add Friend or Follow button`

### forgot-password.spec.ts (5 tests)

- **Forgot password page**
  - `renders the heading and email input` @smoke
  - `submit button is disabled when email is empty` @smoke
  - `submit button becomes enabled after typing a valid email`
  - `submitting a valid email shows the success confirmation`
  - `clicking the back-to-login link navigates to /login`

### invite-expiry.spec.ts (2 tests)

- **Invite expiry** @security @auth
  - `expired or invalid invite code shows rejection message` @smoke
  - `empty invite code does not submit` @smoke

### keyboard-shortcuts.spec.ts (4 tests)

- **Keyboard shortcuts modal**
  - `Ctrl+/ opens the keyboard shortcuts modal` @smoke
  - `pressing Escape closes the keyboard shortcuts modal` @smoke
  - `pressing Ctrl+/ a second time closes the modal`
  - `modal lists at least some keyboard shortcuts`

### live-canvas.spec.ts (3 tests)

- **Live Canvas -- panel**
  - `canvas toggle button is present in channel header`
  - `opening the canvas panel reveals music widget and poll`
  - `poll vote button triggers POST to vote endpoint`

### media-messaging.spec.ts (5 tests)

- **Media messaging -- Yap recorder**
  - `clicking Record a Yap button shows the recorder UI` @smoke
- **Media messaging -- Clip recorder**
  - `clicking Record a Clip button shows the recorder UI` @smoke
- **Media messaging -- Yap cancel flow**
  - `cancelling Yap recording dismisses recorder without sending a message` @smoke
- **Media messaging -- Yap size limit**
  - `recording that exceeds size limit shows an error toast` @regression
- **Media messaging -- DM recorder buttons**
  - `Yap and Clip buttons exist in DM message input toolbar` @smoke

### mobile-responsive.spec.ts (9 tests)

- **Mobile -- Login page**
  - `login form is fully visible without horizontal scroll` @smoke
  - `brand panel (left side) is hidden on mobile` @smoke
- **Mobile -- Register page**
  - `register form is fully visible without horizontal scroll` @smoke
- **Mobile -- Explore page**
  - `search bar is visible on mobile` @smoke
  - `page renders without horizontal overflow` @smoke
- **Mobile -- Settings page**
  - `settings page loads and renders navigation` @smoke
  - `settings nav collapses to icon-only on mobile`
  - `page renders without horizontal overflow`
- **Mobile -- DM page**
  - `DM inbox page loads without horizontal overflow` @smoke

### multi-device.spec.ts (2 tests)

- **Multi-device auth**
  - `trusted primary device reaches the app shell and sees the pending secondary device`
  - `secondary device is held at the pending approval gate`

### multi-device-edge-cases.spec.ts (4 tests)

- **Multi-device -- WS device revocation** @multidevice @smoke
  - `WS error frame (code 4001, Device revoked) clears session and redirects to /login`
- **Multi-device -- offline approval persistence** @multidevice @smoke
  - `Approved pending device IDs persist in localStorage across reload`
- **Multi-device -- sync-events retry on 500** @multidevice
  - `App retries sync-events on HTTP 500 and eventually becomes ready`
- **Multi-device -- PendingDeviceGate backup restore link** @multidevice @smoke
  - `"Restore from encrypted backup" section visible on PendingDeviceGate`

### navigation.spec.ts (12 tests)

- **Unauthenticated redirects**
  - `/dm redirects to /login when not authenticated`
  - `/servers redirects to /login when not authenticated`
  - `/explore redirects to /login when not authenticated`
  - `/settings redirects to /login when not authenticated`
- **Public pages**
  - `root page loads`
  - `/login page title contains Yapper`
  - `/register page title contains Yapper`
  - `/forgot-password page loads`
- **Authenticated navigation**
  - `DM page renders`
  - `Servers page renders`
  - `Explore page renders`
  - `Settings page renders`

### offline-delivery.spec.ts (1 test)

- **Offline message delivery**
  - `User B receives a message that was sent while they were offline`

### onboarding.spec.ts (4 tests)

- **Onboarding carousel**
  - `step 1 renders with a visible title and dot pagination` @smoke
  - `clicking Next advances to the next step` @smoke
  - `clicking the first dot returns to step 1`
  - `completing all steps navigates to /explore or /login`

### parental-approval-gate.spec.ts (1 test)

- **Parental approval gate** @security @e2ee @coppa
  - `child DM creation is blocked for non-friends and pending approval does not unlock key bundles`

### parental-controls.spec.ts (6 tests)

- **Parental controls -- child setup wizard**
  - `step 1 renders profile fields (display name, username, email, password)` @smoke
  - `step 2 shows date of birth fields after completing step 1`
  - `COPPA warning appears for under-13 date of birth`
  - `step 3 shows safety toggles (friend requests, server joins, screen time)`
- **Parental controls -- parent dashboard**
  - `parent dashboard renders and shows safety section` @smoke
  - `pending alerts section shows alert items`

### presence.spec.ts (2 tests)

- **Presence -- online status**
  - `GET /api/v2/users/:id/presence returns online for a logged-in user`
  - `presence endpoint returns a valid presence state for another user`

### profile.spec.ts (6 tests)

- **Profile page -- mocked rendering**
  - `profile header shows display name and @username` @smoke
  - `bio card renders bio text` @smoke
  - `follow button renders and shows not-following state`
  - `profile with isFollowing=true shows following state`
- **Profile page -- own profile**
  - `bio card and hype moments section are visible on own profile` @smoke
- **Profile page -- live API follow action**
  - `clicking Follow increments the follower count`

### read-receipts.spec.ts (1 test)

- **Read receipts -- DM**
  - `User A sees a read receipt after User B opens the conversation`

### refresh-token-replay.spec.ts (1 test)

- **Refresh token replay** @security @auth
  - `reusing the same refresh token concurrently succeeds once and rejects the replay`

### safety-numbers.spec.ts (1 test)

- **Safety numbers modal**
  - `safety numbers modal opens and displays fingerprint content`

### screen-time.spec.ts (3 tests)

- **Screen Time -- parent dashboard** @smoke
  - `displays daily usage summaries for a child`
  - `renders screen time section when navigating to child details`
- **Screen Time -- report ingestion stub**
  - `POST /screentime/reports accepts a report payload`

### security-input-surfaces.spec.ts (5 tests)

- **XSS neutralization -- explore search result usernames** @security @xss @smoke
  - `XSS in search result display names is escaped, not executed`
- **XSS neutralization -- DM peer display names** @security @xss @smoke
  - `XSS in DM peer display names is escaped, not executed`
- **XSS neutralization -- server names in sidebar** @security @xss @smoke
  - `XSS in server names is escaped, not executed`
- **XSS neutralization -- canvas poll question** @security @xss @smoke
  - `XSS in canvas poll question text is escaped, not executed`
- **XSS neutralization -- support ticket descriptions** @security @xss @smoke
  - `XSS in support ticket description text is escaped, not executed`

### server-settings.spec.ts (2 tests)

- **Server settings**
  - `server settings page renders name and description fields`
  - `custom emoji management section renders upload form` @smoke

### servers.spec.ts (5 tests)

- **Servers - authenticated**
  - `opens the create-server flow from the app shell`
- **Channel - authenticated**
  - `first server channel page renders message input`
  - `sending a channel message renders it in the list`
  - `typing indicator appears when typing in channel`
- **Invite links - authenticated**
  - `invite link can be generated for a server`

### session-expiry.spec.ts (2 tests)

- **Session expiry** @security @auth
  - `401 during profile save shows error and remains stable` @smoke
  - `401 on device list fetch shows error state gracefully` @smoke

### settings-interactions.spec.ts (12 tests)

- **Settings -- My Profile**
  - `profile section renders display name and username fields` @smoke
  - `saving a profile change triggers a success notification`
- **Settings -- Appearance**
  - `Appearance section renders theme options` @smoke
  - `toggling the theme changes the data-theme attribute`
- **Settings -- Change Password**
  - `Change Password section renders current/new/confirm fields` @smoke
- **Settings -- Yapper Premium**
  - `Premium section renders heading and promo code input` @smoke
  - `entering a valid promo code shows a success message`
  - `entering an invalid promo code shows an error message`
- **Settings -- Support**
  - `Support section renders the ticket creation form` @smoke
  - `submitting a support ticket adds it to the history list`
- **Settings -- Device management**
  - `devices sidebar shows the registered devices list` @smoke
  - `revoking a non-current device removes it from the list`

### social.spec.ts (8 tests)

- **Social -- follow graph**
  - `following User B increments their follower count and sets isFollowing=true`
  - `unfollowing User B decrements their follower count and sets isFollowing=false`
  - `follow is idempotent -- following twice does not double-increment the count`
  - `User B's profile page renders after User A follows them`
- **Social -- friend requests**
  - `User A sends a friend request and it appears in User B incoming list`
  - `User B accepting the request establishes a friendship (isFriend=true for both)`
  - `User B rejecting the request leaves isFriend=false`
  - `duplicate friend request is idempotent (409 is handled gracefully)`

### tauri-deep-links.spec.ts (1 test)

- **Tauri deep links** @desktop @regression
  - `yapper://invite/:code opens the join server modal`

### tauri-keyboard-shortcuts.spec.ts (3 tests)

- **Tauri keyboard shortcuts** @desktop @smoke
  - `Ctrl+K opens command palette or search`
  - `Ctrl+, opens settings`
  - `Escape closes open modal/panel`

### tauri-native-notifications.spec.ts (2 tests)

- **Tauri native notifications** @desktop @regression
  - `app becomes ready without crashing when notifications are granted`
  - `app becomes ready without crashing when notifications are denied`

### tauri-vault-gate.spec.ts (2 tests)

- **Tauri vault gate** @desktop @smoke
  - `Stronghold passphrase gate appears on first launch`
  - `empty passphrase shows validation error`

### toast-notifications.spec.ts (3 tests)

- **Toast notifications -- success**
  - `export data action shows a success toast` @smoke
- **Toast notifications -- error**
  - `failed action shows an error toast or error state`
- **Toast notifications -- appearance**
  - `settings page has a working toast container element` @smoke

### typing-indicators.spec.ts (2 tests)

- **Typing indicators -- channel**
  - `User A typing causes a typing indicator to appear for User B`
- **Typing indicators -- DM**
  - `User A typing in DM triggers typing indicator for User B`

### uat/uat.spec.ts (30 tests)

- **UAT-01 -- Health & Infrastructure**
  - `UAT-01-A  API /health returns ok with db:true`
  - `UAT-01-B  Frontend loads within 5 seconds`
  - `UAT-01-C  Unauthenticated visit to /servers redirects to /login`
  - `UAT-01-D  Health endpoint does not require authentication`
- **UAT-17 -- Security Headers & Hardening**
  - `UAT-17-A  API includes X-Content-Type-Options: nosniff`
  - `UAT-17-B  API includes X-Frame-Options DENY or SAMEORIGIN`
  - `UAT-17-C  API includes Strict-Transport-Security on HTTPS` (skipped on localhost)
  - `UAT-17-D  CORS rejects unlisted origin`
  - `UAT-17-G  Mutating request without X-CSRF-Token returns 403`
  - `UAT-17-F  Login response sets HttpOnly Secure cookie`
- **UAT-06 -- E2EE -- Client-Observable Behaviour**
  - `UAT-06-A  IndexedDB yapper-signal store exists after login`
  - `UAT-06-D  Key bundle API returns public keys only, no private keys`
  - `UAT-06-E  WebSocket URL does not contain token in query string`
- **UAT-14 -- Premium & GoPro**
  - `UAT-14-A  Free user premium status returns is_premium:false`
  - `UAT-14-B  Invalid promo code returns 400 or 404`
- **UAT-16 -- Push Notifications**
  - `UAT-16-A  Register FCM device token returns 200/201`
  - `UAT-16-B  Unregister FCM device token returns 200`
- **UAT-15 -- Support Tickets -- Validation**
  - `UAT-15-A  Valid ticket creation returns 201`
  - `UAT-15-B  GET /support/tickets returns ticket list`
  - `UAT-15-C  Invalid ticket type returns 400/422`
  - `UAT-15-D  Subject over 200 characters returns 400/422`
- **UAT-12 -- Account Lifecycle**
  - `UAT-12-A  PATCH /users/me updates display_name`
  - `UAT-12-B  Username change cooldown -- second change returns 409`
  - `UAT-12-E  Data export returns a ZIP file`
  - `UAT-12-L  Change password with wrong current returns 400/401`
- **UAT-11 -- Custom Emoji Limits**
  - `UAT-11-C  Non-admin emoji upload returns 403`
- **UAT-07 -- Media Upload Validation**
  - `UAT-07-A  Upload URL endpoint returns a presigned URL`
  - `UAT-07-B  Presigned URL hostname matches R2 (not API)`
  - `UAT-07-D  Upload size exceeding limit returns 400/413`
- **UAT-04 -- Server API Validation**
  - `UAT-04-E  Create server with no name returns 400/422`
  - `UAT-04-F  Join with invalid invite code returns 404`
- **UAT-10 -- Profiles & Social API**
  - `UAT-10-A  GET /users/me returns user with id and username`
  - `UAT-10-B  GET /users/by/:username does not expose private fields`
- **UAT-18 -- Device Trust Flow**
  - `UAT-18-A  GET /devices returns current device as trusted`
- **UAT-08 -- Live Canvas API**
  - `UAT-08-A  Canvas state returns music, polls, clips, event keys`
  - `UAT-08-B  Creating a poll returns 201`
- **UAT-09 -- Explore API**
  - `UAT-09-A  Trending tags returns an array`
  - `UAT-09-B  Server search returns without error`
  - `UAT-09-C  Live servers returns an array`

### vault-failure.spec.ts (1 test)

- **Vault failure modes** @desktop-native @encryption @edge-case
  - `OS-level write revocation surfaces Secure Storage Unavailable UI` @smoke

### ws-disruption.spec.ts (1 test)

- **WebSocket MITM disruption** @e2ee @websocket @deep-analytical
  - `forceful WS termination mid-session triggers reconnecting banner`

### ws-reconnection.spec.ts (2 tests)

- **WebSocket reconnection banner**
  - `reconnecting banner appears when WebSocket connection is lost` @smoke
  - `app shell renders normally on initial load` @smoke

### xss-injection.spec.ts (1 test)

- **XSS payload neutralization** @security @xss @gui-execution
  - `obfuscated script payloads in community names render as escaped text` @smoke

### accessibility.spec.ts (9 tests)

- **Accessibility -- unauthenticated pages** @accessibility
  - `/login page meets WCAG 2.1 AA`
  - `/register page meets WCAG 2.1 AA`
  - `/forgot-password page meets WCAG 2.1 AA`
- **Accessibility -- authenticated pages** @accessibility
  - `/explore page meets WCAG 2.1 AA`
  - `/dm page meets WCAG 2.1 AA`
  - `/settings page meets WCAG 2.1 AA`
  - `/settings -- each section panel meets WCAG 2.1 AA`
  - `/parent/children/setup meets WCAG 2.1 AA`
  - `/profile/:username page meets WCAG 2.1 AA`

---

## Test Infrastructure

### Playwright Configuration

| Setting | Value |
|---------|-------|
| Config | `frontend/playwright.config.ts` |
| Workers | 1 (serial) |
| Retries | 0 local, 2 in CI |
| Auth | Global setup via `tests/global-setup.ts` |
| Projects | `chromium`, `mobile-chrome`, `tauri-desktop` (when `TAURI_BINARY` set) |
| Credentials | `.env.test` (gitignored) |

### CI Pipelines

| Pipeline | File | Trigger |
|----------|------|---------|
| E2E Smoke (PR) | `.github/workflows/e2e-pr-smoke.yml` | PRs touching frontend/backend |
| E2E Nightly | `.github/workflows/e2e-nightly.yml` | Daily 02:00 SAST + manual dispatch |
| Security Scans | `.github/workflows/security-scans.yml` | Push to main, PRs, daily |
| CI (unit + build) | `.github/workflows/ci.yml` | Push to main, PRs |

### Tag Glossary

| Tag | Meaning |
|-----|---------|
| `@smoke` | Core happy-path; runs in PR smoke suite |
| `@security` | Security-focused (XSS, CSRF, auth) |
| `@auth` | Authentication/authorization flow |
| `@xss` | XSS injection prevention |
| `@accessibility` | WCAG 2.1 AA compliance |
| `@desktop` | Tauri desktop-only |
| `@multidevice` | Multi-device trust flow |
| `@regression` | Regression guard for fixed bugs |
| `@e2ee` | End-to-end encryption |
| `@websocket` | WebSocket behavior |

---

## Latest Run Results

**Date:** 2026-05-27

### Backend (`cargo test --lib --bins`)

| Metric | Count |
|--------|-------|
| Backend unit tests | 217 passed |

Run on every push/PR via the `Backend (Rust)` job in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml).

### Frontend E2E

PR-time signal comes from [`.github/workflows/e2e-pr-smoke.yml`](../.github/workflows/e2e-pr-smoke.yml) which runs the `@smoke`-tagged subset against a CI-bound stack and is **green on every recent PR**. This is the trustworthy E2E gate today.

The full suite ([`.github/workflows/e2e-nightly.yml`](../.github/workflows/e2e-nightly.yml)) is **not currently producing signal**: it targets `staging.yapperhq.com` / `staging-api.yapperhq.com`, but staging was never properly provisioned on Fly. The probe-auth job correctly detects this as `edge-blocked` and gracefully skips the Playwright shards, so every scheduled nightly since at least 2026-05-22 has exited green without running tests. See [`docs/deployment.md` § Staging environment](../docs/deployment.md#staging-environment) for the diagnosis and the provisioning runbook.

The previous "70 UI selector timeouts" line that lived here was a **localhost-only** artifact from 2026-03-27 (likely a missing backend / fixture issue on that environment) and was not representative of CI. It has been removed to avoid misleading future readers; once staging is provisioned and the nightly produces real numbers, replace this section with the new baseline.

### Skip patterns

25 of 49 spec files contain `test.skip` calls. All are healthy:

- **Conditional credential skips** (~20 files) — `test.skip(!process.env.E2E_EMAIL, '...')`. Skip locally without credentials; run in CI where secrets exist.
- **Conditional fixture skips** — multi-device, safety-numbers, live-canvas, etc. Skip when a precondition (second auth state, button visible, channel type) is unmet.
- **Permanent "not yet implemented" markers** (4 tests in 2 files):
  - `discord-import.spec.ts:11,17,25` — Discord import BE pending (matches HANDOVER S12 status)
  - `tauri-deep-links.spec.ts:16` — feature pending

No `.fixme`, no `.failing` — the suite itself is clean.
