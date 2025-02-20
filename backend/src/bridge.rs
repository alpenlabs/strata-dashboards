use jsonrpsee::http_client::HttpClient;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::ClientError;
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc, collections::BTreeMap};
use tokio::{sync::Mutex, time::{timeout, Duration, interval}};
use dotenvy::dotenv;
use axum::Json;
use log::{info, error};
use crate::utils::create_rpc_client;


pub struct BridgeMonitoringConfig {
    strata_rpc_url: String,
    bridge_rpc_url: String,
    stats_refetch_interval_s: u64,
    bridge_operator_ping_timeout_s: u64,
}

impl BridgeMonitoringConfig {
    pub fn new() -> Self {
        dotenv().ok(); // Load `.env` file if present

        let strata_rpc_url = env::var("STRATA_RPC_URL").ok()
            .unwrap_or_else(|| {
                "https://fnclientbcc5fe8c4454c314eb0a00cd3882.devnet-annapurna.stratabtc.org/".to_string()
            });

        let bridge_rpc_url = env::var("STRATA_BRIDGE_RPC_URL").ok()
            .unwrap_or_else(|| "https://strataclient1ff4bc1df.devnet-annapurna.stratabtc.org".to_string());

        let refresh_interval_s = env::var("BRIDGE_STATUS_REFETCH_INTERVAL_S").ok()
            .unwrap_or_else(|| "120000".to_string());
        let refetch_interval_s_u64: u64 = refresh_interval_s.parse().expect("Failed to parse BRIDGE_STATUS_REFETCH_INTERVAL_S as u64");

        let ping_timeout_s = env::var("BRIDGE_OPERATOR_PING_TIMEOUT_S").ok()
            .unwrap_or_else(|| "120000".to_string());
        let ping_timeout_s_u64: u64 = ping_timeout_s.parse().expect("Failed to parse BRIDGE_OPERATOR_PING_TIMEOUT_S as u64");

        info!("🔹 Strata rpc url {}, bridge rpc url {}", strata_rpc_url, bridge_rpc_url);

        BridgeMonitoringConfig {
            strata_rpc_url,
            bridge_rpc_url,
            stats_refetch_interval_s: refetch_interval_s_u64,
            bridge_operator_ping_timeout_s: ping_timeout_s_u64,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OperatorStatus {
    operator_id: String,
    operator_address: String,
    status: String,
}


// Define response structure to match PublickeyTable from RPC
#[derive(Debug, Deserialize)]
struct OperatorPublicKeys(BTreeMap<u32, String>); // OperatorIdx -> PublicKey

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DepositInfo {
    pub deposit_request_txid: String,
    pub deposit_txid: Option<String>,
    pub mint_txid: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WithdrawalInfo {
    pub withdrawal_request_txid: String,
    pub fulfillment_txid: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReimbursementInfo {
    pub claim_txid: String,
    pub challenge_step: String,
    pub payout_txid: Option<String>,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BridgeStatus {
    operators: Vec<OperatorStatus>,
    deposits: Vec<DepositInfo>,
    withdrawals: Vec<WithdrawalInfo>,
    reimbursements: Vec<ReimbursementInfo>,
}

impl BridgeStatus {
    pub fn default() -> BridgeStatus {
        BridgeStatus {
            operators: Vec::new(),
            deposits: Vec::new(),
            withdrawals: Vec::new(),
            reimbursements: Vec::new(),
        }
    }
}

// Shared usage stats
pub type BridgeState = Arc<Mutex<BridgeStatus>>;

/// Periodically fetch bridge status and update shared bridge state
pub async fn bridge_monitoring_task(state: BridgeState, config: &BridgeMonitoringConfig) {
    let mut interval = interval(tokio::time::Duration::from_secs(config.stats_refetch_interval_s));
    let strata_rpc_client = create_rpc_client(&config.strata_rpc_url);
    let bridge_rpc_client = create_rpc_client(&config.bridge_rpc_url);

    loop {
        interval.tick().await;
        let mut locked_state = state.lock().await;

        let operators = get_bridge_operators(&config, &strata_rpc_client, &bridge_rpc_client).await.unwrap();
        info!("operator status {}", operators.len());
        locked_state.operators = operators;
    }
}

pub async fn get_bridge_operators(config: &BridgeMonitoringConfig, strata_rpc: &HttpClient, bridge_rpc: &HttpClient) -> Result<Vec<OperatorStatus>, ClientError> {
    // Fetch active operator public keys
    // let operator_table: OperatorPublicKeys = match strata_rpc.request("getActiveOperatorChainPubkeySet", ((),)).await {
    //     Ok(data) => data,
    //     Err(e) => {
    //         error!("Bridge status query failed with {}", e);
    //         return Err(e);
    //     }
    // };

    let mut statuses = Vec::new();

    // for (index, public_key) in operator_table.0.iter() {
    //     let operator_id = format!("Alpen Labs #{}", index);

    //     // Check if operator responds to an RPC request
    //     // Explicitly define return type as `bool`
    //     let ping_result: Result<bool, ClientError> =
    //         timeout(Duration::from_secs(config.bridge_operator_ping_timeout_s), bridge_rpc.request("pingOperator", (public_key.clone(),)))
    //             .await
    //             .map_err(|_| ClientError::Custom("Timeout".into()))?;

    //     let status = if ping_result.is_ok() && ping_result.unwrap() {
    //         "Online".to_string()
    //     } else {
    //         "Offline".to_string()
    //     };

    //     statuses.push(OperatorStatus {
    //         operator_id,
    //         operator_address: public_key.clone(),
    //         status,
    //     });
    // }

    // Mock status for now
    for i in 1..=3 {
        statuses.push(OperatorStatus {
            operator_id: format!("Alpen Labs #{}", i),
            operator_address: format!("0xdeadbeef{}", i),
            status: "Online".to_string(),
        })
    }

    Ok(statuses)
}


// pub async fn get_current_deposits() -> Result<Vec<String>, reqwest::Error> {
//     let client = Client::new();
//     let rpc_url = env::var("STRATA_RPC_URL").unwrap();
//     let response = client.get(format!("{}/getCurrentDeposits", rpc_url)).send().await?;
//     let deposit_indexes: Vec<String> = response.json().await?;
//     Ok(deposit_indexes)
// }

// pub async fn get_deposit_info(deposit_txid: &str) -> Result<DepositInfo, reqwest::Error> {
//     let client = Client::new();
//     let rpc_url = env::var("BRIDGE_ORCHESTRATOR_URL").unwrap();
//     let response = client.get(format!("{}/getDepositInfo/{}", rpc_url, deposit_txid)).send().await?;
//     let deposit_info: DepositInfo = response.json().await?;
//     Ok(deposit_info)
// }

// pub async fn get_withdrawal_info(outpoint: &str) -> Result<Option<WithdrawalInfo>, reqwest::Error> {
//     let client = Client::new();
//     let rpc_url = env::var("BRIDGE_ORCHESTRATOR_URL").unwrap();
//     let response = client.get(format!("{}/getWithdrawalInfo/{}", rpc_url, outpoint)).send().await?;
//     let withdrawal_info: Option<WithdrawalInfo> = response.json().await?;
//     Ok(withdrawal_info)
// }

// pub async fn get_claim_info(claim_txid: &str) -> Result<ReimbursementInfo, reqwest::Error> {
//     let client = Client::new();
//     let rpc_url = env::var("BRIDGE_ORCHESTRATOR_URL").unwrap();
//     let response = client.get(format!("{}/getClaimInfo/{}", rpc_url, claim_txid)).send().await?;
//     let claim_info: ReimbursementInfo = response.json().await?;
//     Ok(claim_info)
// }

// pub async fn bridge_deposits() -> Json<Vec<rpc_client::DepositInfo>> {
//     let deposits = rpc_client::get_current_deposits().await.unwrap_or_default();
//     let mut deposit_infos = Vec::new();

//     for deposit_txid in deposits {
//         if let Ok(info) = rpc_client::get_deposit_info(&deposit_txid).await {
//             deposit_infos.push(info);
//         }
//     }

//     Json(deposit_infos)
// }

// pub async fn bridge_withdrawals() -> Json<Vec<rpc_client::WithdrawalInfo>> {
//     let withdrawals = vec!["outpoint_1", "outpoint_2"];
//     let mut withdrawal_infos = Vec::new();

//     for outpoint in withdrawals {
//         if let Ok(Some(info)) = rpc_client::get_withdrawal_info(&outpoint).await {
//             withdrawal_infos.push(info);
//         }
//     }

//     Json(withdrawal_infos)
// }

// pub async fn bridge_reimbursements() -> Json<Vec<rpc_client::BridgeReimbursement>> {
//     let claims = vec!["claim_txid_1", "claim_txid_2"];
//     let mut claim_infos = Vec::new();

//     for claim in claims {
//         if let Ok(info) = rpc_client::get_claim_info(&claim).await {
//             claim_infos.push(info);
//         }
//     }

//     Json(claim_infos)
// }

pub async fn get_bridge_status(state: BridgeState) -> Json<BridgeStatus> {
    let data = state.lock().await.clone();
    Json(data)
}