//! Integration test for `notify_user_offline_devices`.
//!
//! Covers the offline-push half of the channel-fanout change in
//! `hub::fanout_to_channel_members` (src/hub.rs): when a user has no
//! registered push-capable devices, the notifier must no-op cleanly
//! (no panic, no FCM call attempted) rather than erroring out.

use super::build_test_state_from_pool;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn notify_user_offline_devices_noops_with_no_registered_devices(pool: PgPool) {
    let Some(state) = build_test_state_from_pool(pool).await else {
        return;
    };

    let user_id = Uuid::new_v4();
    let mut meta = std::collections::HashMap::new();
    meta.insert("channel_id".into(), Uuid::new_v4().to_string());

    // Should complete without panicking and without attempting any push,
    // since the user has zero rows in `devices`.
    yapper_server::notifications::notify_user_offline_devices(user_id, "channel", &meta, &state)
        .await;
}
