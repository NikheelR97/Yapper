//! Centralised constants for server-enforced limits (HANDOVER.md Section 3, Rule 3).
//!
//! Every collection or client-controlled allocation must be bounded by a constant
//! defined here. Module-local constants are acceptable only when they are
//! purely implementation details (e.g. internal buffer sizes).

/// Maximum WebSocket frame size in bytes (64 KB).
///
/// Enforced at the protocol layer on both inbound and outbound frames.
/// Frames exceeding this limit are rejected before deserialization.
pub const MAX_WS_FRAME_SIZE: usize = 64 * 1024;

/// Maximum wire size of a single message's ciphertext payload in bytes.
///
/// This limit applies to the raw BYTEA stored in the `messages.ciphertext`
/// column and to HTTP/WS message payloads before the protocol-level frame cap
/// takes over (`MAX_WS_FRAME_SIZE` = 64 KB).
///
/// AES-256-GCM adds 28 bytes of overhead per payload: a 12-byte IV and a
/// 16-byte authentication tag. The client enforces `MAX_MESSAGE_CHARS` before
/// encryption; the server enforces this ciphertext-sized bound after
/// encryption.
pub const MAX_MESSAGE_LENGTH: usize = 8_000;

/// Maximum HTTP request body size for message submission routes.
/// This comfortably fits the largest supported multi-device DM payload while
/// rejecting oversized JSON before deserialization.
pub const MAX_MESSAGE_REQUEST_BODY_SIZE: usize = 256 * 1024;

/// Maximum length of a plaintext message in Unicode characters.
///
/// This is enforced client-side before encryption. The server enforces
/// `MAX_MESSAGE_LENGTH` on the resulting ciphertext payload.
///
/// Applies to both DM composition and channel composition before the message is
/// encrypted for transport.
pub const MAX_MESSAGE_CHARS: usize = 4_000;

/// Maximum upload size for free-tier users (10 MB).
pub const MAX_UPLOAD_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum upload size for premium users (50 MB).
pub const MAX_UPLOAD_SIZE_PREMIUM: u64 = 50 * 1024 * 1024;

/// Maximum one-time prekeys per upload batch.
pub const MAX_OPK_BATCH: usize = 100;

/// Maximum custom emoji file size before processing (256 KB).
pub const MAX_EMOJI_SIZE: usize = 256 * 1024;

/// Maximum concurrent WebSocket connections per user.
pub const MAX_CONNECTIONS_PER_USER: usize = 5;

/// Maximum server members at MVP (hard cap on join).
pub const MAX_SERVER_MEMBERS_MVP: i64 = 500;

/// Maximum password length in bytes (prevents Argon2id DoS).
pub const MAX_PASSWORD_LENGTH: usize = 1_024;

/// Maximum inbound WebSocket messages processed per select! iteration
/// before yielding back to the executor. Prevents a single fast sender
/// from starving other connections on the same runtime thread.
pub const MAX_MESSAGES_PER_TICK: usize = 32;

/// Maximum media uploads per user per minute.
pub const MAX_UPLOADS_PER_MINUTE: u32 = 10;

/// Maximum single image dimension (width or height) accepted for upload.
/// Prevents decompression bombs that expand a tiny file into gigabytes of RAM.
pub const MAX_IMAGE_DIMENSION: u32 = 4096;

/// Maximum decoded pixel count (width × height) for uploaded images.
pub const MAX_DECODED_PIXELS: u32 = 4096 * 4096;

/// Maximum device IDs that can be requested in a single key-bundle query.
pub const MAX_DEVICE_IDS: usize = 100;

/// Maximum push tokens a single user may register across all devices.
pub const MAX_PUSH_TOKENS_PER_USER: i64 = 10;

/// Days after which a media upload row is eligible for GC.
/// R2 objects are cleaned independently via lifecycle rules.
pub const MEDIA_UPLOAD_EXPIRY_DAYS: i32 = 7;
