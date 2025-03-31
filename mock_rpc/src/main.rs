mod rpc_server;

use strata_bridge_rpc::StrataBridgeMonitoringApiServer;
use tracing_subscriber;

use crate::rpc_server::{
    MockBridgeMonitoring, 
    MockStrataRpc,
    StrataRpcServer,
    start_rpc_server
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let strata_rpc = MockStrataRpc::load_from_files("mock_data/strata_rpc")?;
    let bridge_rpc = MockBridgeMonitoring::load_from_files("mock_data/bridge_rpc")?;

    let strata_addr = "0.0.0.0:8545";
    let bridge_addr = "0.0.0.0:8546";

    let bridge_module = StrataBridgeMonitoringApiServer::into_rpc(bridge_rpc);
    let strata_module = StrataRpcServer::into_rpc(strata_rpc);

    tokio::try_join!(
        start_rpc_server(strata_module, strata_addr),
        start_rpc_server(bridge_module, bridge_addr),
    )?;

    Ok(())
}
