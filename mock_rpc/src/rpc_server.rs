use anyhow::Context;
use async_trait::async_trait;
use bitcoin::{OutPoint, Txid, PublicKey};
use jsonrpsee::{RpcModule, types::ErrorObjectOwned, proc_macros::rpc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};
use strata_bridge_rpc::StrataBridgeMonitoringApiServer;
use strata_bridge_rpc::types::{
    RpcOperatorStatus,
    RpcDepositInfo,
    RpcWithdrawalInfo,
    RpcClaimInfo,
};
use strata_bridge_primitives::duties::BridgeDuty;
use strata_bridge_primitives::types::PublickeyTable;
use tokio::sync::oneshot;
use tracing::{info, warn};

/// JSON-RPC result.
pub type RpcResult<T> = std::result::Result<T, jsonrpsee_types::ErrorObjectOwned>;
pub type OperatorIdx = u32;
pub type DepositId = u32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcDepositEntry {
    deposit_idx: u32,

    /// The outpoint that this deposit entry references.
    output: OutPoint,

    /// List of notary operators, by their indexes.
    // TODO convert this to a windowed bitmap or something
    notary_operators: Vec<OperatorIdx>,

    /// Deposit amount, in the native asset.
    amt: u64,

    /// Withdrawal request transaction id
    withdrawal_request_txid: Option<String>,
}

#[rpc(server, namespace = "strata")]
pub trait StrataRpc {
    #[method(name = "getCurrentDeposits")]
    async fn get_current_deposits(&self) -> RpcResult<Vec<u32>>;

    #[method(name = "getCurrentDepositById")]
    async fn get_current_deposit_by_id(&self, deposit_idx: u32) -> RpcResult<RpcDepositEntry>;
}

#[derive(Clone)]
pub struct MockStrataRpc {
    current_deposits: Vec<u32>,
    current_deposit_entries: HashMap<u32, RpcDepositEntry>,
}

impl MockStrataRpc {
    pub fn load_from_files(path: &str) -> Result<Self, ErrorObjectOwned> {
        fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, ErrorObjectOwned> {
            let content = fs::read_to_string(path)
                .map_err(|e| ErrorObjectOwned::owned(-32000, "file read error", Some(e.to_string())))?;
            serde_json::from_str(&content)
                .map_err(|e| ErrorObjectOwned::owned(-32000, "deserialization error", Some(e.to_string())))
        }

        Ok(Self {
            current_deposits: read_json(&format!("{}/current_deposits.json", path))?,
            current_deposit_entries: read_json(&format!("{}/deposit_entries.json", path))?,
        })
    }
}

#[async_trait]
impl StrataRpcServer for MockStrataRpc {
    async fn get_current_deposits(&self) -> RpcResult<Vec<DepositId>> {
        Ok(self.current_deposits.clone())
    }

    async fn get_current_deposit_by_id(&self, id: DepositId) -> RpcResult<RpcDepositEntry> {
        self.current_deposit_entries
            .get(&id)
            .cloned()
            .ok_or_else(|| ErrorObjectOwned::owned(-32000, "not found", Some(id.to_string())))
    }
}

#[derive(Clone)]
pub struct MockBridgeMonitoring {
    pub deposit_infos: HashMap<String, RpcDepositInfo>,
    pub withdrawal_infos: HashMap<String, RpcWithdrawalInfo>,
    pub claim_infos: HashMap<String, RpcClaimInfo>,
    pub duties: Vec<BridgeDuty>,
    pub claim_ids: Vec<Txid>,
    pub pubkeys: PublickeyTable,
    pub operator_statuses: HashMap<u32, RpcOperatorStatus>,
}

impl MockBridgeMonitoring {
    pub fn load_from_files(path: &str) -> Result<Self, ErrorObjectOwned> {
        fn read_json(path: &str, name: &str) -> Result<String, ErrorObjectOwned> {
            fs::read_to_string(format!("{}/{}.json", path, name))
                .map_err(|e| ErrorObjectOwned::owned(-32000, "file read error", Some(e.to_string())))
        }

        fn parse_json<T: serde::de::DeserializeOwned + std::fmt::Debug>(data: &str) -> Result<T, ErrorObjectOwned> {
            serde_json::from_str(data)
                .map_err(|e| ErrorObjectOwned::owned(-32000, "deserialization error", Some(e.to_string())))
        }

        let deposit_infos = parse_json(&read_json(path, "deposit_infos")?)?;
        let withdrawal_infos = parse_json(&read_json(path, "withdrawal_infos")?)?;
        let claim_infos = parse_json(&read_json(path, "claim_infos")?)?;
        let duties = Vec::new();
        let claim_ids = parse_json(&read_json(path, "claims")?)?;
        let pubkeys = parse_json(&read_json(path, "bridge_operators")?)?;
        let operator_statuses = parse_json(&read_json(path, "operator_status")?)?;

        Ok(Self {
            deposit_infos,
            withdrawal_infos,
            claim_infos,
            duties,
            claim_ids,
            pubkeys,
            operator_statuses,
        })
    }
}

#[async_trait]
impl StrataBridgeMonitoringApiServer for MockBridgeMonitoring {
    async fn get_bridge_operators(&self) -> RpcResult<PublickeyTable> {
        Ok(self.pubkeys.clone())
    }

    async fn get_operator_status(&self, operator_idx: OperatorIdx) -> RpcResult<RpcOperatorStatus> {
        self.operator_statuses
            .get(&operator_idx)
            .cloned()
            .ok_or_else(|| ErrorObjectOwned::owned(
                -32000,
                "operator not found",
                Some(format!("operator index: {}", operator_idx)),
            ))
    }

    async fn get_deposit_info(&self, outpoint: OutPoint) -> RpcResult<RpcDepositInfo> {
        let key = format!("{outpoint}");
        self.deposit_infos
            .get(&key)
            .cloned()
            .ok_or_else(|| ErrorObjectOwned::owned(
                -32000,
                "not found",
                Some("deposit not found"),
            ))
    }

    async fn get_withdrawal_info(&self, outpoint: OutPoint) -> RpcResult<RpcWithdrawalInfo> {
        let key = format!("{outpoint}");
        self.withdrawal_infos
            .get(&key)
            .cloned()
            .ok_or_else(|| ErrorObjectOwned::owned(
                -32000,
                "not found",
                Some("withdrawal not found"),
            ))
    }

    async fn get_claim_info(&self, txid: Txid) -> RpcResult<RpcClaimInfo> {
        let key = format!("{txid}");
        self.claim_infos
            .get(&key)
            .cloned()
            .ok_or_else(|| ErrorObjectOwned::owned(
                -32000,
                "not found",
                Some("claim not found"),
            ))
    }

    async fn get_claims(&self) -> RpcResult<Vec<Txid>> {
        Ok(self.claim_ids.clone())
    }

    async fn get_bridge_duties(&self) -> RpcResult<Vec<BridgeDuty>> {
        Ok(self.duties.clone())
    }

    async fn get_bridge_duties_by_operator_id(&self, _operator_idx: u32) -> RpcResult<Vec<BridgeDuty>> {
        Ok(Vec::new())
    }

    async fn get_bridge_duties_by_operator_pk(&self, _operator_pk: PublicKey) -> RpcResult<Vec<BridgeDuty>> {
        Ok(Vec::new())
    }
}

pub(crate) async fn start_rpc_server<C: Send + Sync + 'static>(
    rpc_module: RpcModule<C>,
    rpc_addr: &str,
) -> anyhow::Result<()> {
    let server = jsonrpsee::server::ServerBuilder::default()
        .build(rpc_addr)
        .await
        .context("failed to build RPC server")?;

    let handle = server.start(rpc_module);
    info!(%rpc_addr, "RPC server started");

    let (_stop_tx, stop_rx) = oneshot::channel::<bool>();

    let _ = stop_rx.await;
    info!("stopping RPC server");

    if handle.stop().is_err() {
        warn!("rpc server already stopped");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs};
    use strata_bridge_rpc::types::{RpcDepositInfo, RpcWithdrawalInfo, RpcClaimInfo};
    use serde_json;

    #[test]
    fn test_deserialize_deposit_infos() {
        // Path to your mock JSON file
        let json_path = "mock_data/bridge_rpc/deposit_infos.json";
        let json_str = fs::read_to_string(json_path).expect("read to succeed");

        // Try to deserialize
        let parsed: HashMap<String, RpcDepositInfo> =
            serde_json::from_str(&json_str).expect("deserialize to succeed");

        // Debug output
        for (key, value) in &parsed {
            println!("{} => {:?}", key, value);
        }

        // Simple assertion
        assert!(!parsed.is_empty(), "Expected at least one deposit entry");
    }

    #[test]
    fn test_deserialize_withdrawal_infos() {
        // Path to your mock JSON file
        let json_path = "mock_data/bridge_rpc/withdrawal_infos.json";
        let json_str = fs::read_to_string(json_path).expect("read to read succeed");

        // Try to deserialize
        let parsed: HashMap<String, RpcWithdrawalInfo> =
            serde_json::from_str(&json_str).expect("deserialize to succeed");

        // Debug output
        for (key, value) in &parsed {
            println!("{} => {:?}", key, value);
        }

        // Simple assertion
        assert!(!parsed.is_empty(), "Expected at least one withdrawal entry");
    }

    #[test]
    fn test_deserialize_claim_infos() {
        // Path to your mock JSON file
        let json_path = "mock_data/bridge_rpc/claim_infos.json";
        let json_str = fs::read_to_string(json_path).expect("read to succeed");

        // Try to deserialize
        let parsed: HashMap<String, RpcClaimInfo> =
            serde_json::from_str(&json_str).expect("deserialize to succeed");

        // Debug output
        for (key, value) in &parsed {
            println!("{} => {:?}", key, value);
        }

        // Simple assertion
        assert!(!parsed.is_empty(), "Expected at least one claim entry");
    }
}
