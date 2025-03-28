use axum::Json;
use bitcoin::{Txid, OutPoint, secp256k1::PublicKey};
use dotenvy::dotenv;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::ClientError;
use jsonrpsee::http_client::HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, sync::Arc, str::FromStr};
use strata_bridge_primitives::types::PublickeyTable;
use strata_bridge_rpc::types::{
    RpcOperatorStatus,
    RpcDepositInfo,
    RpcDepositStatus,
    RpcWithdrawalInfo,
    RpcWithdrawalStatus,
    RpcClaimInfo,
    RpcReimbursementStatus,
};
use tokio::{sync::Mutex, time::{Duration, interval}};
use tracing::{info, error, warn};

use crate::utils::create_rpc_client;

/// Default bridge status refetch interval
const DEFAULT_BRIDGE_STATUS_REFETCH_INTERVAL_S: u64 = 120_000;

pub struct BridgeMonitoringConfig {
    strata_rpc_url: String,
    bridge_rpc_url: String,
    stats_refetch_interval_s: u64,
}

impl BridgeMonitoringConfig {
    pub fn new() -> Self {
        dotenv().ok(); // Load `.env` file if present

        let strata_rpc_url = env::var("STRATA_RPC_URL").ok()
            .unwrap_or_else(|| {
                "http://0.0.0.1:8545".to_string()
            });

        let bridge_rpc_url = env::var("STRATA_BRIDGE_RPC_URL").ok()
            .unwrap_or_else(|| "http://0.0.0.1:8546".to_string());

        let refresh_interval_s: u64 = std::env::var("BRIDGE_STATUS_REFETCH_INTERVAL_S")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_BRIDGE_STATUS_REFETCH_INTERVAL_S);

        info!(%strata_rpc_url, %bridge_rpc_url, "Bridge monitoring configuration");

        BridgeMonitoringConfig {
            strata_rpc_url,
            bridge_rpc_url,
            stats_refetch_interval_s: refresh_interval_s,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OperatorStatus {
    operator_id: String,
    operator_address: PublicKey,
    status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DepositInfo {
    pub deposit_request_txid: Txid,
    pub deposit_txid: Option<Txid>,
    pub status: String,
}

impl From<RpcDepositInfo> for DepositInfo {
    fn from(rpc_info: RpcDepositInfo) -> Self {
        match rpc_info.status {
            RpcDepositStatus::InProgress { deposit_request_txid } => DepositInfo {
                deposit_request_txid: deposit_request_txid,
                deposit_txid: None,
                status: "In progress".to_string(),
            },
            RpcDepositStatus::Failed { 
                deposit_request_txid, 
                failure_reason: _
            } => DepositInfo {
                deposit_request_txid: deposit_request_txid,
                deposit_txid: None,
                status: "Failed".to_string(),
            },
            RpcDepositStatus::Complete { 
                deposit_request_txid, 
                deposit_txid 
            } => DepositInfo {
                deposit_request_txid: deposit_request_txid,
                deposit_txid: Some(deposit_txid),
                status: "Complete".to_string(),
            },
        }
    }
}

#[derive(Debug)]
struct DepositToWithdrawal {
    deposit_outpoint: OutPoint,
    withdrawal_request_txid: Option<Txid>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WithdrawalInfo {
    pub withdrawal_request_txid: Txid,
    pub fulfillment_txid: Option<Txid>,
    pub status: String,
}

impl WithdrawalInfo {
    pub fn from_rpc(
        rpc_info: &RpcWithdrawalInfo, 
        withdrawal_request_txid: Txid
    ) -> Self {
        match &rpc_info.status {
            RpcWithdrawalStatus::InProgress => Self {
                withdrawal_request_txid,
                fulfillment_txid: None,
                status: "In progress".to_string(),
            },
            RpcWithdrawalStatus::Complete { fulfillment_txid } => Self {
                withdrawal_request_txid,
                fulfillment_txid: Some(fulfillment_txid.clone()),
                status: "Complete".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReimbursementInfo {
    pub claim_txid: Txid,
    pub challenge_step: String,
    pub payout_txid: Option<Txid>,
    pub status: String,
}

impl From<&RpcClaimInfo> for ReimbursementInfo {
    fn from(rpc_info: &RpcClaimInfo) -> Self {
        match &rpc_info.status {
            RpcReimbursementStatus::InProgress { challenge_step } => Self {
                claim_txid: rpc_info.claim_txid,
                challenge_step: format!("{:?}", challenge_step),
                payout_txid: None,
                status: "In progress".to_string(),
            },
            RpcReimbursementStatus::Challenged { challenge_step } => Self {
                claim_txid: rpc_info.claim_txid,
                challenge_step: format!("{:?}", challenge_step),
                payout_txid: None,
                status: "Challenged".to_string(),
            },
            RpcReimbursementStatus::Cancelled => Self {
                claim_txid: rpc_info.claim_txid,
                challenge_step: "N/A".to_string(),
                payout_txid: None,
                status: "Cancelled".to_string(),
            },
            RpcReimbursementStatus::Complete { payout_txid } => Self {
                claim_txid: rpc_info.claim_txid,
                challenge_step: "N/A".to_string(),
                payout_txid: Some(payout_txid.clone()),
                status: "Complete".to_string(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BridgeStatus {
    operators: Vec<OperatorStatus>,
    deposits: Vec<DepositInfo>,
    withdrawals: Vec<WithdrawalInfo>,
    reimbursements: Vec<ReimbursementInfo>,
}

// Shared usage stats
pub type SharedBridgeState = Arc<Mutex<BridgeStatus>>;

/// Periodically fetch bridge status and update shared bridge state
pub async fn bridge_monitoring_task(state: SharedBridgeState, config: &BridgeMonitoringConfig) {
    let mut interval = interval(Duration::from_secs(config.stats_refetch_interval_s));
    let strata_rpc = create_rpc_client(&config.strata_rpc_url);
    let bridge_rpc = create_rpc_client(&config.bridge_rpc_url);

    loop {
        interval.tick().await;
        let mut locked_state = state.lock().await;

        // Bridge operator status
        let operators = get_bridge_operators(&bridge_rpc).await.unwrap();
        let mut operator_statuses = Vec::new();
        for (index, public_key) in operators.0.iter() {
            let operator_id = format!("Alpen Labs #{}", index);
            let status = get_operator_status(&bridge_rpc, *index).await.unwrap();

            operator_statuses.push(OperatorStatus {
                operator_id,
                operator_address: *public_key,
                status,
            });
        }

        locked_state.operators = operator_statuses;

        // Current deposits
        let current_deposits = get_current_deposits(&strata_rpc).await.unwrap();
        // Deposits with withdrawal requests
        let mut deposits_to_withdrawals: Vec<DepositToWithdrawal> = Vec::new();
    
        for deposit_id in current_deposits {
            let (deposit_info, deposit_to_wd) = get_deposit_info(&strata_rpc, &bridge_rpc, deposit_id).await.unwrap();
            if deposit_info.is_some() {
                locked_state.deposits.push(deposit_info.unwrap());
                deposits_to_withdrawals.push(deposit_to_wd.unwrap());
            } else {
                warn!(%deposit_id, "Missing deposit entry for id");
            }
        }

        // Check if deposit_info has withdrawal request txid
        let mut withdrawal_infos: Vec<WithdrawalInfo> = match get_withdrawals(&bridge_rpc, deposits_to_withdrawals).await {
            Ok(data) => data,
            Err(e) => {
                error!(error = %e, "Bridge get withdrawal failed");
                Vec::new()
            }
        };
        locked_state.withdrawals.append(& mut withdrawal_infos);

        // Reimbursements        
        let reimbursements: Vec<ReimbursementInfo> = match get_reimbursements(&bridge_rpc).await {
            Ok(data) => data,
            Err(e) => {
                error!(error = %e, "Bridge get reimbursement failed");
                Vec::new()
            }
        };
        locked_state.reimbursements = reimbursements;
    }
}

async fn get_bridge_operators(rpc_client: &HttpClient) -> Result<PublickeyTable, ClientError> {
    // Fetch active operator public keys
    let operator_table: PublickeyTable = match rpc_client.request("stratabridge_bridgeOperators", ((),)).await {
        Ok(data) => data,
        Err(e) => {
            error!(error = %e, "Bridge operators query failed");
            return Err(e);
        }
    };

    Ok(operator_table)
}

async fn get_operator_status(bridge_client: &HttpClient, operator_idx: u32) -> Result<String, ClientError> {
    let status: RpcOperatorStatus = match bridge_client.request("stratabridge_operatorStatus", (operator_idx,)).await {
        Ok(data) => data,
        Err(e) => {
            error!(error = %e, "Bridge operator status query");
            return Err(e);
        }
    };

    Ok(format!("{:?}", status))
}

async fn get_current_deposits(strata_client: &HttpClient) -> Result<Vec<u32>, ClientError> {
    let deposit_ids: Vec<u32> = match strata_client.request("strata_getCurrentDeposits", ((),)).await {
        Ok(data) => data,
        Err(e) => {
            error!(error = %e, "Current deposits query failed");
            return Err(e);
        }
    };

    Ok(deposit_ids)
}

async fn get_deposit_info(strata_rpc: &HttpClient, bridge_rpc: &HttpClient, deposit_id: u32) -> Result<(Option<DepositInfo>, Option<DepositToWithdrawal>), ClientError> {

    let response: Value = match strata_rpc
        .request("strata_getCurrentDepositById", (deposit_id,))
        .await
    {
        Ok(resp) => {
            info!(?resp, "deposit entry");
            resp
        }
        Err(e) => {
            warn!(%deposit_id, %e, "Skipping deposit id due to RPC error");
            return Ok((None, None));
        }
    };

    // Extract output (deposit_outpoint)
    let deposit_outpoint: Option<OutPoint> = response
        .get("output")
        .and_then(|v| v.as_str())
        .and_then(|s| OutPoint::from_str(s).ok());
    // Extract withdrawal_request_txid
    let withdrawal_request_txid: Option<Txid> = response
        .get("withdrawal_request_txid")
        .and_then(|v| v.as_str())
        .and_then(|s| Txid::from_str(s).ok());

    // Let caller decide what to do
    if deposit_outpoint.is_none() {
        return Ok((None, None));
    }

    let deposit_to_withdrawal = DepositToWithdrawal {
        deposit_outpoint: deposit_outpoint.clone().unwrap(),
        withdrawal_request_txid,
    };

    let deposit_info: RpcDepositInfo = match bridge_rpc.request("stratabridge_depositInfo", (deposit_outpoint,)).await {
        Ok(data) => data,
        Err(e) => {
            error!(error = %e, "Get deposit info failed");
            return Err(e);
        }
    };

    Ok((Some(DepositInfo::from(deposit_info)), Some(deposit_to_withdrawal)))
}

async fn get_withdrawals(bridge_rpc: &HttpClient, deposit_to_withdrawals: Vec<DepositToWithdrawal>) -> Result<Vec<WithdrawalInfo>, ClientError> {
    let mut withdrawal_infos = Vec::new();
    for deposit_to_wd in deposit_to_withdrawals.iter() {
        if deposit_to_wd.withdrawal_request_txid.is_none() {
            continue;
        }

        let wd_info: RpcWithdrawalInfo = match bridge_rpc
            .request("stratabridge_withdrawalInfo", (deposit_to_wd.deposit_outpoint.clone(), )).await {
            Ok(data) => data,
            Err(e) => {
                error!(error = %e, "Get withdrawal info failed");
                return Err(e);
            }
        };

        withdrawal_infos.push(WithdrawalInfo::from_rpc(&wd_info,
            deposit_to_wd.withdrawal_request_txid.clone().unwrap()));
    }

    Ok(withdrawal_infos)
}

async fn get_reimbursements(bridge_rpc: &HttpClient) -> Result<Vec<ReimbursementInfo>, ClientError> {
    let claim_txids: Vec<String> = match bridge_rpc.request("stratabridge_claims", ((),)).await {
        Ok(data) => data,
        Err(e) => {
            error!(error = %e, "Get claims failed");
            return Err(e);
        }
    };

    let mut reimbursement_infos = Vec::new();
    for txid in claim_txids.iter() {
        let reimb_info: RpcClaimInfo = match bridge_rpc.request("stratabridge_claimInfo", (txid.clone(), )).await {
            Ok(data) => data,
            Err(e) => {
                error!(error = %e, "Get claim info failed");
                return Err(e);
            }
        };

        reimbursement_infos.push(ReimbursementInfo::from(&reimb_info));
    }

    Ok(reimbursement_infos)
}

pub async fn get_bridge_status(state: SharedBridgeState) -> Json<BridgeStatus> {
    let data = state.lock().await.clone();
    Json(data)
}
