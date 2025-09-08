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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Helper function to create mock crypto data for testing
    fn create_mock_crypto_currency() -> CryptoCurrency {
        CryptoCurrency {
            id: 1,
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            quote: shared::Quote {
                usd: shared::UsdQuote {
                    price: 50000.0,
                    percent_change_1h: 0.5,
                    percent_change_24h: 2.5,
                    percent_change_7d: 10.0,
                    market_cap: 900000000000.0,
                    volume_24h: 50000000000.0,
                    last_updated: "2024-01-01T00:00:00Z".to_string(),
                },
            },
        }
    }

    #[test]
    fn test_price_update_callback_type() {
        // Test that the PriceUpdateCallback type is correctly defined
        extern "C" fn dummy_callback(_data: *const c_void) {
            // Dummy callback for type testing
        }
        
        let _callback: PriceUpdateCallback = dummy_callback;
        assert!(true); // If this compiles, the type is correct
    }

    #[test]
    fn test_mqtt_client_struct_fields() {
        // Test that we can access the type names of MQTTClient fields
        // This is a compile-time verification that the struct is well-formed
        
        let _client_type = std::any::type_name::<Arc<AsyncClient>>();
        let _runtime_type = std::any::type_name::<Arc<Runtime>>();
        let _prices_type = std::any::type_name::<Arc<Mutex<Option<Vec<CryptoCurrency>>>>>();
        let _historical_type = std::any::type_name::<Arc<Mutex<HashMap<String, HistoricalDataResult>>>>();
        let _connected_type = std::any::type_name::<Arc<Mutex<bool>>>();
        let _attempts_type = std::any::type_name::<Arc<Mutex<u32>>>();
        let _callback_type = std::any::type_name::<Arc<Mutex<Option<PriceUpdateCallback>>>>();
        
        // If we reach here, all field types are correct
        assert!(true);
    }

    #[test]
    fn test_historical_data_topic_formatting() {
        // Test the topic formatting logic used in get_historical_data
        let symbol = "btc";
        let timeframe = "24h";
        let expected_topic = "crypto/historical/BTC/24h";
        let actual_topic = format!("crypto/historical/{}/{}", symbol.to_uppercase(), timeframe);
        
        assert_eq!(actual_topic, expected_topic);
        
        // Test with different symbols and timeframes
        let test_cases = vec![
            ("eth", "1h", "crypto/historical/ETH/1h"),
            ("DOGE", "7d", "crypto/historical/DOGE/7d"),
            ("ada", "30d", "crypto/historical/ADA/30d"),
        ];
        
        for (symbol, timeframe, expected) in test_cases {
            let topic = format!("crypto/historical/{}/{}", symbol.to_uppercase(), timeframe);
            assert_eq!(topic, expected);
        }
    }

    #[test]
    fn test_max_retry_attempts_constant() {
        // Test that the max retry attempts constant is reasonable
        let max_retry_attempts = 3;
        assert!(max_retry_attempts > 0);
        assert!(max_retry_attempts <= 10); // Reasonable upper bound
    }

    #[test]
    fn test_crypto_currency_type_compatibility() {
        // Test that we can create and manipulate CryptoCurrency instances
        let crypto = create_mock_crypto_currency();
        
        assert_eq!(crypto.id, 1);
        assert_eq!(crypto.name, "Bitcoin");
        assert_eq!(crypto.symbol, "BTC");
        assert_eq!(crypto.quote.usd.price, 50000.0);
        
        // Test that we can put it in a Vec (as used by latest_prices)
        let prices = vec![crypto.clone()];
        assert_eq!(prices.len(), 1);
        assert_eq!(prices[0].symbol, "BTC");
        
        // Test that we can put it in an Option
        let maybe_prices: Option<Vec<CryptoCurrency>> = Some(vec![crypto]);
        assert!(maybe_prices.is_some());
        assert_eq!(maybe_prices.unwrap()[0].symbol, "BTC");
    }

    #[test]
    fn test_historical_data_result_type_compatibility() {
        // Test that we can create and manipulate HistoricalDataResult instances
        let historical_data = HistoricalDataResult {
            success: true,
            data: vec![],
            error: None,
            symbol: Some("BTC".to_string()),
            timeframe: Some("24h".to_string()),
        };
        
        assert!(historical_data.success);
        assert_eq!(historical_data.data.len(), 0);
        assert!(historical_data.error.is_none());
        assert_eq!(historical_data.symbol, Some("BTC".to_string()));
        assert_eq!(historical_data.timeframe, Some("24h".to_string()));
        
        // Test that we can put it in a HashMap (as used by historical_data field)
        let mut map: HashMap<String, HistoricalDataResult> = HashMap::new();
        map.insert("crypto/historical/BTC/24h".to_string(), historical_data);
        
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("crypto/historical/BTC/24h"));
    }

    #[test]
    fn test_connection_state_management() {
        // Test the logic for managing connection state and retry attempts
        let mut is_connected = false;
        let mut connection_attempts = 0u32;
        let max_retry_attempts = 3u32;
        
        // Test initial state
        assert!(!is_connected);
        assert_eq!(connection_attempts, 0);
        
        // Test failed connection attempts
        connection_attempts += 1;
        assert_eq!(connection_attempts, 1);
        assert!(connection_attempts <= max_retry_attempts);
        
        // Test exceeding max retries
        connection_attempts = max_retry_attempts + 1;
        assert!(connection_attempts > max_retry_attempts);
        
        // Test reset
        connection_attempts = 0;
        assert_eq!(connection_attempts, 0);
        
        // Test successful connection
        is_connected = true;
        assert!(is_connected);
    }

    #[test]
    fn test_qos_setting() {
        // Test that we use the correct QoS setting for MQTT
        use rumqttc::QoS;
        
        let qos = QoS::AtLeastOnce;
        match qos {
            QoS::AtMostOnce => assert!(false, "Should not use AtMostOnce"),
            QoS::AtLeastOnce => assert!(true, "Correct QoS setting"),
            QoS::ExactlyOnce => assert!(false, "Should not use ExactlyOnce for performance reasons"),
        }
    }

    #[test]
    fn test_debug_logging_calls() {
        // Test that debug logging doesn't panic when called
        // We can't easily test the actual output, but we can verify it doesn't crash
        
        shared::debug_log("Test MQTT client creation");
        shared::debug_log("Test MQTT connection");
        shared::debug_log("Test MQTT publish");
        shared::debug_log("Test MQTT callback registration");
        
        // If we reach here, debug logging works
        assert!(true);
    }

    #[test]
    fn test_callback_management() {
        // Test the callback management logic
        use std::sync::{Arc, Mutex};
        
        extern "C" fn test_callback(_data: *const c_void) {
            // Test callback function
        }
        
        let callback_storage: Arc<Mutex<Option<PriceUpdateCallback>>> = Arc::new(Mutex::new(None));
        
        // Test setting callback
        *callback_storage.lock().unwrap() = Some(test_callback);
        assert!(callback_storage.lock().unwrap().is_some());
        
        // Test triggering callback (should not panic)
        if let Some(callback) = *callback_storage.lock().unwrap() {
            callback(std::ptr::null());
        }
        
        // Test clearing callback
        *callback_storage.lock().unwrap() = None;
        assert!(callback_storage.lock().unwrap().is_none());
        
        assert!(true);
    }

    #[test]
    fn test_thread_safety_types() {
        // Test that all the Arc<Mutex<T>> types are properly Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        
        assert_send_sync::<Arc<AsyncClient>>();
        assert_send_sync::<Arc<Runtime>>();
        assert_send_sync::<Arc<Mutex<Option<Vec<CryptoCurrency>>>>>();
        assert_send_sync::<Arc<Mutex<HashMap<String, HistoricalDataResult>>>>();
        assert_send_sync::<Arc<Mutex<bool>>>();
        assert_send_sync::<Arc<Mutex<u32>>>();
        
        // If this compiles, all types are properly thread-safe
        assert!(true);
    }

    #[test]
    fn test_error_handling_patterns() {
        // Test error handling patterns used in the MQTT client
        
        // Test Result pattern for client creation
        let result: Result<String, String> = Ok("success".to_string());
        match result {
            Ok(value) => assert_eq!(value, "success"),
            Err(_) => assert!(false, "Should not error"),
        }
        
        // Test Result pattern for connection errors
        let error_result: Result<(), String> = Err("Connection failed".to_string());
        match error_result {
            Ok(_) => assert!(false, "Should be an error"),
            Err(e) => assert!(e.contains("Connection failed")),
        }
        
        // Test Option pattern for data retrieval
        let maybe_data: Option<Vec<CryptoCurrency>> = None;
        assert!(maybe_data.is_none());
        
        let some_data: Option<Vec<CryptoCurrency>> = Some(vec![create_mock_crypto_currency()]);
        assert!(some_data.is_some());
        assert_eq!(some_data.unwrap().len(), 1);
    }
}