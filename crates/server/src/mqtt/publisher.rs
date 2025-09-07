use rumqttc::{AsyncClient, QoS};
use log::{info, warn, error};
use crate::types::CryptoCurrency;
use shared::HistoricalDataResult;

pub async fn publish_crypto_data_to_mqtt(mqtt_client: &AsyncClient, crypto_data: &[CryptoCurrency]) {
    // Publish all crypto data to main topic with retention
    let payload = match serde_json::to_string(crypto_data) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize crypto data for MQTT: {}", e);
            return;
        }
    };
    
    if let Err(e) = mqtt_client.publish("crypto/prices/latest", QoS::AtLeastOnce, true, payload).await {
        error!("Failed to publish to crypto/prices/latest: {}", e);
    } else {
        info!("Published {} cryptocurrencies to MQTT topic crypto/prices/latest", crypto_data.len());
    }
    
    // Publish individual symbol updates
    for crypto in crypto_data {
        let individual_payload = match serde_json::to_string(crypto) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize {} for MQTT: {}", crypto.symbol, e);
                continue;
            }
        };
        
        let topic = format!("crypto/prices/{}", crypto.symbol);
        if let Err(e) = mqtt_client.publish(topic, QoS::AtLeastOnce, true, individual_payload).await {
            error!("Failed to publish to crypto/prices/{}: {}", crypto.symbol, e);
        }
    }
}

pub async fn publish_historical_data_to_mqtt(
    mqtt_client: &AsyncClient, 
    symbol: &str, 
    timeframe: &str, 
    data: &HistoricalDataResult
) {
    let payload = match serde_json::to_string(data) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize historical data for MQTT: {}", e);
            return;
        }
    };
    
    let topic = format!("crypto/historical/{}/{}", symbol.to_uppercase(), timeframe);
    
    // Use QoS 0 for historical data (less critical than live prices)
    // Set retain=true so clients get immediate data when subscribing
    if let Err(e) = mqtt_client.publish(&topic, QoS::AtMostOnce, true, payload).await {
        error!("Failed to publish historical data to {}: {}", topic, e);
    } else {
        info!("Published historical data for {} {} to MQTT", symbol, timeframe);
    }
}

pub async fn publish_empty_retained_message(mqtt_client: &AsyncClient, topic: &str) {
    match mqtt_client.publish(topic, rumqttc::QoS::AtLeastOnce, true, "").await {
        Ok(_) => info!("Cleared MQTT retained message for topic: {}", topic),
        Err(e) => warn!("Failed to clear MQTT retained message for {}: {}", topic, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{Quote, UsdQuote, HistoricalDataPoint};

    fn create_test_crypto() -> CryptoCurrency {
        CryptoCurrency {
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
        }
    }

    #[test]
    fn test_crypto_data_serialization() {
        let crypto = create_test_crypto();
        let json = serde_json::to_string(&crypto).unwrap();
        
        assert!(json.contains("\"symbol\":\"BTC\""));
        assert!(json.contains("\"price\":50000.0"));
        assert!(json.contains("\"market_cap\":900000000000.0"));
    }

    #[test]
    fn test_historical_data_serialization() {
        let historical_data = HistoricalDataResult {
            success: true,
            symbol: Some("BTC".to_string()),
            timeframe: Some("24h".to_string()),
            data: vec![
                HistoricalDataPoint {
                    timestamp: 1704067200.0, // Unix timestamp for 2024-01-01T00:00:00Z
                    price: 45000.0,
                    volume: Some(1000000000.0),
                },
                HistoricalDataPoint {
                    timestamp: 1704070800.0, // Unix timestamp for 2024-01-01T01:00:00Z
                    price: 45500.0,
                    volume: Some(1100000000.0),
                },
            ],
            error: None,
        };

        let json = serde_json::to_string(&historical_data).unwrap();
        
        assert!(json.contains("\"symbol\":\"BTC\""));
        assert!(json.contains("\"timeframe\":\"24h\""));
        assert!(json.contains("45000.0"));
        assert!(json.contains("45500.0"));
    }

    #[test]
    fn test_mqtt_topic_formatting() {
        let symbol = "BTC";
        let timeframe = "24h";
        
        let topic = format!("crypto/historical/{}/{}", symbol.to_uppercase(), timeframe);
        assert_eq!(topic, "crypto/historical/BTC/24h");
        
        let price_topic = format!("crypto/prices/{}", symbol);
        assert_eq!(price_topic, "crypto/prices/BTC");
    }

    #[test]
    fn test_multiple_crypto_serialization() {
        let cryptos = vec![
            create_test_crypto(),
            CryptoCurrency {
                id: 2,
                name: "Ethereum".to_string(),
                symbol: "ETH".to_string(),
                quote: Quote {
                    usd: UsdQuote {
                        price: 3000.0,
                        market_cap: 350000000000.0,
                        percent_change_1h: 0.3,
                        percent_change_24h: 1.5,
                        percent_change_7d: 5.0,
                        volume_24h: 25000000000.0,
                        last_updated: "2024-01-01T00:00:00Z".to_string(),
                    },
                },
            },
        ];

        let json = serde_json::to_string(&cryptos).unwrap();
        
        assert!(json.contains("Bitcoin"));
        assert!(json.contains("Ethereum"));
        assert!(json.contains("BTC"));
        assert!(json.contains("ETH"));
        assert!(json.contains("50000.0"));
        assert!(json.contains("3000.0"));
    }
}