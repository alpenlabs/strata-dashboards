// use serde_json::json;
// use std::sync::Arc;
// use jsonrpsee::http_client::HttpClient;
// use tokio::time::{interval, Duration};
// use anyhow::Result;
// use log::info;
// use crate::db::DatabaseWrapper;
// use crate::utils::create_rpc_client;

// #[derive(Clone)]
// pub struct IndexerConfig {
//     pub rpc_url: String,
//     pub entrypoint_address: String,   // e.g. "0x..."
//     pub userop_event_topic: String,   // keccak256 of UserOperationEvent signature
//     pub batch_size: u64,             // e.g. 2000
// }

// #[derive(Clone)]
// pub struct AppState {
//     pub config: IndexerConfig,
// }

// pub async fn fetch_user_ops_task(database: DatabaseWrapper, config: &IndexerConfig) -> Result<()> {
//     info!("calling get_ethLogs for UserOperationEvent ...");
//     let mut interval = interval(Duration::from_secs(10));
//     let rpc_client = create_rpc_client(&config.rpc_url);

//     // 1) Get latest block
//     let latest_block = get_latest_block_number(&rpc_client).await?;

//     // 2) Load last processed block
//     let mut from_block = load_last_processed_block(&database).await?.unwrap_or(0);

//     // 3) Loop in batches
//     while from_block <= latest_block {
//         let to_block = std::cmp::min(from_block + config.batch_size, latest_block);

//         // 4) Fetch logs
//         let logs = fetch_logs_in_range(
//             &rpc_client,
//             &config.entrypoint_address,
//             &config.userop_event_topic,
//             from_block,
//             to_block
//         )
//         .await?;

//         // 5) Store them in Postgres
//         // store_logs(&database, &logs).await?;

//         // 6) Update last processed block
//         // save_last_processed_block(&database, to_block).await?;

//         info!("✅ Indexed blocks {} -> {}", from_block, to_block);

//         if to_block == latest_block {
//             break;
//         }
//         from_block = to_block + 1;

//         // Sleep briefly to avoid rate-limit
//         interval.tick().await;
//     }

//     Ok(())
// }

// async fn get_latest_block_number(client: &HttpClient) -> Result<u64> {
//     info!("🔹 Fetching latest block number");

//     let params = [];  // ✅ Use a tuple instead of `serde_json::Value`
//     let response: Result<serde_json::Value, _> = client.request("eth_blockNumber", params).await;
//     match response {
//         Ok(json) => {
//             if let Some(hex_str) = json["result"].as_str() {
//                 block_num = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)?;
//                 return Ok(block_num)
//             }
//         }
//         Err(e) => {
//             info!("🔹 Error fetching latest block number: {}", e);
//         }
//     }
//     None
// }

// async fn load_last_processed_block(database: DatabaseWrapper) -> Result<Option<u64>> {
//     Ok(Some(0))
// }

// async fn fetch_logs_in_range(
//     client: &HttpClient,
//     entrypoint_address: &str,
//     userop_event_topic: &str,
//     from_block: u64,
//     to_block: u64
// ) -> Result<Vec<EthLog>> {
//     // Convert to hex strings
//     let from_hex = format!("0x{:x}", from_block);
//     let to_hex   = format!("0x{:x}", to_block);

//     let filter = serde_json::json!({
//         "fromBlock": from_hex,
//         "toBlock": to_hex,
//         "address": entrypoint_address,
//         "topics": [userop_event_topic]
//     });

//     let body = serde_json::json!({
//         "jsonrpc": "2.0",
//         "id": 1,
//         "method": "eth_getLogs",
//         "params": [filter]
//     });

//     let resp = client.post(rpc_url)
//         .json(&body)
//         .send()
//         .await?
//         .error_for_status()?
//         .json::<JsonRpcResponse>()
//         .await?;
    
//     Ok(resp.result)
// }