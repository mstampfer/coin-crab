use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use chrono;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;
use log::{info, warn, error};
use std::fs::OpenOptions;
use std::io::Write;
use dotenv;

// Global MQTT client instance
static MQTT_CLIENT: Mutex<Option<MQTTClient>> = Mutex::new(None);

// Debug function to write to a file
fn debug_log(msg: &str) {
    // Try multiple locations for debug log
    let log_paths = [
        "/tmp/mqtt_debug.log",
        "/var/tmp/mqtt_debug.log", 
        "/usr/tmp/mqtt_debug.log",
        "./mqtt_debug.log"
    ];
    
    // Get current timestamp
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
    
    for path in &log_paths {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path) {
            let _ = writeln!(file, "[{}] [DEBUG] {}", timestamp, msg);
            break;
        }
    }
    println!("{}", msg); // Also print to console
}

#[cfg(test)]
mod test_historical;

#[cfg(test)]
mod test_c_function;

#[cfg(test)]
mod test_timeframes;

#[cfg(test)]
mod test_specific_symbols;

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
pub struct ApiResponse {
    pub data: Vec<CryptoCurrency>,
    pub last_updated: String,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CryptoClientResult {
    pub success: bool,
    pub data: Option<Vec<CryptoCurrency>>,
    pub error: Option<String>,
    pub last_updated: Option<String>,
    pub cached: bool,
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

// MQTT Client wrapper for thread-safe usage
pub struct MQTTClient {
    client: Arc<AsyncClient>,
    runtime: Arc<Runtime>,
    latest_prices: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
    historical_data: Arc<Mutex<HashMap<String, HistoricalDataResult>>>,
    is_connected: Arc<Mutex<bool>>,
    connection_attempts: Arc<Mutex<u32>>,
    max_retry_attempts: u32,
}

impl MQTTClient {
    pub fn new() -> Result<Self, String> {
        debug_log("MQTT: Creating new MQTTClient...");
        let rt = Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;
        debug_log("MQTT: Runtime created successfully");
        
        // Load .env file and get MQTT broker host
        // First try to load .env.client file, but don't fail if it doesn't exist
        if let Err(_) = dotenv::from_filename(".env.client") {
            debug_log("MQTT: .env.client file not found, using environment variables");
        }
        
        let broker_host = std::env::var("MQTT_BROKER_HOST").unwrap_or_else(|_| {
            debug_log("MQTT: MQTT_BROKER_HOST not set in .env file, using localhost (127.0.0.1)");
            debug_log("MQTT: For iOS device testing, update MQTT_BROKER_HOST in .env file to your machine's IP address");
            "127.0.0.1".to_string()
        });
        debug_log(&format!("MQTT: Connecting to broker at {}:1883", broker_host));
        let mut mqttoptions = MqttOptions::new("rust-ios-client", &broker_host, 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(60)); // Match working client
        mqttoptions.set_clean_session(false); // Match working client 
        mqttoptions.set_max_packet_size(102400, 102400); // Match broker config
        debug_log(&format!("MQTT: Configured MQTT options for {}:1883 (keep_alive=60s, clean_session=false, max_packet=102400)", broker_host));
        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        debug_log("MQTT: Created async client and event loop");
        
        let client_arc = Arc::new(client);
        let runtime_arc = Arc::new(rt);
        let latest_prices = Arc::new(Mutex::new(None));
        let historical_data = Arc::new(Mutex::new(HashMap::new()));
        let is_connected = Arc::new(Mutex::new(false));
        let connection_attempts = Arc::new(Mutex::new(0));
        let max_retry_attempts = 3;
        
        // Clone for the event loop task
        let client_clone = client_arc.clone();
        let runtime_clone = runtime_arc.clone();
        let latest_prices_clone = latest_prices.clone();
        let historical_data_clone = historical_data.clone();
        let is_connected_clone = is_connected.clone();
        let connection_attempts_clone = connection_attempts.clone();
        
        // Spawn event loop handling in the background
        debug_log("MQTT: About to spawn event loop thread");
        std::thread::spawn(move || {
            debug_log("MQTT: Event loop thread started");
            runtime_clone.block_on(async {
                debug_log("MQTT: Starting event loop polling");
                loop {
                    match eventloop.poll().await {
                        Ok(Event::Incoming(Packet::ConnAck(_))) => {
                            debug_log("MQTT: *** CONNECTION SUCCESSFUL *** Connected to broker!");
                            info!("MQTT: Connected to broker");
                            *is_connected_clone.lock().unwrap() = true;
                            *connection_attempts_clone.lock().unwrap() = 0; // Reset retry counter on successful connection
                            
                            // Subscribe to topics
                            debug_log("MQTT: Subscribing to crypto/prices/latest");
                            if let Err(e) = client_clone.subscribe("crypto/prices/latest", QoS::AtLeastOnce).await {
                                debug_log(&format!("MQTT: Failed to subscribe to latest prices: {}", e));
                            }
                            debug_log("MQTT: Subscribing to crypto/prices/+");
                            if let Err(e) = client_clone.subscribe("crypto/prices/+", QoS::AtLeastOnce).await {
                                debug_log(&format!("MQTT: Failed to subscribe to individual prices: {}", e));
                            }
                            debug_log("MQTT: Subscribing to crypto/historical/+/+");
                            if let Err(e) = client_clone.subscribe("crypto/historical/+/+", QoS::AtMostOnce).await {
                                debug_log(&format!("MQTT: Failed to subscribe to historical data: {}", e));
                            }
                            debug_log("MQTT: All subscription requests sent");
                        }
                        Ok(Event::Incoming(Packet::Publish(publish))) => {
                            let topic = &publish.topic;
                            let payload = String::from_utf8_lossy(&publish.payload);
                            debug_log(&format!("MQTT: *** MESSAGE RECEIVED *** Topic: {}, Size: {} bytes", topic, payload.len()));
                            debug_log(&format!("MQTT: First 300 chars: {}", &payload[..payload.len().min(300)]));
                            
                            if topic == "crypto/prices/latest" {
                                // Handle latest prices update
                                debug_log("MQTT: Processing crypto/prices/latest payload...");
                                match serde_json::from_str::<Vec<CryptoCurrency>>(&payload) {
                                    Ok(crypto_data) => {
                                        debug_log(&format!("MQTT: *** SUCCESS *** Parsed {} cryptocurrencies from latest prices", crypto_data.len()));
                                        if !crypto_data.is_empty() {
                                            debug_log(&format!("MQTT: Sample crypto: {} ({}) - Price: ${:.2}", 
                                                crypto_data[0].name, 
                                                crypto_data[0].symbol,
                                                crypto_data[0].quote.usd.price
                                            ));
                                        }
                                        *latest_prices_clone.lock().unwrap() = Some(crypto_data.clone());
                                        debug_log(&format!("MQTT: *** CACHED {} CRYPTOCURRENCIES ***", crypto_data.len()));
                                        info!("MQTT: Updated latest prices from broker");
                                    }
                                    Err(e) => {
                                        debug_log(&format!("MQTT: Failed to parse crypto/prices/latest - Error: {}", e));
                                        debug_log(&format!("MQTT: Full payload (first 1000 chars): {}", &payload[..payload.len().min(1000)]));
                                    }
                                }
                            } else if topic.starts_with("crypto/historical/") {
                                // Handle historical data: crypto/historical/BTC/24h
                                debug_log(&format!("MQTT: Processing historical data for topic: {}", topic));
                                match serde_json::from_str::<HistoricalDataResult>(&payload) {
                                    Ok(hist_data) => {
                                        debug_log(&format!("MQTT: *** SUCCESS *** Parsed {} historical data points for {}", hist_data.data.len(), topic));
                                        let mut hist_map = historical_data_clone.lock().unwrap();
                                        hist_map.insert(topic.to_string(), hist_data);
                                        debug_log(&format!("MQTT: *** CACHED HISTORICAL DATA *** for topic: {}", topic));
                                        info!("MQTT: Updated historical data for topic: {}", topic);
                                    }
                                    Err(e) => {
                                        debug_log(&format!("MQTT: Failed to parse historical data for topic {} - Error: {}", topic, e));
                                        debug_log(&format!("MQTT: Historical payload (first 800 chars): {}", &payload[..payload.len().min(800)]));
                                    }
                                }
                            } else if topic.starts_with("crypto/prices/") {
                                // Handle individual crypto price updates
                                debug_log(&format!("MQTT: Processing individual crypto price for topic: {}", topic));
                                match serde_json::from_str::<CryptoCurrency>(&payload) {
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
                            } else {
                                debug_log(&format!("MQTT: *** UNHANDLED TOPIC *** {}, payload: {}", topic, &payload[..payload.len().min(200)]));
                            }
                        }
                        Ok(Event::Incoming(Packet::Disconnect)) => {
                            debug_log("MQTT: *** DISCONNECT RECEIVED *** Broker initiated disconnect");
                            warn!("MQTT: Disconnected from broker");
                            *is_connected_clone.lock().unwrap() = false;
                        }
                        Err(e) => {
                            error!("MQTT: Connection error: {}", e);
                            *is_connected_clone.lock().unwrap() = false;
                            
                            let mut attempts = connection_attempts_clone.lock().unwrap();
                            *attempts += 1;
                            
                            if *attempts <= 3 {
                                // Exponential backoff: 2^attempt seconds (2, 4, 8 seconds)
                                let delay_secs = 2u64.pow(*attempts - 1);
                                debug_log(&format!("MQTT: Connection attempt {} failed, retrying in {} seconds", attempts, delay_secs));
                                drop(attempts); // Release the lock before sleeping
                                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                            } else {
                                debug_log(&format!("MQTT: All {} connection attempts failed, giving up", attempts));
                                error!("MQTT: Maximum retry attempts exceeded, connection abandoned");
                                break; // Exit the event loop after max retries
                            }
                        }
                        _ => {}
                    }
                }
            });
        });
        
        debug_log("MQTT: MQTTClient creation completed successfully");
        Ok(MQTTClient {
            client: client_arc,
            runtime: runtime_arc,
            latest_prices,
            historical_data,
            is_connected,
            connection_attempts,
            max_retry_attempts,
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
        
        match self.client.publish(topic, rumqttc::QoS::AtLeastOnce, false, payload).await {
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
}



#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if s.is_null() {
            return;
        }
        let _ = CString::from_raw(s);
    }
}

// Simplified single function for Swift to get crypto data
#[no_mangle]
pub extern "C" fn get_crypto_data() -> *mut c_char {
    debug_log("get_crypto_data: Starting data fetch using MQTT");
    
    // Initialize MQTT client if needed (but only once)
    {
        let client_exists = MQTT_CLIENT.lock().unwrap().is_some();
        if !client_exists {
            debug_log("get_crypto_data: MQTT client not initialized, creating new client...");
            match MQTTClient::new() {
                Ok(client) => {
                    debug_log("get_crypto_data: MQTT client created successfully");
                    if let Err(e) = client.connect() {
                        debug_log(&format!("get_crypto_data: Failed to connect to MQTT broker: {}", e));
                        return return_mqtt_error("Failed to connect to MQTT broker");
                    }
                    *MQTT_CLIENT.lock().unwrap() = Some(client);
                }
                Err(e) => {
                    debug_log(&format!("get_crypto_data: Failed to initialize MQTT client: {}", e));
                    return return_mqtt_error("Failed to initialize MQTT client");
                }
            }
            
            // Give MQTT time to connect and receive initial data (reduced from 5s to 2s)
            debug_log("get_crypto_data: Waiting for MQTT connection and initial data...");
            std::thread::sleep(Duration::from_millis(2000));
        } else {
            debug_log("get_crypto_data: Using existing MQTT client");
        }
    }
    
    // Try to get latest prices from MQTT client
    if let Some(ref client) = *MQTT_CLIENT.lock().unwrap() {
        if let Some(prices) = client.get_latest_prices() {
            debug_log(&format!("get_crypto_data: Successfully got {} cryptocurrencies via MQTT", prices.len()));
            
            let result = CryptoClientResult {
                success: true,
                data: Some(prices),
                error: None,
                last_updated: Some(chrono::Utc::now().to_rfc3339()),
                cached: true,
            };
            
            match serde_json::to_string(&result) {
                Ok(json) => {
                    debug_log(&format!("get_crypto_data: Successfully returning {} bytes via MQTT", json.len()));
                    return CString::new(json).unwrap().into_raw();
                }
                Err(e) => {
                    debug_log(&format!("get_crypto_data: MQTT serialization error: {}", e));
                }
            }
        } else {
            debug_log("get_crypto_data: MQTT client has no cached data");
        }
    } else {
        debug_log("get_crypto_data: MQTT client not available");
    }
    
    debug_log("get_crypto_data: MQTT data not available");
    return_mqtt_error("MQTT connection failed or no data available")
}

// Helper function for returning MQTT errors
fn return_mqtt_error(error_msg: &str) -> *mut c_char {
    debug_log(&format!("return_mqtt_error: {}", error_msg));
    
    let error_result = CryptoClientResult {
        success: false,
        data: None,
        error: Some(error_msg.to_string()),
        last_updated: None,
        cached: false,
    };
    
    let json = serde_json::to_string(&error_result).unwrap_or_else(|_| {
        r#"{"success":false,"error":"MQTT connection failed","data":null,"last_updated":null,"cached":false}"#.to_string()
    });
    CString::new(json).unwrap().into_raw()
}




// Generic historical data function (no MQTT reference in name)
#[no_mangle]
pub extern "C" fn get_historical_data(symbol: *const c_char, timeframe: *const c_char) -> *mut c_char {
    debug_log("get_historical_data: Starting historical data fetch");
    
    let symbol_str = unsafe {
        match CStr::from_ptr(symbol).to_str() {
            Ok(s) => s,
            Err(_) => {
                debug_log("get_historical_data: Invalid symbol string");
                return CString::new("{\"success\":false,\"error\":\"Invalid symbol\",\"data\":[]}").unwrap().into_raw();
            }
        }
    };
    
    let timeframe_str = unsafe {
        match CStr::from_ptr(timeframe).to_str() {
            Ok(s) => s,
            Err(_) => {
                debug_log("get_historical_data: Invalid timeframe string");
                return CString::new("{\"success\":false,\"error\":\"Invalid timeframe\",\"data\":[]}").unwrap().into_raw();
            }
        }
    };
    
    debug_log(&format!("get_historical_data: Fetching {} {} via MQTT", symbol_str, timeframe_str));
    
    // Initialize MQTT client if needed
    let is_connected = if let Some(ref client) = *MQTT_CLIENT.lock().unwrap() {
        client.is_connected()
    } else {
        false
    };
    
    if !is_connected {
        debug_log("get_historical_data: MQTT not connected, initializing...");
        match MQTTClient::new() {
            Ok(client) => {
                debug_log("get_historical_data: MQTT client created successfully");
                if let Err(e) = client.connect() {
                    debug_log(&format!("get_historical_data: Failed to connect to MQTT broker: {}", e));
                    return CString::new("{\"success\":false,\"error\":\"Failed to connect to MQTT broker\",\"data\":[]}").unwrap().into_raw();
                }
                *MQTT_CLIENT.lock().unwrap() = Some(client);
            }
            Err(e) => {
                debug_log(&format!("get_historical_data: Failed to initialize MQTT client: {}", e));
                return CString::new("{\"success\":false,\"error\":\"Failed to initialize MQTT client\",\"data\":[]}").unwrap().into_raw();
            }
        }
        
        // Give MQTT time to connect and receive data
        debug_log("get_historical_data: Waiting for MQTT connection and data...");
        std::thread::sleep(Duration::from_millis(1000));
    }
    
    // Try to get historical data from MQTT client
    if let Some(ref client) = *MQTT_CLIENT.lock().unwrap() {
        if let Some(hist_data) = client.get_historical_data(symbol_str, timeframe_str) {
            debug_log(&format!("get_historical_data: Successfully got {} data points via MQTT", hist_data.data.len()));
            let json = serde_json::to_string(&hist_data).unwrap();
            return CString::new(json).unwrap().into_raw();
        } else {
            debug_log("get_historical_data: MQTT client has no cached historical data");
        }
    } else {
        debug_log("get_historical_data: MQTT client not available");
    }
    
    // No MQTT data available - request from server and retry
    debug_log(&format!("get_historical_data: Requesting {} {} from server via MQTT", symbol_str, timeframe_str));
    
    // Publish request to server
    if let Some(ref client) = *MQTT_CLIENT.lock().unwrap() {
        let request_payload = format!("{}:{}", symbol_str, timeframe_str);
        debug_log(&format!("get_historical_data: Publishing request: {}", request_payload));
        
        // Use client runtime to publish request
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = client.publish_message("crypto/requests/historical", &request_payload).await {
                debug_log(&format!("get_historical_data: Failed to publish request: {}", e));
            } else {
                debug_log("get_historical_data: Request published successfully");
            }
        });
        
        // Wait and retry for data (server needs time to fetch from CMC API)
        debug_log("get_historical_data: Waiting for server to populate data...");
        std::thread::sleep(Duration::from_millis(2000)); // Wait 2 seconds
        
        // Retry getting the data
        if let Some(hist_data) = client.get_historical_data(symbol_str, timeframe_str) {
            debug_log(&format!("get_historical_data: Successfully got {} data points after retry", hist_data.data.len()));
            let json = serde_json::to_string(&hist_data).unwrap();
            return CString::new(json).unwrap().into_raw();
        } else {
            debug_log("get_historical_data: Still no data after retry - server may be busy");
        }
    }
    
    let error_result = HistoricalDataResult {
        success: false,
        data: vec![],
        error: Some("MQTT data not available after request - server may be busy".to_string()),
        symbol: Some(symbol_str.to_string()),
        timeframe: Some(timeframe_str.to_string()),
    };
    
    let json = serde_json::to_string(&error_result).unwrap_or_else(|_| {
        r#"{"success":false,"error":"MQTT data not available after request","data":[]}"#.to_string()
    });
    CString::new(json).unwrap().into_raw()
}





