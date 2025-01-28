use clap::Parser;
use dotenvy::dotenv;
use std::env;

/// CLI Args Struct
#[derive(Parser, Debug)]
#[command(version, about = "Strata Network Monitor")]
pub struct Config {
    /// JSON-RPC Endpoint for Strata Sync Status (overrides `.env`)
    #[arg(long)]
    rpc_url: Option<String>,

    /// Bundler health check URL (overrides `.env`)
    #[arg(long)]
    bundler_url: Option<String>,
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

        println!(
            "🔹 Loaded Config: rpc_url = {}, bundler_url = {}",
            rpc_url, bundler_url
        );

        Config {
            rpc_url: Some(rpc_url),
            bundler_url: Some(bundler_url),
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
}
