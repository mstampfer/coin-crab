use serde::{Deserialize, Serialize};
use reqwest::Client;
use rumqttc::AsyncClient;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// Re-export shared types for convenience
pub use shared::{CryptoCurrency, HistoricalDataResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinMarketCapResponse {
    pub data: Vec<CryptoCurrency>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse {
    pub data: Vec<CryptoCurrency>,
    pub last_updated: String,
    pub cached: bool,
}

pub struct AppState {
    pub cache: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
    pub last_fetch: Arc<Mutex<SystemTime>>,
    pub client: Client,
    pub api_key: String,
    pub mqtt_client: Arc<AsyncClient>,
    pub historical_cache: Arc<Mutex<HashMap<String, (HistoricalDataResult, SystemTime)>>>,
    pub update_interval_seconds: u64,
}

#[derive(Deserialize)]
pub struct HistoricalQuery {
    pub timeframe: String,
}