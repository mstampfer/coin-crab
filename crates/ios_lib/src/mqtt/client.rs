use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;
use std::os::raw::c_void;
use tokio::runtime::Runtime;
use rumqttc::{AsyncClient, QoS};

use crate::config::Config;
use crate::types::{CryptoCurrency, HistoricalDataResult};
use shared::debug_log;
use super::connection::ConnectionManager;
use super::message_handler::MessageHandler;

// Callback function type for notifying iOS of price updates
pub type PriceUpdateCallback = extern "C" fn(*const c_void);

// MQTT Client wrapper for thread-safe usage
pub struct MQTTClient {
    pub(crate) client: Arc<AsyncClient>,
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) latest_prices: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
    pub(crate) historical_data: Arc<Mutex<HashMap<String, HistoricalDataResult>>>,
    pub(crate) is_connected: Arc<Mutex<bool>>,
    pub(crate) connection_attempts: Arc<Mutex<u32>>,
    pub(crate) max_retry_attempts: u32,
    pub(crate) price_update_callback: Arc<Mutex<Option<PriceUpdateCallback>>>,
}

impl MQTTClient {
    pub fn new() -> Result<Self, String> {
        // Initialize logging first
        shared::init_logging();
        debug_log("MQTT: Creating new MQTTClient...");
        
        let rt = Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;
        debug_log("MQTT: Runtime created successfully");
        
        // Load configuration
        let config = Config::load()?;
        debug_log(&format!("MQTT: Connecting to broker at {}:{}", config.broker_host, config.broker_port));
        
        // Create connection manager and get client
        let connection_manager = ConnectionManager::new(&config)?;
        let (client, eventloop) = connection_manager.create_client()?;
        
        let client_arc = Arc::new(client);
        let runtime_arc = Arc::new(rt);
        let latest_prices = Arc::new(Mutex::new(None));
        let historical_data = Arc::new(Mutex::new(HashMap::new()));
        let is_connected = Arc::new(Mutex::new(false));
        let connection_attempts = Arc::new(Mutex::new(0));
        let max_retry_attempts = 3;
        let price_update_callback = Arc::new(Mutex::new(None));
        
        // Start the connection manager event loop
        connection_manager.start_event_loop(
            eventloop,
            client_arc.clone(),
            runtime_arc.clone(),
            latest_prices.clone(),
            historical_data.clone(),
            is_connected.clone(),
            connection_attempts.clone(),
            price_update_callback.clone(),
        );
        
        debug_log("MQTT: MQTTClient creation completed successfully");
        Ok(MQTTClient {
            client: client_arc,
            runtime: runtime_arc,
            latest_prices,
            historical_data,
            is_connected,
            connection_attempts,
            max_retry_attempts,
            price_update_callback,
        })
    }
    
    pub fn connect(&self) -> Result<(), String> {
        // Connection is established automatically when the event loop starts polling
        // The event loop will handle ConnAck and subscribe to topics
        debug_log("MQTT: Connection will be established by event loop");
        Ok(())
    }
    
    pub fn get_latest_prices(&self) -> Option<Vec<CryptoCurrency>> {
        self.latest_prices.lock().unwrap().clone()
    }
    
    pub fn get_historical_data(&self, symbol: &str, timeframe: &str) -> Option<HistoricalDataResult> {
        let topic = format!("crypto/historical/{}/{}", symbol.to_uppercase(), timeframe);
        self.historical_data.lock().unwrap().get(&topic).cloned()
    }
    
    pub fn is_connected(&self) -> bool {
        *self.is_connected.lock().unwrap()
    }
    
    pub fn get_connection_attempts(&self) -> u32 {
        *self.connection_attempts.lock().unwrap()
    }
    
    pub fn reset_connection_attempts(&self) {
        debug_log("MQTT: Resetting connection attempts counter");
        *self.connection_attempts.lock().unwrap() = 0;
    }
    
    pub fn has_exceeded_max_retries(&self) -> bool {
        *self.connection_attempts.lock().unwrap() > self.max_retry_attempts
    }
    
    pub async fn publish_message(&self, topic: &str, payload: &str) -> Result<(), String> {
        debug_log(&format!("MQTT: Publishing to topic: {}", topic));
        
        match self.client.publish(topic, QoS::AtLeastOnce, false, payload).await {
            Ok(_) => {
                debug_log(&format!("MQTT: Successfully published to {}", topic));
                Ok(())
            }
            Err(e) => {
                debug_log(&format!("MQTT: Failed to publish to {}: {}", topic, e));
                Err(format!("Failed to publish: {}", e))
            }
        }
    }
    
    pub fn set_price_update_callback(&self, callback: PriceUpdateCallback) {
        debug_log("MQTT: Setting price update callback");
        *self.price_update_callback.lock().unwrap() = Some(callback);
    }
    
    pub fn trigger_price_update_callback(&self) {
        if let Some(callback) = *self.price_update_callback.lock().unwrap() {
            debug_log("MQTT: Triggering price update callback");
            callback(std::ptr::null());
        }
    }
}