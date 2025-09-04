use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use rumqttc::Publish;
use log::info;

use crate::types::{CryptoCurrency, HistoricalDataResult};
use shared::debug_log;

pub struct MessageHandler {
    latest_prices: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
    historical_data: Arc<Mutex<HashMap<String, HistoricalDataResult>>>,
}

impl MessageHandler {
    pub fn new(
        latest_prices: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
        historical_data: Arc<Mutex<HashMap<String, HistoricalDataResult>>>,
    ) -> Self {
        Self {
            latest_prices,
            historical_data,
        }
    }
    
    pub async fn handle_message(&self, publish: &Publish) {
        let topic = &publish.topic;
        let payload = String::from_utf8_lossy(&publish.payload);
        debug_log(&format!("MQTT: *** MESSAGE RECEIVED *** Topic: {}, Size: {} bytes", topic, payload.len()));
        debug_log(&format!("MQTT: First 300 chars: {}", &payload[..payload.len().min(300)]));
        
        if topic == "crypto/prices/latest" {
            self.handle_latest_prices(&payload).await;
        } else if topic.starts_with("crypto/historical/") {
            self.handle_historical_data(topic, &payload).await;
        } else if topic.starts_with("crypto/prices/") {
            self.handle_individual_price(topic, &payload).await;
        } else {
            debug_log(&format!("MQTT: *** UNHANDLED TOPIC *** {}, payload: {}", topic, &payload[..payload.len().min(200)]));
        }
    }
    
    async fn handle_latest_prices(&self, payload: &str) {
        debug_log("MQTT: Processing crypto/prices/latest payload...");
        match serde_json::from_str::<Vec<CryptoCurrency>>(payload) {
            Ok(crypto_data) => {
                debug_log(&format!("MQTT: *** SUCCESS *** Parsed {} cryptocurrencies from latest prices", crypto_data.len()));
                if !crypto_data.is_empty() {
                    debug_log(&format!("MQTT: Sample crypto: {} ({}) - Price: ${:.2}", 
                        crypto_data[0].name, 
                        crypto_data[0].symbol,
                        crypto_data[0].quote.usd.price
                    ));
                }
                *self.latest_prices.lock().unwrap() = Some(crypto_data.clone());
                debug_log(&format!("MQTT: *** CACHED {} CRYPTOCURRENCIES ***", crypto_data.len()));
                info!("MQTT: Updated latest prices from broker");
            }
            Err(e) => {
                debug_log(&format!("MQTT: Failed to parse crypto/prices/latest - Error: {}", e));
                debug_log(&format!("MQTT: Full payload (first 1000 chars): {}", &payload[..payload.len().min(1000)]));
            }
        }
    }
    
    async fn handle_historical_data(&self, topic: &str, payload: &str) {
        debug_log(&format!("MQTT: Processing historical data for topic: {}", topic));
        match serde_json::from_str::<HistoricalDataResult>(payload) {
            Ok(hist_data) => {
                debug_log(&format!("MQTT: *** SUCCESS *** Parsed {} historical data points for {}", hist_data.data.len(), topic));
                let mut hist_map = self.historical_data.lock().unwrap();
                hist_map.insert(topic.to_string(), hist_data);
                debug_log(&format!("MQTT: *** CACHED HISTORICAL DATA *** for topic: {}", topic));
                info!("MQTT: Updated historical data for topic: {}", topic);
            }
            Err(e) => {
                debug_log(&format!("MQTT: Failed to parse historical data for topic {} - Error: {}", topic, e));
                debug_log(&format!("MQTT: Historical payload (first 800 chars): {}", &payload[..payload.len().min(800)]));
            }
        }
    }
    
    async fn handle_individual_price(&self, topic: &str, payload: &str) {
        debug_log(&format!("MQTT: Processing individual crypto price for topic: {}", topic));
        match serde_json::from_str::<CryptoCurrency>(payload) {
            Ok(crypto_data) => {
                debug_log(&format!("MQTT: *** SUCCESS *** Individual crypto: {} ({}) - Price: ${:.2}", 
                    crypto_data.name, 
                    crypto_data.symbol,
                    crypto_data.quote.usd.price
                ));
            }
            Err(e) => {
                debug_log(&format!("MQTT: Failed to parse individual crypto for topic {} - Error: {}", topic, e));
                debug_log(&format!("MQTT: Individual crypto payload: {}", &payload[..payload.len().min(500)]));
            }
        }
    }
}