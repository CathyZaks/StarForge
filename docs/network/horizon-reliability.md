# Horizon Client Reliability and Retry Policy

## 1. Overview
StarForge uses bounded timeouts, exponential backoff retry policies, and automatic endpoint failover for Horizon HTTP requests to guard against transient network partitions, rate limits, and provider outages.

## 2. Reliability Specifications
- **Bounded Timeout**: 10-second default request timeout on the shared HTTP client.
- **Retry Count**: Maximum of 3 attempts with exponential backoff (`150ms -> 300ms -> 600ms`).
- **Retry Classification**: Retries on transient 5xx server errors, HTTP 429 Too Many Requests, and low-level connection resets.
- **Permanent Errors**: 4xx client errors (e.g. 404 Account Not Found, 400 Bad Request) fail fast without retrying.

## 3. Endpoint Failover Configuration
StarForge supports configuring multiple Horizon endpoints per network with automatic health-based failover.

### Configuration Schema
Add `horizon_endpoints` to your network configuration to enable failover:

```toml
[networks.testnet]
horizon_url = "https://horizon-testnet.stellar.org"  # Primary endpoint (fallback)
horizon_endpoints = [
    "https://horizon-testnet.stellar.org",
    "https://horizon-testnet-backup.example.com",
    "https://horizon-testnet-tertiary.example.com"
]
health_timeout_secs = 5  # Health probe timeout (default: 5 seconds)
soroban_rpc_url = "https://soroban-testnet.stellar.org"
passphrase = "Test SDF Network ; September 2015"
```

### Behavior
- **Ordered Failover**: Endpoints are tried in the order specified in `horizon_endpoints`.
- **Health Probes**: Each endpoint is checked for HTTP 200 response within `health_timeout_secs`.
- **Automatic Retry**: On 5xx errors or connection failures, the next endpoint is tried automatically.
- **Client Error Fast-Fail**: 4xx errors (invalid requests) are returned immediately without trying other endpoints.
- **Endpoint Context**: In verbose/JSON modes, the response includes which endpoint served the request and any failed endpoints.

### Usage
No code changes required. Once configured, all Horizon requests automatically use failover:

```bash
# Configure a network with multiple endpoints
starforge network add mynet \
  --horizon-url https://primary.example.com \
  --horizon-endpoints https://primary.example.com,https://backup.example.com

# All commands now use automatic failover
starforge wallet show
starforge wallet fund
starforge deploy --wasm contract.wasm
```

### Testing Endpoint Health
Use the network test command to verify all configured endpoints:

```bash
starforge network test testnet
```
