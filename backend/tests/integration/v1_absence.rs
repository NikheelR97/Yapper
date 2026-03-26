//! Integration tests for removal of the legacy `/api/v1` surface.

use super::spawn_test_server;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn legacy_v1_endpoints_are_not_mounted() {
    let Some(server) = spawn_test_server().await else {
        return;
    };

    let cases = [
        ("GET", "/api/v1/users/me"),
        ("GET", "/api/v1/servers"),
        ("POST", "/api/v1/auth/login"),
        ("POST", "/api/v1/keys/identity"),
    ];

    for (method, path) in cases {
        let response = match method {
            "GET" => server.get(path).await,
            "POST" => server.post(path).json(&serde_json::json!({})).await,
            _ => unreachable!("unsupported method"),
        };

        assert_eq!(
            response.status_code().as_u16(),
            404,
            "legacy route should be unmounted: {method} {path} -> {}",
            response.status_code()
        );
    }
}
