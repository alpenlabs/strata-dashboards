use anyhow::anyhow;
use axum::Json;
use dotenvy::dotenv;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::ClientError;
use log::{info, error, warn};
use jsonrpsee::http_client::HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, sync::Arc, collections::BTreeMap};
use tokio::{sync::Mutex, time::{timeout, Duration, interval}};
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
                "https://strataclient1ff4bc1df.devnet-annapurna.stratabtc.org".to_string()
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

/// Enum to handle deposit and withdrawal operations without relying on a "type" field
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]  // This allows Serde to determine the type based on the structure
enum BridgeDuty {
    Deposit(RpcDepositInfo),
    Withdrawal(RpcWithdrawalInfo),
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcDepositInfo {
    deposit_request_outpoint: String,  // Convert OutPoint to String (txid:vout)
    el_address: String,  // Convert Vec<u8> (EVM Address) to Hex String
    total_amount: u64,   // Amount in satoshis
    take_back_leaf_hash: String,  // Convert TapNodeHash to String
    original_taproot_addr: String,  // Convert BitcoinAddress to String
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcWithdrawalInfo {
    deposit_outpoint: String,  // Convert OutPoint to String (txid:vout)
    user_destination: String,  // Convert Descriptor to String
    assigned_operator_idx: u32,  // Operator index as is
    exec_deadline: u32,  // Bitcoin block height as is
}

/// Struct for `getBridgeDuties` Response
#[derive(Debug, Serialize, Deserialize)]
struct RpcBridgeDuties {
    duties: Vec<BridgeDuty>,  // Mixed deposit & withdrawal duties
    start_index: u64,
    stop_index: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DepositInfo {
    pub deposit_request_txid: String,
    pub deposit_txid: Option<String>,
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
    let mut interval = interval(Duration::from_secs(config.stats_refetch_interval_s));
    let strata_rpc_client = create_rpc_client(&config.strata_rpc_url);
    let bridge_rpc_client = create_rpc_client(&config.bridge_rpc_url);

    loop {
        interval.tick().await;
        let mut locked_state = state.lock().await;

        // Bridge operator status
        let operators = get_bridge_operators(&strata_rpc_client).await.unwrap();
        let mut operator_statuses = Vec::new();
        for (index, public_key) in operators.0.iter() {
            let operator_id = format!("Alpen Labs #{}", index);
            let status = get_operator_status(&config, &strata_rpc_client, *index).await.unwrap();

            operator_statuses.push(OperatorStatus {
                operator_id,
                operator_address: public_key.clone(),
                status,
            });
        }

        locked_state.operators = operator_statuses;

        // Current deposits
        let current_deposits = get_current_deposits(&strata_rpc_client).await.unwrap();
        info!("current deposits {:?}", current_deposits);

        for deposit_id in current_deposits {
            let deposit_info = get_deposit_info(&strata_rpc_client, &bridge_rpc_client, deposit_id).await.unwrap();
            if deposit_info.is_some() {
                info!("found deposit entry {}", deposit_id);
                locked_state.deposits.push(deposit_info.unwrap());
            } else {
                warn!("Missing deposit entry for idx {}", deposit_id);
            }
        }

        // Withdrawals
        for (index, _) in operators.0.iter() {
            let mut withdrawal_infos: Vec<WithdrawalInfo> = match get_withdrawals(&bridge_rpc_client, *index).await {
                Ok(data) => data,
                Err(e) => {
                    error!("Bridge get withdrawal failed with {}", e);
                    Vec::new()
                }
            };
            locked_state.withdrawals.append(& mut withdrawal_infos);
        }

        // Reimbursements        
        let reimbursements: Vec<ReimbursementInfo> = match get_reimbursements(&bridge_rpc_client).await {
            Ok(data) => data,
            Err(e) => {
                error!("Bridge get withdrawal failed with {}", e);
                Vec::new()
            }
        };
        locked_state.reimbursements = reimbursements;
    }
}

async fn get_bridge_operators(strata_client: &HttpClient) -> Result<OperatorPublicKeys, ClientError> {
    // Fetch active operator public keys
    let operator_table: OperatorPublicKeys = match strata_client.request("strata_getActiveOperatorChainPubkeySet", ((),)).await {
        Ok(data) => data,
        Err(e) => {
            error!("Bridge status query failed with {}", e);
            return Err(e);
        }
    };

    Ok(operator_table)
}

async fn get_operator_status(config: &BridgeMonitoringConfig, bridge_client: &HttpClient, operator_idx: u32) -> Result<String, ClientError> {
    // Check if operator responds to an RPC request
    // Explicitly define return type as `bool`
    let status = timeout(
            Duration::from_secs(config.bridge_operator_ping_timeout_s),
            bridge_client.request("stratabridge_operatorStatus", (operator_idx,))
        )
        .await
        .map_err(|_| ClientError::Custom("Timeout".into())) // ❌ Timeout → Return error
        .and_then(|res| res.map_err(|_| ClientError::Custom("RPC Error".into()))) // ❌ RPC failure → Error
        .unwrap_or(false); // ❌ Any failure → Default to `false` (Offline)

    let status = if status {
        "Online".to_string()
    } else {
        "Offline".to_string()
    };

    Ok(status)
}

async fn get_current_deposits(strata_client: &HttpClient) -> Result<Vec<u32>, ClientError> {
    let deposit_ids: Vec<u32> = match strata_client.request("strata_getCurrentDeposits", ((),)).await {
        Ok(data) => data,
        Err(e) => {
            error!("Current deposits query failed with {}", e);
            return Err(e);
        }
    };

    Ok(deposit_ids)
}

fn mock_deposit_info(deposit_outpoint: String, deposit_status: String) -> DepositInfo {
    let output_prefix: String = deposit_outpoint.chars().take(10).collect();
    DepositInfo {
        deposit_request_txid: format!("abcdefgh{}", output_prefix).into(),
        deposit_txid: format!("12345678{}", output_prefix).into(),
        status: deposit_status,
    }
}

async fn get_deposit_info(strata_client: &HttpClient, bridge_client: &HttpClient, deposit_id: u32) -> Result<Option<DepositInfo>, ClientError> {

    let response: Value = match strata_client
        .request("strata_getCurrentDepositById", (deposit_id,))
        .await
    {
        Ok(resp) => {
            info!("deposit entry {:?}", resp);
            resp
        }
        Err(e) => {
            warn!("⚠️ Skipping deposit {} due to RPC error: {}", deposit_id, e);
            return Ok(None);
        }
    };

    // ✅ Extract "deposit_txid", return None if missing
    let deposit_outpoint = response.get("output")
        .and_then(|v| Some(v.to_string().trim_matches('"').to_string()));
    let deposit_status = response.get("state")
        .and_then(|state| {
            if let Some(state_str) = state.as_str() {
                Some(state_str.to_string()) // ✅ Case 1: State is a String
            } else if let Some(state_obj) = state.as_object() {
                state_obj.keys().next().map(|key| key.to_string()) // ✅ Case 2: State is an Object
            } else {
                None
            }
        })
        .map(|state| {
            let mut chars = state.chars();
            chars.next()
                .map(|c| c.to_uppercase().to_string() + chars.as_str())
                .unwrap_or(state) // Handle empty string case
        })
        .unwrap_or("-".to_string()); // Default to "Unknown" if missing

    let deposit_info = mock_deposit_info(deposit_outpoint.unwrap(), deposit_status);
    info!("deposit info {:?}", deposit_info);

    // let deposit_info: DepositInfo = match bridge_client.request("stratabridge_depositInfo", (deposit_outpoint,)).await {
    //     Ok(data) => data,
    //     Err(e) => {
    //         error!("Get deposit by id failed with {}", e);
    //         return Err(e);
    //     }
    // };

    Ok(Some(deposit_info))
}

fn mock_withdrawal_info(operator_idx: u32) -> Vec<WithdrawalInfo> {
    let mut withdrawals = Vec::new();
    withdrawals.push(WithdrawalInfo {
        withdrawal_request_txid: format!("aabbccddee{}", operator_idx).to_string(),
        fulfillment_txid: Some(format!("ffcdbade{}", operator_idx).to_string()),
    status: "Accepted".to_string(),
    });

    withdrawals
}

async fn get_withdrawals(bridge_client: &HttpClient, operator_idx: u32) -> Result<Vec<WithdrawalInfo>, ClientError> {
    let withdrawal_infos = mock_withdrawal_info(operator_idx);

    // let bridge_duties: RpcBridgeDuties = match bridge_client.request("stratabridge_bridgeDuties", operator_idx)).await {
    //     Ok(data) => data,
    //     Err(e) => {
    //         error!("Get bridge duties failed with {}", e);
    //         return Err(e);
    //     }
    // };

    // let mut withdrawal_infos = Vec::new();
    // for duty in bridge_duties.duties.iter() {
    //     if let BridgeDuty::Withdrawal(withdrawal_info) = duty {
    //         let deposit_outpoint = withdrawal_info.deposit_outpoint.clone();
    //         println!("Calling getWithdrawalInfo for Outpoint: {}", deposit_outpoint);

    //         // Call `getWithdrawalInfo(deposit_outpoint)` here
    //         let wd_info: WithdrawalInfo = match bridge_client.request("stratabridge_withdrawalInfo", (deposit_outpoint.clone(), )).await {
    //             Ok(data) => data,
    //             Err(e) => {
    //                 error!("Get withdrawal info failed with {}", e);
    //                 return Err(e);
    //             }
    //         };

    //         withdrawal_infos.push(wd_info);
    //     }
    // }

    Ok(withdrawal_infos)
}

fn mock_reimbursement_infos() -> Vec<ReimbursementInfo> {

    let mut reimbursements = Vec::new();

    for i in 1..=4 {
        reimbursements.push(
            ReimbursementInfo {
                claim_txid: format!("fedcbaabcdef123{}", i).to_string(),
                payout_txid: Some(format!("123fedcbaabcdef{}", i).to_string()),
                challenge_step: "N/A".to_string(),
                status: "Complete".to_string(),
        });
    }

    reimbursements
}

async fn get_reimbursements(bridge_client: &HttpClient) -> Result<Vec<ReimbursementInfo>, ClientError> {
    let reimbursement_infos = mock_reimbursement_infos();

    // let claim_txids: Vec<String> = match bridge_client.request("stratabridge_getClaims", ((),)).await {
    //     Ok(data) => data,
    //     Err(e) => {
    //         error!("Get bridge claims failed with {}", e);
    //         return Err(e);
    //     }
    // };

    // let mut reimbursement_infos = Vec::new();
    // for txid in claim_txids.iter() {
        // Call `getWithdrawalInfo(deposit_outpoint)` here
        // let reimb_info: ReimbursementInfo = match rpc_client.request("stratabridge_getClaimInfo", (txid.clone(), )).await {
        //     Ok(data) => data,
        //     Err(e) => {
        //         error!("Get claim info failed with {}", e);
        //         return Err(e);
        //     }
        // };

        // reimbursement_infos.push(wd_info);
    // }

    Ok(reimbursement_infos)
}

pub async fn get_bridge_status(state: BridgeState) -> Json<BridgeStatus> {
    let data = state.lock().await.clone();
    Json(data)
}
