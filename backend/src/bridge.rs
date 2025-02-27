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
                "https://strataseq.temp6-testnet1-staging.stratabtc.org/".to_string()
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
pub enum BridgeDuty {
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

        // Bridge operator status
        let operators = get_bridge_operators(&strata_rpc_client).await.unwrap();
        let mut operator_statuses = Vec::new();
        for (index, public_key) in operators.0.iter() {
            let operator_id = format!("Alpen Labs #{}", index);
            info!("operator {}", operator_id);
            let status = get_operator_status(&config, &strata_rpc_client, public_key).await.unwrap();

            operator_statuses.push(OperatorStatus {
                operator_id,
                operator_address: public_key.clone(),
                status,
            });
        }

        locked_state.operators = operator_statuses;

        // Current deposits
        let current_deposits = get_current_deposits(&bridge_rpc_client).await.unwrap();
        info!("current deposits {}", current_deposits.len());

        for deposit_id in current_deposits {
            let deposit_info = get_deposit_info(&bridge_rpc_client, deposit_id).await.unwrap();
            locked_state.deposits.push(deposit_info);
        }

        // Withdrawals
        for (index, _) in operators.0.iter() {
            let operator_id = format!("Alpen Labs #{}", index);
            info!("operator {}", operator_id);
            let mut withdrawal_infos: Vec<WithdrawalInfo> = match get_withdrawal_info(&strata_rpc_client, *index).await {
                Ok(data) => data,
                Err(e) => {
                    error!("Bridge get withdrawal failed with {}", e);
                    Vec::new()
                }
            };
            locked_state.withdrawals.append(& mut withdrawal_infos);
        }
    }
}

// Mock operator data for testing
fn mock_operator_table() -> OperatorPublicKeys {
    let mut operator_table = OperatorPublicKeys {
        0: BTreeMap::new(),
    };

    for i in 1..=3 {
        operator_table.0.insert(i, format!("0xdeadbeef{}", i).to_string());
    }

    operator_table
}

pub async fn get_bridge_operators(rpc_client: &HttpClient) -> Result<OperatorPublicKeys, ClientError> {
    let operator_table = mock_operator_table();

    // Fetch active operator public keys
    // let operator_table: OperatorPublicKeys = match strata_rpc.request("getActiveOperatorChainPubkeySet", ((),)).await {
    //     Ok(data) => data,
    //     Err(e) => {
    //         error!("Bridge status query failed with {}", e);
    //         return Err(e);
    //     }
    // };

    Ok(operator_table)
}

pub async fn get_operator_status(config: &BridgeMonitoringConfig, rpc_client: &HttpClient, operator_pk: &String) -> Result<String, ClientError> {
    // Check if operator responds to an RPC request
    // Explicitly define return type as `bool`
    // let ping_result: Result<bool, ClientError> =
    //     timeout(Duration::from_secs(config.bridge_operator_ping_timeout_s), rpc_client.request("pingOperator", (operator_pk.clone(),)))
    //         .await
    //         .map_err(|_| ClientError::Custom("Timeout".into()))?;

    // let status = if ping_result.is_ok() && ping_result.unwrap() {
    //     "Online".to_string()
    // } else {
    //     "Offline".to_string()
    // };

    // Ok(status)

    Ok("Online".to_string())
}

pub async fn get_current_deposits(rpc_client: &HttpClient) -> Result<Vec<u32>, ClientError> {
    let deposit_ids = vec![1, 2, 3];
    // let deposit_ids: Vec<u32> = match rpc_client.request("getCurrentDeposits", ((),)).await {
    //     Ok(data) => data,
    //     Err(e) => {
    //         error!("Current deposits query failed with {}", e);
    //         return Err(e);
    //     }
    // };

    Ok(deposit_ids)
}

fn mock_deposit_info(deposit_id: u32) -> DepositInfo {
    let deposit_txid = format!("0xaabbccddee{}", deposit_id);

    let deposit_info = DepositInfo {
        deposit_request_txid: format!("0xabcdefgh{}", deposit_id).into(),
        deposit_txid: Some(deposit_txid.into()),
        mint_txid: Some(format!("0xqrstuvwx{}", deposit_id).into()),
        status: "Accepted".to_string(),
    };

    deposit_info
}

pub async fn get_deposit_info(rpc_client: &HttpClient, deposit_id: u32) -> Result<DepositInfo, ClientError> {

    let deposit_info = mock_deposit_info(deposit_id);

    // let deposit_txid: String = match rpc_client.request("getCurrentDepositById", (deposit_id,)).await {
    //     Ok(data) => data,
    //     Err(e) => {
    //         error!("Get deposit by id failed with {}", e);
    //         return Err(e);
    //     }
    // };
;
    // let deposit_info: DepositInfo = match rpc_client.request("getDepositInfo", (deposit_txid,)).await {
    //     Ok(data) => data,
    //     Err(e) => {
    //         error!("Get deposit by id failed with {}", e);
    //         return Err(e);
    //     }
    // };

    Ok(deposit_info)
}

fn mock_withdrawal_info(operator_idx: u32) -> Vec<WithdrawalInfo> {
    let mut withdrawals = Vec::new();
    withdrawals.push(WithdrawalInfo {
        withdrawal_request_txid: format!("0xaabbccddee{}", operator_idx).to_string(),
        fulfillment_txid: Some(format!("0xffcdbade{}", operator_idx).to_string()),
    status: "Accepted".to_string(),
    });

    withdrawals
}

pub async fn get_withdrawal_info(rpc_client: &HttpClient, operator_idx: u32) -> Result<Vec<WithdrawalInfo>, ClientError> {
    let withdrawal_infos = mock_withdrawal_info(operator_idx);

    // let bridge_duties: RpcBridgeDuties = match rpc_client.request("getBridgeDuties", (operator_idx, 0)).await {
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
    //         let wd_info: WithdrawalInfo = match rpc_client.request("getWithdrawalInfo", (deposit_outpoint.clone(), )).await {
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