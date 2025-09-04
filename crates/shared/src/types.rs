use serde::{Deserialize, Serialize};

// Shared data structures used by both server and iOS library

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoCurrency {
    pub id: i32,
    pub name: String,
    pub symbol: String,
    pub quote: Quote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    #[serde(rename = "USD")]
    pub usd: UsdQuote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdQuote {
    pub price: f64,
    pub percent_change_1h: f64,
    pub percent_change_24h: f64,
    pub percent_change_7d: f64,
    pub market_cap: f64,
    pub volume_24h: f64,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataPoint {
    pub timestamp: f64,
    pub price: f64,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataResult {
    pub success: bool,
    pub data: Vec<HistoricalDataPoint>,
    pub error: Option<String>,
    pub symbol: Option<String>,
    pub timeframe: Option<String>,
}