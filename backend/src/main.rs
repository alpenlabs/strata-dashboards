mod config;
mod wallets;
mod utils;
mod bridge;
mod usage;

use axum::{routing::get, Json, Router};
use dotenvy::dotenv;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::core::client::ClientT;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, time::{interval, Duration}, sync::RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;
use tracing::{info, error};

use config::Config;
use crate::wallets::{ SharedWallets, fetch_balances_task, get_wallets_with_balances, init_paymaster_wallets};
use bridge::{BridgeMonitoringConfig, SharedBridgeState, bridge_monitoring_task, get_bridge_status};
use crate::utils::create_rpc_client;
use crate::usage::{
    UsageMonitoringConfig,
    UsageStats,
    usage_monitoring_task,
    get_usage_stats
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
enum Status {
    Online,
    Offline,
}

#[derive(Serialize, Clone, Debug)]
struct NetworkStatus {
    batch_producer: Status,
    rpc_endpoint: Status,
    bundler_endpoint: Status,
}

/// Shared Network State
type SharedNetworkState = Arc<RwLock<NetworkStatus>>;

/// Calls `strata_syncStatus` using `jsonrpsee`
async fn call_rpc_status(client: &HttpClient) -> Status {
    let response: Result<serde_json::Value, _> = client.request("strata_syncStatus", Vec::<()>::new()).await;

    match response {
        Ok(json) => {
            info!(?json, "RPC Response");
            if json.get("tip_height").is_some() {
                Status::Online
            } else {
                Status::Offline
            }
        }
        Err(e) => {
            error!(error = %e, "Could not get status");
            Status::Offline
        }
    }
}

/// Checks bundler health (`/health`)
async fn check_bundler_health(client: &reqwest::Client, config: &Config) -> Status {
    let url = config.bundler_url();
    if let Ok(resp) = client.get(url).send().await {
        let body = resp.text().await.unwrap_or_default();
        if body.contains("ok") {
            return Status::Online;
        }
    }
    Status::Offline
}

/// Periodically fetches real statuses
async fn fetch_statuses_task(state: SharedNetworkState, config: &Config) {
    info!("Fetching statuses...");
    let mut interval = interval(Duration::from_secs(10));
    let rpc_client = create_rpc_client(&config.rpc_url());
    let http_client = reqwest::Client::new();

    loop {
        interval.tick().await;

        let batch_producer = call_rpc_status(&rpc_client).await;
        let rpc_endpoint = call_rpc_status(&rpc_client).await;
        let bundler_endpoint = check_bundler_health(&http_client, config).await;

        let new_status = NetworkStatus {
            batch_producer,
            rpc_endpoint,
            bundler_endpoint,
        };

        info!(?new_status, "Updated Status");

        let mut locked_state = state.write().await;
        *locked_state = new_status;
    }
}

/// Handler to get the current network status
async fn get_network_status(state: SharedNetworkState) -> Json<NetworkStatus> {
    let data = state.read().await.clone();
    Json(data)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenv().ok();

    let config = Arc::new(config::Config::new());

    let cors = CorsLayer::new().allow_origin(Any);

    // Shared state for network status
    let shared_state = Arc::new(RwLock::new(NetworkStatus {
        batch_producer: Status::Offline, // Default state
        rpc_endpoint: Status::Offline,
        bundler_endpoint: Status::Offline,
    }));

    let paymaster_wallets: SharedWallets = init_paymaster_wallets(&config.clone());

    // Spawn a background task to fetch real statuses
    let state_clone = Arc::clone(&shared_state);
    let paymaster_wallets_clone = Arc::clone(&paymaster_wallets);
    tokio::spawn(
    {
        let config = Arc::clone(&config);
        async move {
        fetch_statuses_task(state_clone, &config).await;
        }
    });
    tokio::spawn(
    {
        let config = Arc::clone(&config.clone());
        async move {
            fetch_balances_task(paymaster_wallets_clone, &config).await;
        }
    }
    );

    // Usage monitoring
    let usage_monitoring_config = UsageMonitoringConfig::new();
    let usage_stats = UsageStats::default(&usage_monitoring_config);
    // Shared state for usage stats
    let shared_usage_stats = Arc::new(RwLock::new(usage_stats));
    tokio::spawn(
    {
        let usage_stats_clone = Arc::clone(&shared_usage_stats);
        async move {
            usage_monitoring_task(usage_stats_clone, &usage_monitoring_config).await;
        }
    });

    // bridge monitoring
    let bridge_monitoring_config = BridgeMonitoringConfig::new();
    // Shared state for bridge status
    let bridge_state = SharedBridgeState::default();
    tokio::spawn(
    {
        let bridge_state_clone = Arc::clone(&bridge_state);
        async move {
            bridge_monitoring_task(bridge_state_clone, &bridge_monitoring_config).await;
        }
    });


    let app = Router::new()
        .route("/api/status", get(move || get_network_status(Arc::clone(&shared_state))))
        .route("/api/balances", get(move || get_wallets_with_balances( paymaster_wallets)))
        .route("/api/bridge_status", get(move || get_bridge_status(Arc::clone(&bridge_state))))
        .route("/api/usage_stats", get(move || get_usage_stats(Arc::clone(&shared_usage_stats))))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!(%addr, "Server running at http://");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}