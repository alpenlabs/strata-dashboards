use axum::{
    routing::get,
    Json, Router,
};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};
use tokio::{sync::Mutex, time::{interval, Duration}};
use tower_http::cors::{Any, CorsLayer};
use tokio::net::TcpListener;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::core::client::ClientT;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
enum Status {
    Online,
    Offline,
}

#[derive(Serialize, Clone, Debug)]
struct NetworkStatus {
    batch_producer: Status,
    rpc_endpoint: Status,
    bundler_endpoint: Status,
}

// Shared State
type SharedState = Arc<Mutex<NetworkStatus>>;

/// Creates a JSON-RPC client
fn create_rpc_client() -> HttpClient {
    HttpClientBuilder::default()
        .build("https://strataclient1ff4bc1df.devnet-annapurna.stratabtc.org")
        .unwrap()
}

/// Calls `strata_syncStatus` using `jsonrpsee`
async fn call_rpc_status(client: &HttpClient) -> Status {
    let response: Result<serde_json::Value, _> = client.request("strata_syncStatus", Vec::<()>::new()).await;
    
    match response {
        Ok(json) => {
            println!("🔹 RPC Response: {:?}", json);
            if json.get("tip_height").is_some() {
                Status::Online
            } else {
                Status::Offline
            }
        }
        Err(e) => {
            println!("🔹 error: {}", e);
            Status::Offline
        }
    }
}

/// Checks bundler health (`/health`)
async fn check_bundler_health(client: &reqwest::Client) -> Status {
    let url = "https://bundler.devnet-annapurna.stratabtc.org/health";
    if let Ok(resp) = client.get(url).send().await {
        let body = resp.text().await.unwrap_or_default();
        if body.contains("ok") {
            return Status::Online;
        }
    }
    Status::Offline
}

/// Periodically fetches real statuses
async fn fetch_real_statuses(state: SharedState) {
    let mut interval = interval(Duration::from_secs(10));
    let rpc_client = create_rpc_client();
    let http_client = reqwest::Client::new();

    loop {
        interval.tick().await;

        let batch_producer = call_rpc_status(&rpc_client).await;
        let rpc_endpoint = call_rpc_status(&rpc_client).await;
        let bundler_endpoint = check_bundler_health(&http_client).await;

        let new_status = NetworkStatus {
            batch_producer,
            rpc_endpoint,
            bundler_endpoint,
        };

        println!("✅ Updated Status: {:?}", new_status);

        let mut locked_state = state.lock().await;
        *locked_state = new_status;
    }
}

/// Handler to get the current network status
async fn get_network_status(state: SharedState) -> Json<NetworkStatus> {
    let data = state.lock().await.clone();
    Json(data)
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new().allow_origin(Any);

    // 🔹 Shared state for network status
    let shared_state = Arc::new(Mutex::new(NetworkStatus {
        batch_producer: Status::Offline, // Default state
        rpc_endpoint: Status::Offline,
        bundler_endpoint: Status::Offline,
    }));

    // 🔹 Spawn a background task to fetch real statuses
    let state_clone = Arc::clone(&shared_state);
    tokio::spawn(async move {
        fetch_real_statuses(state_clone).await;
    });

    let app = Router::new()
        .route("/api/status", get(move || get_network_status(Arc::clone(&shared_state))))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Server running at http://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}