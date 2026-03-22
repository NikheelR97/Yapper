use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ─── Constants ───────────────────────────────────────────────────────────────

/// Maximum tracks in the music queue per server.
pub const MAX_MUSIC_QUEUE_SIZE: i64 = 50;

/// Maximum tracks retained in music history per server.
pub const MAX_MUSIC_HISTORY_SIZE: i64 = 20;

/// Maximum length for music artist/title fields (chars).
pub const MAX_MUSIC_FIELD_LEN: usize = 200;

/// Maximum URL length for source/album art.
pub const MAX_URL_LEN: usize = 2048;

/// Maximum track duration in seconds (24 hours).
pub const MAX_TRACK_DURATION_SECS: i32 = 86400;

/// Default skip vote threshold (percentage of online members).
pub const DEFAULT_SKIP_THRESHOLD_PCT: i16 = 50;

/// Maximum DJ users per server.
pub const MAX_DJ_USERS_PER_SERVER: i64 = 20;

/// Maximum active polls per channel.
pub const MAX_ACTIVE_POLLS_PER_CHANNEL: i64 = 5;

/// Maximum poll options for multiple_choice and emoji_reaction types.
pub const MAX_POLL_OPTIONS: usize = 6;

/// Maximum poll question length (chars).
pub const MAX_POLL_QUESTION_LEN: usize = 500;

/// Maximum poll option text length (chars).
pub const MAX_POLL_OPTION_LEN: usize = 200;

/// Maximum poll duration (24 hours in seconds).
pub const MAX_POLL_DURATION_SECS: i64 = 86400;

/// Maximum total reactions per clip (across all emoji + users).
pub const MAX_REACTIONS_PER_CLIP: i64 = 500;

/// Maximum pinned clips per server.
pub const MAX_PINNED_CLIPS: i64 = 3;

/// Maximum future time for canvas events (7 days).
pub const MAX_EVENT_FUTURE_SECS: i64 = 7 * 24 * 3600;

/// Maximum event title length (chars).
pub const MAX_EVENT_TITLE_LEN: usize = 200;

/// Maximum event description length (chars).
pub const MAX_EVENT_DESCRIPTION_LEN: usize = 1000;

/// How long after event_at a "Live Now" event remains active (24h).
pub const EVENT_LIVE_WINDOW_SECS: i64 = 24 * 3600;

/// Clip expiry period in days, matching R2 lifecycle.
pub const CLIP_EXPIRY_DAYS: i64 = 30;

// ─── Music DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct MusicInput {
    #[validate(length(min = 1, max = 200))]
    pub artist: String,
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(max = 2048))]
    pub source_url: Option<String>,
    #[validate(length(max = 2048))]
    pub album_art_url: Option<String>,
    #[validate(range(min = 1, max = 86400))]
    pub duration_secs: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct EnqueueTrackReq {
    #[validate(length(min = 1, max = 200))]
    pub artist: String,
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(range(min = 1, max = 86400))]
    pub duration_secs: i32,
    #[validate(length(max = 2048))]
    pub source_url: Option<String>,
    #[validate(length(max = 2048))]
    pub album_art_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ReorderQueueReq {
    #[validate(length(min = 1, max = 50))]
    pub track_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvanceTrackReq {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateMusicSettingsReq {
    #[validate(range(min = 10, max = 100))]
    pub skip_threshold_pct: Option<i16>,
    pub queue_enabled: Option<bool>,
}

// ─── Music response types ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct NowPlayingResp {
    pub track_id: Option<Uuid>,
    pub artist: String,
    pub title: String,
    pub duration_secs: Option<i32>,
    pub source_url: Option<String>,
    pub album_art_url: Option<String>,
    pub started_at: DateTime<Utc>,
    pub set_by: Uuid,
}

#[derive(Debug, Serialize)]
pub struct QueueEntryResp {
    pub id: Uuid,
    pub artist: String,
    pub title: String,
    pub duration_secs: i32,
    pub source_url: Option<String>,
    pub album_art_url: Option<String>,
    pub added_by: Uuid,
    pub position: i32,
}

#[derive(Debug, Serialize)]
pub struct HistoryTrackResp {
    pub id: Uuid,
    pub artist: String,
    pub title: String,
    pub duration_secs: i32,
    pub source_url: Option<String>,
    pub album_art_url: Option<String>,
    pub added_by: Uuid,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

// ─── Poll DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreatePollReq {
    #[validate(length(min = 1, max = 20))]
    #[serde(default = "default_poll_type")]
    pub poll_type: String,
    #[validate(length(min = 1, max = 500))]
    pub question: String,
    pub options: Option<Vec<String>>,
    pub ends_at: Option<String>,
    #[serde(default)]
    pub anonymous: bool,
}

fn default_poll_type() -> String {
    "multiple_choice".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoteReq {
    pub option_index: i32,
}

// ─── Clip DTOs ───────────────────────────────────────────────────────────────

// E2EE BOUNDARY NOTE: Reactions are plaintext metadata. The emoji and user_id
// are stored unencrypted. The Clip *content* (ciphertext in the messages table)
// remains encrypted. Reactions are social signals about content, not content
// itself.

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AddReactionReq {
    #[validate(length(min = 1, max = 8))]
    pub emoji: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RemoveReactionQuery {
    #[validate(length(min = 1, max = 8))]
    pub emoji: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinClipReq {
    pub clip_id: Uuid,
}

// ─── Event DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CanvasEventResp {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub event_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateEventReq {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    /// ISO-8601 UTC timestamp. Must be in the future, at most 7 days out.
    pub event_at: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateEventReq {
    #[validate(length(min = 1, max = 200))]
    pub title: Option<String>,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub event_at: Option<String>,
}

// ─── Hydration ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasStateQuery {
    pub channel_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── MusicInput deserialization ────────────────────────────────────────

    #[test]
    fn music_input_deserializes_with_all_fields() {
        let json = serde_json::json!({
            "artist": "Kendrick Lamar",
            "title": "HUMBLE.",
            "source_url": "https://example.com/song.mp3",
            "album_art_url": "https://example.com/art.jpg",
            "duration_secs": 180
        });
        let input: MusicInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.artist, "Kendrick Lamar");
        assert_eq!(input.title, "HUMBLE.");
        assert_eq!(input.duration_secs, Some(180));
    }

    #[test]
    fn music_input_deserializes_with_required_fields_only() {
        let json = serde_json::json!({
            "artist": "Drake",
            "title": "God's Plan"
        });
        let input: MusicInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.artist, "Drake");
        assert!(input.source_url.is_none());
        assert!(input.duration_secs.is_none());
    }

    #[test]
    fn music_input_rejects_unknown_fields() {
        let json = serde_json::json!({
            "artist": "Drake",
            "title": "God's Plan",
            "extra_field": "should fail"
        });
        assert!(serde_json::from_value::<MusicInput>(json).is_err());
    }

    // ── EnqueueTrackReq ──────────────────────────────────────────────────

    #[test]
    fn enqueue_track_deserializes_valid() {
        let json = serde_json::json!({
            "artist": "SZA",
            "title": "Kill Bill",
            "duration_secs": 240
        });
        let req: EnqueueTrackReq = serde_json::from_value(json).unwrap();
        assert_eq!(req.artist, "SZA");
        assert_eq!(req.duration_secs, 240);
    }

    #[test]
    fn enqueue_track_rejects_missing_duration() {
        let json = serde_json::json!({
            "artist": "SZA",
            "title": "Kill Bill"
        });
        assert!(serde_json::from_value::<EnqueueTrackReq>(json).is_err());
    }

    // ── CreatePollReq ────────────────────────────────────────────────────

    #[test]
    fn create_poll_defaults_to_multiple_choice() {
        let json = serde_json::json!({
            "question": "Best language?",
            "options": ["Rust", "Go"]
        });
        let req: CreatePollReq = serde_json::from_value(json).unwrap();
        assert_eq!(req.poll_type, "multiple_choice");
        assert!(!req.anonymous);
    }

    #[test]
    fn create_poll_binary_without_options() {
        let json = serde_json::json!({
            "poll_type": "binary",
            "question": "Should we do it?"
        });
        let req: CreatePollReq = serde_json::from_value(json).unwrap();
        assert_eq!(req.poll_type, "binary");
        assert!(req.options.is_none());
    }

    // ── VoteReq ──────────────────────────────────────────────────────────

    #[test]
    fn vote_req_rejects_unknown_fields() {
        let json = serde_json::json!({ "option_index": 0, "extra": true });
        assert!(serde_json::from_value::<VoteReq>(json).is_err());
    }

    // ── AddReactionReq ───────────────────────────────────────────────────

    #[test]
    fn add_reaction_deserializes() {
        let json = serde_json::json!({ "emoji": "👍" });
        let req: AddReactionReq = serde_json::from_value(json).unwrap();
        assert_eq!(req.emoji, "👍");
    }

    // ── CreateEventReq ───────────────────────────────────────────────────

    #[test]
    fn create_event_deserializes() {
        let json = serde_json::json!({
            "title": "Game Night",
            "description": "Mario Kart!",
            "event_at": "2026-03-22T20:00:00Z"
        });
        let req: CreateEventReq = serde_json::from_value(json).unwrap();
        assert_eq!(req.title, "Game Night");
    }

    // ── Constants ────────────────────────────────────────────────────────

    #[test]
    fn constants_are_sensible() {
        assert_eq!(MAX_MUSIC_FIELD_LEN, 200);
        assert_eq!(MAX_URL_LEN, 2048);
        assert!(MAX_URL_LEN >= MAX_MUSIC_FIELD_LEN);
        assert!(MAX_POLL_OPTIONS <= 10);
        assert!(MAX_PINNED_CLIPS <= 10);
        assert!(MAX_EVENT_FUTURE_SECS == 604800);
    }
}
