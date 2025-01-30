
// #[derive(Debug, Default)]
// pub struct TimeWindowStats {
//     pub user_ops: u64,        // count of all user ops
//     pub gas_used: u64,        // total gas used
//     pub unique_accounts: u64, // active accounts
//     pub gas_consumers: Vec<String>, // top consumers of gas
// }

// #[derive(Debug, Default)]
// pub struct RecentUsageStats {
//     pub recent_contracts: Vec<String>,
// }

// #[derive(Serialize, Deserialize)]
// struct UsageStats {
//     stats: HashMap<String, TimeWindowStats>,
//     // e.g., "24h" => TimeWindowStats
//     recent_contracts: Vec<String>,
// }

// impl UsageStats {
//     pub fn new(
//         stats: HashMap<String, TimeWindowStats>,
//         recent_contracts: Vec<String>,
//     ) -> Self {
//         Self {
//             stats,
//             recent_contracts,
//         }
//     }
// }