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