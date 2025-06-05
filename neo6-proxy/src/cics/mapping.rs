// COBOL ↔ JSON mapping

use neo6_protocols_lib::protocol::TransactionConfig;
use std::collections::HashMap;
pub type TransactionMap = HashMap<String, TransactionConfig>;

// #[derive(Debug, Deserialize)]
// pub type TransactionMap = std::collections::HashMap<String, neo6_protocols_lib::protocol::TransactionConfig>;

pub fn load_transaction_map(path: &str) -> Result<TransactionMap, Box<dyn std::error::Error>> {
    use std::fs::File;
    use serde_yaml;
    let file = File::open(path)?;
    let map: TransactionMap = serde_yaml::from_reader(file)?;
    Ok(map)
}