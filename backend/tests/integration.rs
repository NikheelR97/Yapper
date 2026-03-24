//! Integration test suite for the Yapper backend.
//!
//! Run with:
//!   TEST_DATABASE_URL=postgres://yapper:yapper_dev@localhost:5432/yapper_test \
//!     cargo test --test integration -- --test-threads=1
//!
//! Tests must run single-threaded (`--test-threads=1`) because they share a
//! single database. The `serial_test::serial` attribute enforces ordering within
//! a test binary, but `--test-threads=1` is still required when running the full
//! suite to prevent interference between test files.

#[path = "integration/mod.rs"]
mod helpers;

// Re-export test modules so they are discovered by the test harness.
#[allow(unused_imports)]
use helpers::{account, auth, devices, keys};
