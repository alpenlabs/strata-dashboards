mod config;
mod wallets;
mod utils;
mod db;
mod indexer;
mod usage_stats;

use axum::{routing::get, Json, Router};
use log::info;
use dotenvy::dotenv;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::core::client::ClientT;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, time::{interval, Duration}, sync::Mutex};
use chrono::{Utc, Days};
use std::collections::HashMap;

use tower_http::cors::{Any, CorsLayer};
use config::Config;

use crate::wallets::{ SharedWallets, fetch_balances_task, get_wallets_with_balances, init_paymaster_wallets};
use crate::utils::create_rpc_client;
// use crate::db::DatabaseWrapper;
// use crate::indexer::IndexerConfig;
// use crate::usage_stats::{UsageStats, TimeWindowStats};

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

// Shared State
type SharedState = Arc<Mutex<NetworkStatus>>;

/// Calls `strata_syncStatus` using `jsonrpsee`
async fn call_rpc_status(client: &HttpClient) -> Status {
    let response: Result<serde_json::Value, _> = client.request("strata_syncStatus", Vec::<()>::new()).await;
    
    match response {
        Ok(json) => {
            info!("RPC Response: {:?}", json);
            if json.get("tip_height").is_some() {
                Status::Online
            } else {
                Status::Offline
            }
        }
        Err(e) => {
            info!("🔹 error: {}", e);
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
async fn fetch_statuses_task(state: SharedState, config: &Config) {
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

        info!("Updated Status: {:?}", new_status);

        let mut locked_state = state.lock().await;
        *locked_state = new_status;
    }
}

/// Handler to get the current network status
async fn get_network_status(state: SharedState) -> Json<NetworkStatus> {
    let data = state.lock().await.clone();
    Json(data)
}

#[derive(Serialize, Clone, Debug)]
struct Account {
    address: String,
    deployed_at: String, // ISO 8601 formatted timestamp
    gas_used: u64,
}

#[derive(Serialize, Clone, Debug)]
struct UsageStats {
    user_ops_count: HashMap<String, u64>,
    total_gas_used: HashMap<String, u64>,
    unique_active_accounts: HashMap<String, u64>,
    recent_accounts: Vec<Account>,
    top_gas_consumers: Vec<Account>,
}

// Shared usage stats
type SharedUsageStats = Arc<Mutex<UsageStats>>;

/// Function to generate a list of mock recent accounts.
fn generate_recent_accounts(count: usize) -> Vec<Account> {
    let mut accounts = Vec::new();
    let start_date = Utc::now();
    for i in 0..count {
        let days_offset = Days::new(i as u64);
        accounts.push(Account {
            address: String::from("0xaa"),
            deployed_at: start_date.checked_sub_days(days_offset).unwrap().to_string(),
            gas_used: 100*i as u64,
        });
    }

    accounts
}

async fn get_usage_stats(stats: SharedUsageStats) -> Json<UsageStats> {
    let data = stats.lock().await.clone();
    Json(data)
}

#[tokio::main]
async fn main() {
    // ✅ Initialize logger with info level
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    dotenv().ok();

    let config = Arc::new(config::Config::new());

    let cors = CorsLayer::new().allow_origin(Any);

    // 🔹 Shared state for network status
    let shared_state = Arc::new(Mutex::new(NetworkStatus {
        batch_producer: Status::Offline, // Default state
        rpc_endpoint: Status::Offline,
        bundler_endpoint: Status::Offline,
    }));

    let paymaster_wallets: SharedWallets = init_paymaster_wallets(&config.clone());

    // 🔹 Spawn a background task to fetch real statuses
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

    // Initialize database and user ops fetcher
    // let indexer_config = IndexerConfig::parse();
    let mut user_ops = HashMap::new();
    user_ops.insert("24h".to_string(), 1200u64);
    user_ops.insert("30d".to_string(), 32000u64);
    user_ops.insert("YTD".to_string(), 250000u64);

    let mut gas_used = HashMap::new();
    gas_used.insert("24h".to_string(), 1500000u64);
    gas_used.insert("30d".to_string(), 74000000u64);
    gas_used.insert("YTD".to_string(), 480000000u64);

    let mut unique_active_accounts = HashMap::new();
    unique_active_accounts.insert("24h".to_string(), 48u64);
    unique_active_accounts.insert("30d".to_string(), 450u64);
    unique_active_accounts.insert("YTD".to_string(), 3200u64);

    let shared_usage_stats = Arc::new(Mutex::new(UsageStats {
        user_ops_count: user_ops,
        total_gas_used: gas_used,
        unique_active_accounts: unique_active_accounts,
        recent_accounts: generate_recent_accounts(5),
        top_gas_consumers: generate_recent_accounts(5),
    }));

    // let database = Arc::new(DatabaseWrapper::new(&indexer_config.database_url).await);

    // // 🔹 Shared state for usage stats
    // let shared_state = Arc::new(Mutex::new(UsageStats {
    //     stats: HashMap<String, TimeWindowStats>::new(),
    //     recent_stats: Vec<String>::new(),
    // }));
    // tokio::spawn(
    //     {
    //         let config = Arc::clone(&indexer_config);
    //         async move {
    //             fetch_user_ops_task(database.clone(), &config).await;
    //         }
    //     });

    let app = Router::new()
        .route("/api/status", get(move || get_network_status(Arc::clone(&shared_state))))
        .route("/api/balances", get(move || get_wallets_with_balances( paymaster_wallets)))
        .route("/api/usage_stats", get(move || get_usage_stats(Arc::clone(&shared_usage_stats))))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("🚀 Server running at http://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}