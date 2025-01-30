use clap::Parser;
use dotenvy::dotenv;
use log::info;
use std::env;
/// CLI Args Structc
#[derive(Parser, Debug, Clone)]
#[command(version, about = "Strata Network Monitor")]
pub struct Config {
    /// JSON-RPC Endpoint for Strata client
    #[arg(long)]
    rpc_url: Option<String>,

    /// JSON-RPC Endpoint for Strata reth for wallet balance
    #[arg(long)]
    reth_url: Option<String>,

    /// Bundler health check URL (overrides `.env`)
    #[arg(long)]
    bundler_url: Option<String>,

    /// Deposit paymaster wallet
    #[arg(long)]
    deposit_wallet: Option<String>,

    /// Validating paymaster wallet
    #[arg(long)]
    validating_wallet: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        dotenv().ok(); // Load `.env` file if present

        let cli = Config::parse();

        let rpc_url = cli
            .rpc_url
            .or_else(|| env::var("RPC_URL").ok()) // Load from CLI, then `.env`
            .unwrap_or_else(|| {
                "https://strataclient1ff4bc1df.devnet-annapurna.stratabtc.org".to_string()
            });

        let bundler_url = cli
            .bundler_url
            .or_else(|| env::var("BUNDLER_URL").ok()) // Load from CLI, then `.env`
            .unwrap_or_else(|| "https://bundler.devnet-annapurna.stratabtc.org/hth".to_string());

        let reth_url = cli
            .reth_url
            .or_else(|| env::var("RETH_URL").ok()) // Load from CLI, then `.env`
            .unwrap_or_else(|| "https://reth1ff4bc1df.devnet-annapurna.stratabtc.org".to_string());

        let deposit_wallet = cli
            .deposit_wallet
            .or_else(|| env::var("DEPOSIT_PAYMASTER_WALLET").ok()) // Load from CLI, then `.env`
            .unwrap_or_else(|| "0xCAFE".to_string());

        let validating_wallet = cli
            .validating_wallet
            .or_else(|| env::var("VALIDATING_PAYMASTER_WALLET").ok()) // Load from CLI, then `.env`
            .unwrap_or_else(|| "0xC0FFEE".to_string());

        info!(
            "🔹 Loaded Config: rpc_url = {}, bundler_url = {}",
            rpc_url, bundler_url
        );

        Config {
            rpc_url: Some(rpc_url),
            bundler_url: Some(bundler_url),
            reth_url: Some(reth_url),
            deposit_wallet: Some(deposit_wallet),
            validating_wallet: Some(validating_wallet),
        }
    }

    /// Getter for `rpc_url`
    pub fn rpc_url(&self) -> String {
        self.rpc_url
            .clone()
            .expect("RPC_URL must be provided via CLI or .env")
    }

    /// Getter for `bundler_url`
    pub fn bundler_url(&self) -> String {
        self.bundler_url
            .clone()
            .expect("BUNDLER_URL must be provided via CLI or .env")
    }

    pub fn reth_url(&self) -> String {
        self.reth_url
            .clone()
            .expect("RETH_URL must be provided via CLI or .env")
    }

    pub fn deposit_wallet(&self) -> String {
        self.deposit_wallet
            .clone()
            .expect("DEPOSIT_WALLET must be provided via CLI or .env")
    }

    pub fn validating_wallet(&self) -> String {
        self.validating_wallet
            .clone()
            .expect("VALIDATING_WALLET must be provided via CLI or .env")
    }
}

// pub struct IndexerConfig {
//     /// JSON-RPC Endpoint for Strata reth
//     #[arg(
//         long,
//         env = "RETH_URL",
//         default_value = "https://reth1ff4bc1df.devnet-annapurna.stratabtc.org",
//         help = "Strata reth URL"
//     )]
//     pub rpc_url: String,

//     /// URL of the PostgreSQL database used by indexer
//     #[arg(
//         long,
//         env = "DATABASE_URL",
//         default_value = "postgres://alpen:alpen@123@localhost:5432/usage_stats_db",
//         help = "PostgreSQL database URL",
//     )]
//     pub database_url: String,

//     /// EntryPoint address
//     #[arg(
//         long,
//         env = "ENTRYPOINT_ADDRESS",
//         default_value = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
//         help = "EndPoint address",
//     )]
//     pub entrypoint_address: String,

//     /// Eth logs batch size
//     #[arg(
//         long,
//         env = "ETH_LOGS_BATCH_SIZE",
//         default_value = 1000,
//         help = "Batch size for fetching eth logs",
//     )]
//     pub batch_size: u32,
// }