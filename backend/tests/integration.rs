//! Integration test suite for the Yapper backend.
//!
//! Run with:
//!   TEST_DATABASE_URL=postgres://yapper:yapper_dev@localhost:5432/yapper_test \
//!     cargo test --test integration
//!
//! Each test provisions an isolated PostgreSQL schema, so integration cases can
//! run under the default thread count without sharing table state.

#[path = "integration/mod.rs"]
mod helpers;

// Re-export test modules so they are discovered by the test harness.
#[allow(unused_imports)]
use helpers::{
    account, auth, canvas, devices, identity, keys, messages, parental, screentime, server_caps,
    v1_absence,
};
