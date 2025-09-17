use serde::{Deserialize, Serialize};

// Re-export shared types for convenience
pub use shared::{CryptoCurrency, HistoricalDataResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub data: Vec<CryptoCurrency>,
    pub last_updated: String,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CryptoClientResult {
    pub success: bool,
    pub data: Option<Vec<CryptoCurrency>>,
    pub error: Option<String>,
    pub last_updated: Option<String>,
    pub cached: bool,
}