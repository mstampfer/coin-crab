use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::globals::MQTT_CLIENT;
use crate::mqtt::{MQTTClient, client::PriceUpdateCallback};
use crate::types::{CryptoClientResult, HistoricalDataResult};
use shared::debug_log;

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

// Function to register iOS callback for real-time price updates
#[no_mangle]
pub extern "C" fn register_price_update_callback(callback: PriceUpdateCallback) {
    debug_log("register_price_update_callback: Registering iOS callback for real-time price updates");
    
    if let Some(ref client) = *MQTT_CLIENT.lock().unwrap() {
        client.set_price_update_callback(callback);
        debug_log("register_price_update_callback: Callback registered successfully");
    } else {
        debug_log("register_price_update_callback: MQTT client not initialized - callback will be lost");
    }
}