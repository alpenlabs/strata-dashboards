// use sea_orm::{Database, DatabaseConnection};

// pub struct DatabaseWrapper {
//     pub db: DatabaseConnection,
// }

// impl DatabaseWrapper {
//     /// Create a new database wrapper with the given database URL
//     pub async fn new(database_url: &str) -> Self {
//         let db = Database::connect(database_url)
//             .await
//             .expect("Failed to connect to PostgreSQL {database_url}");
//         Self { db }
//     }
// }