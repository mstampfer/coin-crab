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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub cmc_mapping: Arc<Mutex<HashMap<String, u32>>>,
    pub logo_cache: Arc<Mutex<HashMap<String, (Vec<u8>, SystemTime)>>>,
}

#[derive(Deserialize)]
pub struct HistoricalQuery {
    pub timeframe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmcCurrency {
    pub id: u32,
    pub name: String,
    pub symbol: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmcMappingResponse {
    pub status: CmcStatus,
    pub data: Vec<CmcCurrency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmcStatus {
    pub timestamp: String,
    pub error_code: u32,
    pub error_message: Option<String>,
    pub elapsed: u32,
    pub credit_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{Quote, UsdQuote};

    #[test]
    fn test_coinmarketcap_response_deserialization() {
        let json = r#"{
            "data": [
                {
                    "id": 1,
                    "name": "Bitcoin",
                    "symbol": "BTC",
                    "quote": {
                        "USD": {
                            "price": 50000.0,
                            "market_cap": 900000000000.0,
                            "percent_change_1h": 0.5,
                            "percent_change_24h": 2.5,
                            "percent_change_7d": 10.0,
                            "volume_24h": 50000000000.0,
                            "last_updated": "2024-01-01T00:00:00Z"
                        }
                    }
                }
            ]
        }"#;

        let response: CoinMarketCapResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].symbol, "BTC");
        assert_eq!(response.data[0].quote.usd.price, 50000.0);
    }

    #[test]
    fn test_api_response_serialization() {
        let crypto = CryptoCurrency {
            id: 1,
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            quote: Quote {
                usd: UsdQuote {
                    price: 50000.0,
                    market_cap: 900000000000.0,
                    percent_change_1h: 0.5,
                    percent_change_24h: 2.5,
                    percent_change_7d: 10.0,
                    volume_24h: 50000000000.0,
                    last_updated: "2024-01-01T00:00:00Z".to_string(),
                },
            },
        };

        let api_response = ApiResponse {
            data: vec![crypto],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            cached: false,
        };

        let json = serde_json::to_string(&api_response).unwrap();
        assert!(json.contains("Bitcoin"));
        assert!(json.contains("50000"));
        assert!(json.contains("cached"));
    }

    #[test]
    fn test_historical_query_deserialization() {
        let json = r#"{"timeframe": "24h"}"#;
        let query: HistoricalQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.timeframe, "24h");
    }
}