use mockito::{Server, ServerGuard};
use starforge::utils::{config, horizon};
use std::sync::Once;
use tempfile::TempDir;

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });
}

/// Helper to create a temporary config with custom horizon endpoints
async fn setup_test_config_with_endpoints(
    endpoints: Vec<String>,
    health_timeout_secs: u64,
) -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let endpoints_str = endpoints
        .iter()
        .map(|e| format!("\"{}\"", e))
        .collect::<Vec<_>>()
        .join(", ");

    let config_content = format!(
        r#"
[networks.testnet]
horizon_url = "{}"
horizon_endpoints = [{}]
health_timeout_secs = {}
soroban_rpc_url = "https://soroban-testnet.stellar.org"
passphrase = "Test SDF Network ; September 2015"
"#,
        endpoints[0], endpoints_str, health_timeout_secs
    );

    std::fs::write(&config_path, config_content).unwrap();
    std::env::set_var("STARFORGE_CONFIG", config_path.to_str().unwrap());

    (temp_dir, "testnet".to_string())
}

#[tokio::test]
async fn test_failover_on_unhealthy_primary() {
    init();

    // Start two mock servers
    let mut server1 = Server::new_async().await;
    let mut server2 = Server::new_async().await;

    // Primary server returns 503 (unhealthy)
    let mock1 = server1
        .mock("GET", "/accounts/GABC123")
        .with_status(503)
        .with_body(r#"{"error":"service unavailable"}"#)
        .create_async()
        .await;

    // Secondary server returns 200 (healthy)
    let mock2 = server2
        .mock("GET", "/accounts/GABC123")
        .with_status(200)
        .with_body(
            r#"{
            "id": "GABC123",
            "sequence": "12345",
            "balances": [{"balance": "1000", "asset_type": "native"}],
            "subentry_count": 0
        }"#,
        )
        .create_async()
        .await;

    // Configure with both endpoints
    let (_temp_dir, network) = setup_test_config_with_endpoints(
        vec![server1.url(), server2.url()],
        5,
    )
    .await;

    // Make a request that should failover to server2
    let result = horizon::fetch_account("GABC123", &network).await;

    // Should succeed using the second endpoint
    assert!(result.is_ok(), "Failover should succeed");
    let account = result.unwrap();
    assert_eq!(account.id, "GABC123");

    // Verify both endpoints were called
    mock1.assert_async().await;
    mock2.assert_async().await;
}

#[tokio::test]
async fn test_failover_on_connection_error() {
    init();

    let mut server2 = Server::new_async().await;

    // Second server is healthy
    let mock2 = server2
        .mock("GET", "/accounts/GABC123")
        .with_status(200)
        .with_body(
            r#"{
            "id": "GABC123",
            "sequence": "12345",
            "balances": [{"balance": "1000", "asset_type": "native"}],
            "subentry_count": 0
        }"#,
        )
        .create_async()
        .await;

    // Configure with unreachable primary and healthy secondary
    let (_temp_dir, network) = setup_test_config_with_endpoints(
        vec!["http://127.0.0.1:1".to_string(), server2.url()],
        2,
    )
    .await;

    // Make a request that should failover to server2
    let result = horizon::fetch_account("GABC123", &network).await;

    // Should succeed using the second endpoint
    assert!(result.is_ok(), "Should failover on connection error");
    mock2.assert_async().await;
}

#[tokio::test]
async fn test_no_failover_on_client_error() {
    init();

    let mut server1 = Server::new_async().await;
    let mut server2 = Server::new_async().await;

    // Primary server returns 404 (client error)
    let mock1 = server1
        .mock("GET", "/accounts/GABC123")
        .with_status(404)
        .with_body(r#"{"error":"account not found"}"#)
        .create_async()
        .await;

    // Secondary server should NOT be called
    let mock2 = server2
        .mock("GET", "/accounts/GABC123")
        .with_status(200)
        .expect(0) // Should not be called
        .create_async()
        .await;

    let (_temp_dir, network) = setup_test_config_with_endpoints(
        vec![server1.url(), server2.url()],
        5,
    )
    .await;

    // Make a request that should NOT failover
    let result = horizon::fetch_account("GABC123", &network).await;

    // Should fail with account not found error
    assert!(result.is_err(), "Client errors should not trigger failover");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found"));

    mock1.assert_async().await;
    mock2.assert_async().await; // Asserts it was NOT called
}

#[tokio::test]
async fn test_all_endpoints_fail() {
    init();

    let mut server1 = Server::new_async().await;
    let mut server2 = Server::new_async().await;

    // Both servers are unhealthy
    let mock1 = server1
        .mock("GET", "/accounts/GABC123")
        .with_status(503)
        .create_async()
        .await;

    let mock2 = server2
        .mock("GET", "/accounts/GABC123")
        .with_status(503)
        .create_async()
        .await;

    let (_temp_dir, network) = setup_test_config_with_endpoints(
        vec![server1.url(), server2.url()],
        5,
    )
    .await;

    let result = horizon::fetch_account("GABC123", &network).await;

    // Should fail after trying all endpoints
    assert!(result.is_err(), "Should fail when all endpoints are down");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("All configured Horizon endpoints"));

    mock1.assert_async().await;
    mock2.assert_async().await;
}

#[tokio::test]
async fn test_health_probe_timeout() {
    init();

    // Test with extremely short timeout to force timeout behavior
    let (_temp_dir, network) = setup_test_config_with_endpoints(
        vec!["http://127.0.0.1:1".to_string()],
        1, // 1 second timeout
    )
    .await;

    let result = horizon::select_healthy_endpoint(&network).await;

    assert!(
        result.is_err(),
        "Should fail when all endpoints timeout"
    );
    assert!(result.unwrap_err().to_string().contains("unhealthy"));
}

#[tokio::test]
async fn test_endpoint_context_exposed() {
    init();

    let mut server1 = Server::new_async().await;
    let mut server2 = Server::new_async().await;

    // Primary fails, secondary succeeds
    let _mock1 = server1
        .mock("GET", "/accounts/GABC123")
        .with_status(503)
        .create_async()
        .await;

    let _mock2 = server2
        .mock("GET", "/accounts/GABC123")
        .with_status(200)
        .with_body(
            r#"{
            "id": "GABC123",
            "sequence": "12345",
            "balances": [{"balance": "1000", "asset_type": "native"}],
            "subentry_count": 0
        }"#,
        )
        .create_async()
        .await;

    let (_temp_dir, network) = setup_test_config_with_endpoints(
        vec![server1.url(), server2.url()],
        5,
    )
    .await;

    // Use send_with_failover directly to get context
    let result = horizon::send_with_failover(&network, |endpoint| {
        let url = format!("{}/accounts/GABC123", endpoint);
        horizon::http_client().get(&url).send()
    })
    .await;

    assert!(result.is_ok(), "Failover should succeed");
    let (_response, context) = result.unwrap();

    // Verify context contains correct information
    assert_eq!(context.endpoint_index, 1, "Should use second endpoint");
    assert_eq!(
        context.failed_endpoints.len(),
        1,
        "Should record one failed endpoint"
    );
    assert!(
        context.endpoint_url.contains(&server2.url()),
        "Should report successful endpoint"
    );
}
