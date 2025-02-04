use serde::{Deserialize, Serialize};
use std::{env, sync::Arc, collections::HashMap};
use chrono::{Utc, TimeDelta};
use sqlx::Row;
use rand::Rng;
use hex;
use axum::Json;
use dotenvy::dotenv;
use serde_json::json;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::core::client::ClientT;
use tokio::{sync::Mutex, time::interval, time::Duration};
use anyhow::{Result, anyhow};
// use ethers::prelude::*;
// use ethers::providers::{Ws, Provider};
use log::{info, error};

use crate::db::{DbPool, init_db_pool};
use crate::utils::create_rpc_client;
use crate::pgu64::PgU64;


#[derive(Debug, Clone)]
pub struct UsageMonitorConfig {
    /// PostgreSQL database url
    pub database_url: String,

    /// JSON-RPC Endpoint for eth logs
    pub rpc_url: String,

    /// Canonical EntryPoint address
    pub entrypoint_address: String,

    /// keccak256 of UserOperationEvent signature
    pub userop_event_topic: String,

    /// Batch size for querying eth logs e.g. 2000
    pub batch_size: u64,
}

impl UsageMonitorConfig {
    pub fn new() -> Self {
        dotenv().ok(); // Load `.env` file if present

        let database_url = env::var("DATABASE_URL").ok()
            .unwrap_or_else(|| {"https://bundler.devnet-annapurna.stratabtc.org/hth".to_string()});

        // let rpc_url = env::var("RETH_URL").ok()
        //   .unwrap_or_else(|| {"https://stratareth3666f0713.devnet-annapurna.stratabtc.org/".to_string()});
        let rpc_url = "https://stratareth3666f0713.devnet-annapurna.stratabtc.org/".to_string();

        // let entrypoint_address = env::var("ENTRYPOINT_ADDRESS").ok()
        //     .unwrap_or_else(|| {"0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string()});
        let entrypoint_address = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string();

        let userop_event_topic = env::var("USEROP_EVENT_TOPIC").ok()
            .unwrap_or_else(|| {"0xd88f804a9b3fa8f0bb160c49edc57dff967f62d407125fba2377b89f77dde8f6".to_string()});

        let batch_size = env::var("USEROP_QUERY_BATCH_SIZE").ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or_else(|| {1000});

        info!(
            "🔹 Loaded IndexerConfig: database_url = {}, rpc_url = {}, entrypoint_address = {}",
            database_url, rpc_url, entrypoint_address
        );

        UsageMonitorConfig {
            database_url,
            rpc_url,
            entrypoint_address,
            userop_event_topic,
            batch_size,
        }
    }

    /// Getter for `database_url`
    pub fn database_url(&self) -> String {
        self.database_url.clone()
    }

    /// Getter for `rpc_url`
    pub fn rpc_url(&self) -> String {
        self.rpc_url.clone()
    }

    /// Getter for `entrypoint_address`
    pub fn entrypoint_address(&self) -> String {
        self.entrypoint_address.clone()
    }

    /// Getter for `userop_event_topic`
    pub fn userop_event_topic(&self) -> String {
        self.userop_event_topic.clone()
    }

    /// Getter for `batch_size`
    pub fn batch_size(&self) -> u64 {
        self.batch_size
    }
}

#[derive(Debug, Clone)]
pub struct UsageMonitorHandle {
    pub config: UsageMonitorConfig,
    pub db_pool: DbPool,
    pub rpc_client: HttpClient,
}

#[derive(Debug, Deserialize)]
pub struct EthLog {
    pub sender: String,
    pub topics: Vec<String>,
    pub gas_used: String,
    pub block_number: String,
    pub transaction_hash: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Account {
    address: String,
    deployed_at: String, // ISO 8601 formatted timestamp
    gas_used: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UsageStats {
    // First level key is stat (e.g. "User ops", "Gas used").
    // Second level key is time period ("24h", "30d", "YTD").
    stats: HashMap<String, HashMap<String, u64>>,
    // First level key is stat (e.g. "Recent accounts", "Top gas consumers").
    // Second level key is time period ("recent", "24h").
    sel_accounts: HashMap<String, HashMap<String, Vec<Account>>>,
}

impl UsageStats {
    pub fn new(
        stats: HashMap<String, HashMap<String, u64>>,
        sel_accounts: HashMap<String, HashMap<String, Vec<Account>>>,
    ) -> Self {
        Self {
            stats,
            sel_accounts,
        }
    }
}

// Shared usage stats
pub type SharedUsageStats = Arc<Mutex<UsageStats>>;

pub async fn init_usage_monitor(config: &UsageMonitorConfig) -> Result<UsageMonitorHandle, anyhow::Error> {
    // 1) Connect to DB
    let db_pool = init_db_pool(&config.database_url()).await?;
    // 2) Create indexer_state table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS indexer_state (
            id SERIAL PRIMARY KEY,
            last_processed_block BIGINT NOT NULL
        )
    "#)
    .execute(&db_pool)
    .await?;

    // 3) Insert an initial row
    sqlx::query(r#"
        INSERT INTO indexer_state (id, last_processed_block)
        VALUES (1, $1)
        ON CONFLICT (id) DO NOTHING
    "#)
    .bind(PgU64::from_u64(1).to_i64())
    .bind(PgU64::from_u64(0).to_i64())
    .execute(&db_pool)
    .await?;

    // 3) Create a table to index user ops
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS user_ops (
            id SERIAL PRIMARY KEY,
            sender TEXT NOT NULL,                   -- e.g. from topics[2]
            block_number BIGINT NOT NULL,           -- block number (decoded from hex)
            transaction_hash TEXT NOT NULL,         -- transaction hash for reference
            gas_used BIGINT NOT NULL,               -- gas used
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
    "#)
    .execute(&db_pool)
    .await?;

    // TODO
    // -- Create indexes for faster querying on important fields:
    // CREATE INDEX idx_user_ops_block_number ON user_ops(block_number);
    // CREATE INDEX idx_user_ops_sender         ON user_ops(sender);
    // CREATE INDEX idx_user_ops_tx_hash        ON user_ops(transaction_hash);

    // Create an rpc client
    let rpc_client = create_rpc_client(&config.rpc_url());
    Ok(UsageMonitorHandle {
        config: config.clone(),
        db_pool,
        rpc_client,
    })
}

pub async fn usage_indexer_task(handle: UsageMonitorHandle) {
    info!("calling get_ethLogs for UserOperationEvent ...");
    let mut interval = interval(Duration::from_secs(10));

    // 1) Get latest block
    let latest_block = get_latest_block_number(&handle.rpc_client).await.unwrap();
    info!("🔹 latest block number {}", latest_block);

    // 2) Load last processed block
    let mut from_block = load_last_processed_block(&handle.db_pool).await.unwrap();
    info!("🔹 from block number {}", from_block);

    // 3) Loop in batches
    while from_block <= latest_block {
        let to_block = std::cmp::min(from_block + handle.config.batch_size(), latest_block);

        // 4) Fetch logs
        let logs = fetch_logs_in_range(
            &handle.rpc_client,
            &handle.config.entrypoint_address(),
            &handle.config.userop_event_topic(),
            from_block,
            to_block
        )
        .await.unwrap();

        // 5) Insert logs into DB
        for eth_log in logs {
            // parse each log's data
            // decode userOpHash, sender, etc. from topics/data
            _ = insert_user_operation(&handle.db_pool, &eth_log).await;
            info!("🔹 Log {}, {}", &eth_log.sender, eth_log.gas_used);
        }

        // 6) Update last processed block
        _ = save_last_processed_block(&handle.db_pool, to_block).await;
        info!("✅ Indexed blocks {} -> {}", from_block, to_block);

        if to_block == latest_block {
            break;
        }
        from_block = to_block + 1;

        // Sleep briefly to avoid rate-limit
        interval.tick().await;
    }
}

pub fn parse_u64_from_hex(hex_str: &str) -> Result<u64> {
    // Remove the "0x" prefix if it exists.
    let trimmed = hex_str.trim_start_matches("0x");

    // Attempt to parse the remaining string as a hexadecimal number.
    u64::from_str_radix(trimmed, 16)
        .map_err(|e| anyhow!("Error parsing hex string '{}': {}", hex_str, e))
}

pub async fn get_latest_block_number(client: &HttpClient) -> Result<u64> {
    info!("🔹 Fetching latest block number");

    let params: [(); 0] = [];
    // The request can fail => we use `?` to propagate the error
    let response: serde_json::Value = client.request("eth_blockNumber", params).await?;

    // Extract the "result" field as a string
    let hex_str = response
        .as_str()
        .ok_or_else(|| anyhow!("Expected the response to be a string, but got: {:?}", response))?;

    // Remove the "0x" prefix (if present) and parse the hex string into a u64
    let block_num = parse_u64_from_hex(hex_str)?;

    Ok(block_num)
}

async fn load_last_processed_block(db_pool: &DbPool) -> Result<u64> {
    let row = sqlx::query(
        r#"SELECT last_processed_block FROM indexer_state WHERE id = 1"#
    )
    .fetch_one(db_pool)
    .await?;

    // Extract the column by name; returns an i64
    let last_processed_block: i64 = row.try_get("last_processed_block")?;;

    // Ok(PgU64::from_i64(last_processed_block).0)

    Ok(last_processed_block as u64)
}

pub async fn fetch_logs_in_range(
    client: &HttpClient,
    entrypoint_address: &str,
    userop_event_topic: &str,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<EthLog>> {
    // Convert numeric block to hex (e.g., 0x1234)
    let from_block_hex = format!("0x{:x}", from_block);
    let to_block_hex   = format!("0x{:x}", to_block);

    // Build the filter object
    let filter = json!({
        "fromBlock": from_block_hex,
        "toBlock": to_block_hex,
        "address": entrypoint_address,
        "topics": [userop_event_topic]
    });

    info!("🔹 filter {}", filter);
    // Pass the filter as params to `eth_getLogs`
    // The generic type `Vec<EthLog>` tells jsonrpsee how to deserialize the result
    let result: Result<Vec<serde_json::Value>, _> = client.request("eth_getLogs", (filter.clone(),)).await;
    // .map_err(|err| anyhow!("JSON-RPC error: {err}"))?;
    match result {
        Ok(raw_response) => {
            info!(
                "✅ Raw JSON response: {}",
                serde_json::to_string_pretty(&raw_response).unwrap_or_else(|_| "Failed to format JSON".to_string())
            );
            // Ok(raw_response)
        }
        Err(err) => {
            error!("🚨 JSON-RPC request failed: {:?}", err);
            // Err(anyhow!("JSON-RPC request error: {:?}", err))
        }
    }

    let logs_result: Result<Vec<EthLog>, _> = client.request("eth_getLogs", (filter,)).await;

    match logs_result {
        Ok(logs) => {
            info!("✅ Successfully fetched {} logs", logs.len());
            Ok(logs)
        },
        Err(err) => {
            error!("🚨 JSON-RPC request failed: {:?}", err);
            Err(anyhow!("JSON-RPC request error: {:?}", err))
        }
    }
}

pub fn parse_sender_from_topics(topics: &Vec<String>) -> Result<String> {
    if topics.len() < 3 {
        return Err(anyhow!("Expected at least 3 topics, got {}", topics.len()));
    }

    let topic = &topics[2];

    // A valid 32-byte hex string (with a "0x" prefix) should be 66 characters.
    if topic.len() != 66 {
        return Err(anyhow!(
            "Unexpected topic length: expected 66 characters, got {} for topic: {}",
            topic.len(),
            topic
        ));
    }

    // Extract the last 40 characters.
    // Since the string starts with "0x", the actual data is from index 2 to 66.
    // The first 24 hex characters (indices 2..26) are usually padding zeros,
    // and the actual address is in indices 26..66.
    let sender = &topic[26..66];

    // Return the sender address with a "0x" prefix.
    Ok(format!("0x{}", sender))
}

/// Parses the gas used (actualGasCost) from the event data.
/// 
/// # Arguments
/// 
/// * `data` - A hex string (with a "0x" prefix) containing the ABI‑encoded non‑indexed event parameters.
/// 
/// # Returns
/// 
/// A `Result<u64>` containing the parsed gas used.  
/// 
/// # Example
/// 
/// If `data` is something like:
/// "0x{nonce}{gasCost}{actualGasPrice}..."
/// where each of `{nonce}` and `{gasCost}` are 64 hex characters,
/// this function extracts the second 64‑character block as gas used.
pub fn parse_gas_used_from_data(data: &str) -> Result<u64> {
    // Remove the "0x" prefix, if present.
    let trimmed = data.trim_start_matches("0x");

    // Ensure the string is long enough to contain at least two 32-byte values.
    if trimmed.len() < 128 {
        return Err(anyhow!(
            "Data is too short: expected at least 128 hex digits, got {} in '{}'",
            trimmed.len(),
            data
        ));
    }

    // The first 64 hex digits are for the nonce; the next 64 are for actualGasCost.
    let gas_used_hex = &trimmed[64..128];

    // Parse the gas_used_hex as a hexadecimal number into u64.
    // (Typically, gas cost values are well below 2^64.)
    let gas_used = u64::from_str_radix(gas_used_hex, 16)
        .map_err(|e| anyhow!("Error parsing gas used from '{}': {}", gas_used_hex, e))?;

    Ok(gas_used)
}

pub async fn insert_user_operation(db_pool: &DbPool, log: &EthLog) -> Result<()> {
    // decode fields
    let block_num = parse_u64_from_hex(&log.block_number).unwrap();
    let sender = parse_sender_from_topics(&log.topics).unwrap(); // e.g. topics[2]
    let gas_used = parse_gas_used_from_data(&log.data).unwrap(); // depends on the event
    info!("🔹 user op {}, {}, {}", &sender, gas_used, block_num);

    // insert user op
    sqlx::query(r#"
            INSERT INTO user_ops (sender, block_number, transaction_hash, gas_used)
            VALUES ($1, $2, $3, $4)
        "#
    )
    .bind(sender)                       // Bind the sender address (String)
    .bind(PgU64::from_u64(block_num).to_i64())             // Bind the block number
    .bind(&log.transaction_hash)        // Bind the transaction hash (String)
    .bind(PgU64::from_u64(gas_used).to_i64())
    .execute(db_pool)
    .await?;

    Ok(())
}

async fn save_last_processed_block(db_pool: &DbPool, block_num: u64) -> Result<()> {
    sqlx::query(r#"
            UPDATE indexer_state 
            SET last_processed_block = $1
            WHERE id = 1
            "#
    )
    .bind(PgU64::new(block_num).to_i64())   // Bind the block numbder
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn get_usage_stats(state: SharedUsageStats, handle: UsageMonitorHandle) -> Json<UsageStats> {
    let data = state.lock().await.clone();
    Json(data)
}

/// Function to generate a random Ethereum-style address.
fn generate_random_address() -> String {
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 20] = rng.gen();
    format!("0x{}", hex::encode(random_bytes))
}

/// Function to generate a list of mock recent accounts.
fn generate_recent_accounts(count: usize) -> Vec<Account> {
    let mut accounts = Vec::new();
    let start_date = Utc::now();
    for i in 0..count {
        let time_delta = TimeDelta::minutes(i as i64);
        accounts.push(Account {
            address: generate_random_address(),
            deployed_at: start_date.checked_sub_signed(time_delta).unwrap().to_string(),
            gas_used: 100*(i+1) as u64,
        });
    }

    accounts
}

// Mock data to test backend <> frontend
pub fn get_mock_usage_stats() -> UsageStats {
    let mut stats = HashMap::new();

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

    stats.insert("User ops".to_string(), user_ops);
    stats.insert("Total gas".to_string(), gas_used);
    stats.insert("Unique active accounts".to_string(), unique_active_accounts);

    let mut sel_accounts = HashMap::new();
    let mut accounts_created = HashMap::new();
    let recent_accounts = generate_recent_accounts(5);
    accounts_created.insert("recent".to_string(), recent_accounts);

    let mut gas_consumers = HashMap::new();
    let top_gas_consumers = generate_recent_accounts(5);
    gas_consumers.insert("24h".to_string(), top_gas_consumers);

    sel_accounts.insert("Recent accounts".to_string(), accounts_created);
    sel_accounts.insert("Top gas consumers".to_string(), gas_consumers);

    UsageStats::new(stats, sel_accounts)
}