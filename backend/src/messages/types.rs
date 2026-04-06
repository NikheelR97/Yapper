use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateConversationReq {
    pub peer_id: Uuid,
}

#[derive(Serialize)]
pub struct ConversationResp {
    pub id: Uuid,
    pub peer_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ConversationListItem {
    pub id: Uuid,
    pub peer_id: Uuid,
    pub peer_username: String,
    pub peer_display_name: Option<String>,
    pub peer_avatar_url: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListMessagesQuery {
    /// Fetch messages created before this cursor (UUID of last known message).
    pub before: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct MessageRespV2 {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub sender_device_id: Uuid,
    pub sender_signal_device_id: i32,
    pub ciphertext: String,
    pub ephemeral_key: Option<String>,
    pub opk_id: Option<i32>,
    pub msg_num: i32,
    pub ratchet_pub: Option<String>,
    pub previous_chain_len: Option<i32>,
    pub crypto_version: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendEnvelopeReqV2 {
    pub recipient_user_id: Uuid,
    pub recipient_device_id: Uuid,
    pub ciphertext: String,
    pub ephemeral_key: Option<String>,
    pub opk_id: Option<i32>,
    pub msg_num: i32,
    pub ratchet_pub: Option<String>,
    pub previous_chain_len: Option<i32>,
    pub crypto_version: Option<i16>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendMessageReqV2 {
    pub envelopes: Vec<SendEnvelopeReqV2>,
}
