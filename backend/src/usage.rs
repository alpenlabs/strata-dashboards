use serde::{Deserialize, Serialize};
use std::{sync::Arc, collections::HashMap, collections::HashSet};
use chrono::{Utc, DateTime, Duration, Days, TimeZone, Datelike, TimeDelta};
use rand::Rng;
use hex;
use axum::Json;
use serde_json::{json, Value};
use serde::de::{self, Deserializer};
use tokio::{sync::Mutex, time::interval};
use anyhow::{Result, anyhow};
use log::{info, error};

const USER_OPS_QUERY_URL: &str = "http://localhost/api/v2/proxy/account-abstraction/operations";
const ACCOUNTS_QUERY_URL: &str = "http://localhost/api/v2/proxy/account-abstraction/accounts";


#[derive(Serialize, Deserialize, Clone, Debug)]
struct Account {
    #[serde(deserialize_with = "get_address_hash")]
    address: String,

    #[serde(rename = "creation_timestamp", deserialize_with = "from_null_or_string")]
    deployed_at: String, // ISO 8601 formatted timestamp

    #[serde(default)]
    gas_used: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserOp {
    #[serde(rename = "address", deserialize_with = "get_address_hash")]
    sender: String,

    #[serde(rename = "fee")]
    #[serde(deserialize_with = "convert_to_u64")]
    gas_used: u64,

    #[serde(rename = "timestamp")]
    timestamp: String,
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
    fn new(
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


pub fn get_initial_stats() -> UsageStats {
    let mut stats = HashMap::new();

    let mut user_ops = HashMap::new();
    user_ops.insert("24h".to_string(), 0u64);
    user_ops.insert("30d".to_string(), 0u64);
    user_ops.insert("YTD".to_string(), 0u64);

    let mut gas_used = HashMap::new();
    gas_used.insert("24h".to_string(), 0u64);
    gas_used.insert("30d".to_string(), 0u64);
    gas_used.insert("YTD".to_string(), 0u64);

    let mut unique_active_accounts = HashMap::new();
    unique_active_accounts.insert("24h".to_string(), 0u64);
    unique_active_accounts.insert("30d".to_string(), 0u64);
    unique_active_accounts.insert("YTD".to_string(), 0u64);

    stats.insert("User ops".to_string(), user_ops);
    stats.insert("Gas used".to_string(), gas_used);
    stats.insert("Unique active accounts".to_string(), unique_active_accounts);

    let mut sel_accounts = HashMap::new();
    let mut accounts_created = HashMap::new();
    accounts_created.insert("recent".to_string(), Vec::new());

    let mut gas_consumers = HashMap::new();
    gas_consumers.insert("24h".to_string(), Vec::new());

    sel_accounts.insert("Recent accounts".to_string(), accounts_created);
    sel_accounts.insert("Top gas consumers".to_string(), gas_consumers);

    UsageStats::new(stats, sel_accounts)
}

/// Periodically fetch user operations and accounts and compute usage stats
pub async fn usage_monitoring_task(shared_stats: SharedUsageStats) {
    // info!("Fetching balances...");
    let mut interval = interval(tokio::time::Duration::from_secs(100));

    loop {
        interval.tick().await;
        let now = Utc::now();
        let max_days = max_days_to_monitor(now);
        let start_time = now.checked_sub_days(Days::new(max_days));
        let mut locked_stats = shared_stats.lock().await;
        let result = fetch_user_ops(start_time, Some(now)).await;

        // Aggregate gas used per sender (in the last 24 hours)
        let mut gas_usage: HashMap<String, u64> = HashMap::new();

        match result {
            Ok(user_ops) => {
                info!("🔹 user ops count {}", user_ops.len());
                let time_windows: Vec<(&str, Duration)> = vec![
                    ("24h", Duration::days(1)),
                    ("30d", Duration::days(30)),
                    ("YTD", Duration::days(now.ordinal() as i64)) // Days since Jan 1st
                ];

                // Initialize or reset stats
                for (period, _) in &time_windows {
                    locked_stats.stats.entry("User ops".to_string()).or_default().insert(period.to_string(), 0);
                    locked_stats.stats.entry("Gas used".to_string()).or_default().insert(period.to_string(), 0);
                    locked_stats.stats.entry("Unique active accounts".to_string()).or_default().insert(period.to_string(), 0);
                }

                // Create sets to track unique active accounts per period
                let mut unique_accounts: HashMap<&str, HashSet<String>> = HashMap::new();
                for (period, _) in &time_windows {
                    unique_accounts.insert(period, HashSet::new());
                }

                // compute stats for 24h, 30d and YTD
                for entry in user_ops {
                    if let Ok(op_time) = DateTime::parse_from_rfc3339(&entry.timestamp).map(|dt| dt.with_timezone(&Utc)) {
                        for (period, duration) in &time_windows {
                            if now - *duration <= op_time {
                                *locked_stats.stats.entry("User ops".to_string()).or_default().entry(period.to_string()).or_insert(0) += 1;
                                *locked_stats.stats.entry("Gas used".to_string()).or_default().entry(period.to_string()).or_insert(0) += entry.gas_used;

                                // Track unique senders
                                unique_accounts.get_mut(period).unwrap().insert(entry.sender.clone());
                            }
                        }

                        // Update gas used by sender
                        if now - Duration::days(1) <= op_time {
                            *gas_usage.entry(entry.sender.clone()).or_insert(0) += entry.gas_used;
                        }
                    }
                }

                // Store the count of unique active accounts
                for (period, accounts_set) in unique_accounts {
                    locked_stats.stats.entry("Unique active accounts".to_string())
                        .or_default()
                        .insert(period.to_string(), accounts_set.len() as u64);
                }
            }
            Err(e) =>
            {
                error!("Fetch user ops failed {}", e);
            }
        }

        let result = fetch_accounts(start_time, Some(now)).await;
        match result {
            Ok(accounts) => {
                info!("🔹 accounts count {}", accounts.len());
                // Sort accounts by creation_timestamp (most recent first)
                let mut sorted_accounts: Vec<Account> = accounts
                    .iter()
                    .filter(|acc| acc.deployed_at != "".to_string()) // Ignore accounts without a timestamp
                    .cloned()
                    .collect();

                sorted_accounts.sort_by(|a, b| {
                    let a_time = DateTime::parse_from_rfc3339(&a.deployed_at.as_str())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(Utc::now()); // Default to now if parsing fails
                    let b_time = DateTime::parse_from_rfc3339(b.deployed_at.as_str())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(Utc::now()); 

                    b_time.cmp(&a_time) // Sort descending
                });

                // Take the top 5 most recent accounts
                let recent_accounts = sorted_accounts.into_iter().take(5).collect::<Vec<_>>();

                // Store in shared stats
                locked_stats.sel_accounts.entry("Recent accounts".to_string())
                    .or_default()
                    .insert("recent".to_string(), recent_accounts);
            } 
            Err(e) =>
            {
                error!("Fetch accounts failed {}", e);
            }
        }

        // Top gas consumers: get from gas_usage and sort by gas used (descending)
        let gas_usage_clone = gas_usage.clone();
        let mut top_gas_consumers: Vec<Account> = gas_usage_clone
            .into_iter()
            .map(|(address, gas_used)| Account { address, deployed_at: "".to_string(), gas_used })
            .collect();

        top_gas_consumers.sort_by_key(|acc| gas_usage.get(&acc.address).cloned().unwrap_or(0));
        top_gas_consumers.reverse();
        top_gas_consumers.truncate(5); // Take top 5

        // Store in shared stats
        locked_stats.sel_accounts.entry("Top gas consumers".to_string())
            .or_default()
            .insert("24h".to_string(), top_gas_consumers);
    }
}

fn max_days_to_monitor(now: DateTime<Utc>) -> u64 {
    let ytd_start = Utc.with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0);
    let ytd_days = now.day() as u64 - ytd_start.unwrap().day() as u64;

    if ytd_days > 30 {
        ytd_days
    } else {
        30
    }
}

// Custom deserializer to extract "hash" from the "address" field
fn get_address_hash<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    // Extract "hash" field from "address"
    if let Some(hash) = value.get("hash").and_then(|h| h.as_str()) {
        return Ok(hash.to_string());
    }

    Err(de::Error::missing_field("address.hash"))
}

// Custom deserializer to convert a string to u64
fn convert_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse::<u64>().map_err(de::Error::custom) // Convert string to u64 safely
}

// Custom deserializer to handle `null` timestamps
fn from_null_or_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<Value> = Option::deserialize(deserializer)?;
    match opt {
        Some(Value::String(s)) => Ok(s), // If valid string, return it
        Some(Value::Null) | None => Ok("".to_string()), // If null or missing, return None
        _ => Err(de::Error::custom("Expected a string or null")),
    }
}

async fn fetch_json(endpoint: &str, start_time: Option<DateTime<Utc>>, end_time: Option<DateTime<Utc>>) 
    -> Result<serde_json::Value, anyhow::Error> {

    let http_client = reqwest::Client::new();

    // Construct JSON parameters
    let mut params = json!({
    });

    let format_time = |time: DateTime<Utc>| -> String {
        time.format("%Y-%m-%d%H:%M:%S").to_string() // Correct format: YYYY-MM-DDHH:MM:SS
    };

    if let Some(start_ts) = start_time {
        params["start_time"] = json!(&format_time(start_ts));
    }
    if let Some(end_ts) = end_time {
        params["end_time"] = json!(&format_time(end_ts));
    }

    info!("Sending request to: {} with query params {:?}", endpoint, params);
    // Make an HTTP POST request
    let response = http_client
        .get(endpoint)
        .json(&params)
        .send()
        .await?
        .error_for_status()? // Converts HTTP errors into Rust errors
        .json::<Value>()
        .await?;

    Ok(response)
}

async fn fetch_user_ops(start_time: Option<DateTime<Utc>>, end_time: Option<DateTime<Utc>>) -> Result<Vec<UserOp>, anyhow::Error> {
    info!("🔹 Fetching user operations");

    // Make API call with parameters
    match fetch_json(USER_OPS_QUERY_URL, start_time, end_time).await {
        Ok(data) => {
            // Extract "items" field and deserialize into Vec<UserOps>
            if let Some(items) = data.get("items") {
                let user_ops: Vec<UserOp> = serde_json::from_value(items.clone())
                    .map_err(|e| anyhow!("Failed to deserialize user ops: {}", e))?;
                Ok(user_ops)
            } else {
                error!("Unexpected response");
                Err(anyhow!("Unexpected response"))
            }
        }
        // error!("Failed to fetch User Ops: {:?}", e);
        Err(e) => Err(anyhow!("Fetch user ops failed with {}", e))
    }
}

async fn fetch_accounts(start_time: Option<DateTime<Utc>>, end_time: Option<DateTime<Utc>>) -> Result<Vec<Account>, anyhow::Error> {
    info!("🔹 Fetching accounts");

    // Make API call with parameters
    match fetch_json(ACCOUNTS_QUERY_URL, start_time, end_time).await {
        Ok(data) => {
            // Extract "items" field and deserialize into Vec<Accounts>
            if let Some(items) = data.get("items") {
                let accounts: Vec<Account> = serde_json::from_value(items.clone())
                    .map_err(|e| anyhow!("Failed to deserialize accounts: {}", e))?;
                Ok(accounts)
            } else {
                error!("Unexpected response");
                Err(anyhow!("Unexpected response"))
            }
        },
        Err(e) => Err(anyhow!("Fetch user ops failed with {}", e))
    }
}

pub async fn get_usage_stats(state: SharedUsageStats) -> Json<UsageStats> {
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
    stats.insert("Gas used".to_string(), gas_used);
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