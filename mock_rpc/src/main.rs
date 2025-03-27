mod rpc_server;

use strata_bridge_rpc::StrataBridgeMonitoringApiServer;
use crate::rpc_server::StrataRpcServer;
use rpc_server::{MockBridgeMonitoring, MockStrataRpc, start_rpc_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ✅ Initialize logger with info level
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let strata_rpc = MockStrataRpc::load_from_files("mock_data/strata_rpc")?;
    let bridge_rpc = MockBridgeMonitoring::load_from_files("mock_data/bridge_rpc")?;

    let strata_addr = "127.0.0.1:8545";
    let bridge_addr = "127.0.0.1:8546";

    let bridge_module = StrataBridgeMonitoringApiServer::into_rpc(bridge_rpc);
    let strata_module = StrataRpcServer::into_rpc(strata_rpc);

    tokio::try_join!(
        start_rpc_server(strata_module, strata_addr),
        start_rpc_server(bridge_module, bridge_addr),
    )?;

    Ok(())
}
