use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use rumqttc::Publish;
use log::info;

use crate::types::{CryptoCurrency, HistoricalDataResult};
use shared::debug_log;
use super::client::PriceUpdateCallback;

pub struct MessageHandler {
    latest_prices: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
    historical_data: Arc<Mutex<HashMap<String, HistoricalDataResult>>>,
    price_update_callback: Arc<Mutex<Option<PriceUpdateCallback>>>,
    last_update_time: Arc<Mutex<Option<Instant>>>,
    debounce_duration: Duration,
}

impl MessageHandler {
    pub fn new(
        latest_prices: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
        historical_data: Arc<Mutex<HashMap<String, HistoricalDataResult>>>,
        price_update_callback: Arc<Mutex<Option<PriceUpdateCallback>>>,
    ) -> Self {
        Self {
            latest_prices,
            historical_data,
            price_update_callback,
            last_update_time: Arc::new(Mutex::new(None)),
            debounce_duration: Duration::from_millis(500), // Debounce rapid updates within 500ms
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
                
                // Check if we should debounce this update
                let should_update = {
                    let mut last_time = self.last_update_time.lock().unwrap();
                    let now = Instant::now();
                    
                    if let Some(last) = *last_time {
                        if now.duration_since(last) < self.debounce_duration {
                            debug_log("MQTT: Debouncing price update - too soon since last update");
                            false
                        } else {
                            *last_time = Some(now);
                            true
                        }
                    } else {
                        *last_time = Some(now);
                        true
                    }
                };
                
                if should_update {
                    *self.latest_prices.lock().unwrap() = Some(crypto_data.clone());
                    debug_log(&format!("MQTT: *** CACHED {} CRYPTOCURRENCIES ***", crypto_data.len()));
                    info!("MQTT: Updated latest prices from broker");
                    
                    // Trigger callback to notify iOS of price update
                    if let Some(callback) = *self.price_update_callback.lock().unwrap() {
                        debug_log("MQTT: Triggering iOS callback for price update");
                        callback(std::ptr::null());
                    } else {
                        debug_log("MQTT: No callback registered, price update not sent to iOS");
                    }
                } else {
                    debug_log("MQTT: Skipped price update due to debouncing");
                }
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