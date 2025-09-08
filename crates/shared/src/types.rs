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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_usd_quote() -> UsdQuote {
        UsdQuote {
            price: 50000.0,
            percent_change_1h: 0.5,
            percent_change_24h: 2.5,
            percent_change_7d: 10.0,
            market_cap: 900000000000.0,
            volume_24h: 50000000000.0,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn create_test_crypto() -> CryptoCurrency {
        CryptoCurrency {
            id: 1,
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            quote: Quote {
                usd: create_test_usd_quote(),
            },
        }
    }

    #[test]
    fn test_crypto_currency_creation() {
        let crypto = create_test_crypto();
        
        assert_eq!(crypto.id, 1);
        assert_eq!(crypto.name, "Bitcoin");
        assert_eq!(crypto.symbol, "BTC");
        assert_eq!(crypto.quote.usd.price, 50000.0);
        assert_eq!(crypto.quote.usd.percent_change_24h, 2.5);
    }

    #[test]
    fn test_crypto_currency_serialization() {
        let crypto = create_test_crypto();
        let json = serde_json::to_string(&crypto).unwrap();
        
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"name\":\"Bitcoin\""));
        assert!(json.contains("\"symbol\":\"BTC\""));
        assert!(json.contains("\"price\":50000.0"));
        assert!(json.contains("\"USD\""));
    }

    #[test]
    fn test_crypto_currency_deserialization() {
        let json = r#"{
            "id": 1,
            "name": "Bitcoin",
            "symbol": "BTC",
            "quote": {
                "USD": {
                    "price": 50000.0,
                    "percent_change_1h": 0.5,
                    "percent_change_24h": 2.5,
                    "percent_change_7d": 10.0,
                    "market_cap": 900000000000.0,
                    "volume_24h": 50000000000.0,
                    "last_updated": "2024-01-01T00:00:00Z"
                }
            }
        }"#;
        
        let crypto: CryptoCurrency = serde_json::from_str(json).unwrap();
        
        assert_eq!(crypto.id, 1);
        assert_eq!(crypto.name, "Bitcoin");
        assert_eq!(crypto.symbol, "BTC");
        assert_eq!(crypto.quote.usd.price, 50000.0);
        assert_eq!(crypto.quote.usd.percent_change_24h, 2.5);
        assert_eq!(crypto.quote.usd.last_updated, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_usd_quote_all_fields() {
        let usd_quote = create_test_usd_quote();
        
        assert_eq!(usd_quote.price, 50000.0);
        assert_eq!(usd_quote.percent_change_1h, 0.5);
        assert_eq!(usd_quote.percent_change_24h, 2.5);
        assert_eq!(usd_quote.percent_change_7d, 10.0);
        assert_eq!(usd_quote.market_cap, 900000000000.0);
        assert_eq!(usd_quote.volume_24h, 50000000000.0);
        assert_eq!(usd_quote.last_updated, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_historical_data_point_creation() {
        let point = HistoricalDataPoint {
            timestamp: 1704067200.0,
            price: 45000.0,
            volume: Some(1000000000.0),
        };
        
        assert_eq!(point.timestamp, 1704067200.0);
        assert_eq!(point.price, 45000.0);
        assert_eq!(point.volume, Some(1000000000.0));
    }

    #[test]
    fn test_historical_data_point_no_volume() {
        let point = HistoricalDataPoint {
            timestamp: 1704067200.0,
            price: 45000.0,
            volume: None,
        };
        
        assert_eq!(point.timestamp, 1704067200.0);
        assert_eq!(point.price, 45000.0);
        assert!(point.volume.is_none());
    }

    #[test]
    fn test_historical_data_result_success() {
        let point = HistoricalDataPoint {
            timestamp: 1704067200.0,
            price: 45000.0,
            volume: Some(1000000000.0),
        };
        
        let result = HistoricalDataResult {
            success: true,
            data: vec![point],
            error: None,
            symbol: Some("BTC".to_string()),
            timeframe: Some("24h".to_string()),
        };
        
        assert!(result.success);
        assert_eq!(result.data.len(), 1);
        assert!(result.error.is_none());
        assert_eq!(result.symbol, Some("BTC".to_string()));
        assert_eq!(result.timeframe, Some("24h".to_string()));
    }

    #[test]
    fn test_historical_data_result_error() {
        let result = HistoricalDataResult {
            success: false,
            data: vec![],
            error: Some("API rate limit exceeded".to_string()),
            symbol: Some("BTC".to_string()),
            timeframe: Some("24h".to_string()),
        };
        
        assert!(!result.success);
        assert_eq!(result.data.len(), 0);
        assert_eq!(result.error, Some("API rate limit exceeded".to_string()));
        assert_eq!(result.symbol, Some("BTC".to_string()));
        assert_eq!(result.timeframe, Some("24h".to_string()));
    }

    #[test]
    fn test_historical_data_serialization() {
        let point = HistoricalDataPoint {
            timestamp: 1704067200.0,
            price: 45000.0,
            volume: Some(1000000000.0),
        };
        
        let result = HistoricalDataResult {
            success: true,
            data: vec![point],
            error: None,
            symbol: Some("BTC".to_string()),
            timeframe: Some("24h".to_string()),
        };
        
        let json = serde_json::to_string(&result).unwrap();
        
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"timestamp\":1704067200.0"));
        assert!(json.contains("\"price\":45000.0"));
        assert!(json.contains("\"symbol\":\"BTC\""));
        assert!(json.contains("\"timeframe\":\"24h\""));
    }

    #[test]
    fn test_types_are_cloneable() {
        let crypto = create_test_crypto();
        let _crypto_clone = crypto.clone();
        
        let point = HistoricalDataPoint {
            timestamp: 1704067200.0,
            price: 45000.0,
            volume: Some(1000000000.0),
        };
        let _point_clone = point.clone();
        
        let result = HistoricalDataResult {
            success: true,
            data: vec![point],
            error: None,
            symbol: Some("BTC".to_string()),
            timeframe: Some("24h".to_string()),
        };
        let _result_clone = result.clone();
        
        assert!(true); // If we reach here, all types are cloneable
    }

    #[test]
    fn test_types_are_debuggable() {
        let crypto = create_test_crypto();
        let debug_str = format!("{:?}", crypto);
        assert!(debug_str.contains("CryptoCurrency"));
        assert!(debug_str.contains("Bitcoin"));
        
        let point = HistoricalDataPoint {
            timestamp: 1704067200.0,
            price: 45000.0,
            volume: Some(1000000000.0),
        };
        let debug_str = format!("{:?}", point);
        assert!(debug_str.contains("HistoricalDataPoint"));
        assert!(debug_str.contains("45000.0"));
    }

    #[test]
    fn test_multiple_currencies() {
        let btc = CryptoCurrency {
            id: 1,
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            quote: Quote { usd: create_test_usd_quote() },
        };
        
        let eth = CryptoCurrency {
            id: 2,
            name: "Ethereum".to_string(),
            symbol: "ETH".to_string(),
            quote: Quote {
                usd: UsdQuote {
                    price: 3000.0,
                    percent_change_1h: 0.3,
                    percent_change_24h: 1.5,
                    percent_change_7d: 5.0,
                    market_cap: 350000000000.0,
                    volume_24h: 25000000000.0,
                    last_updated: "2024-01-01T00:00:00Z".to_string(),
                },
            },
        };
        
        let currencies = vec![btc, eth];
        assert_eq!(currencies.len(), 2);
        assert_eq!(currencies[0].symbol, "BTC");
        assert_eq!(currencies[1].symbol, "ETH");
        assert_eq!(currencies[1].quote.usd.price, 3000.0);
    }
}