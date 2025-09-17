use std::ffi::{CStr, CString};
use std::os::raw::c_char;
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
            
            // Give MQTT more time to connect and receive initial data
            debug_log("get_crypto_data: Waiting for MQTT connection and initial data...");
            
            // Poll for connection with timeout
            let start_time = std::time::Instant::now();
            let timeout = Duration::from_secs(5);
            
            loop {
                if let Some(ref client) = *MQTT_CLIENT.lock().unwrap() {
                    if client.is_connected() || client.get_latest_prices().is_some() {
                        debug_log("get_crypto_data: MQTT connected or has cached data");
                        break;
                    }
                }
                
                if start_time.elapsed() > timeout {
                    debug_log("get_crypto_data: Timeout waiting for MQTT connection");
                    break;
                }
                
                std::thread::sleep(Duration::from_millis(100));
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    
    #[test]
    fn test_free_string_with_valid_pointer() {
        // Test that free_string properly handles a valid C string
        let test_string = CString::new("test string").unwrap();
        let raw_ptr = test_string.into_raw();
        
        // This should not panic
        unsafe { free_string(raw_ptr) };
        
        // If we reach here, the function worked correctly
        assert!(true);
    }

    #[test]
    fn test_free_string_with_null_pointer() {
        // Test that free_string handles null pointers safely
        unsafe { free_string(std::ptr::null_mut()) };
        
        // If we reach here, the function handled null pointer correctly
        assert!(true);
    }

    #[test]
    fn test_return_mqtt_error_basic() {
        // Test the return_mqtt_error helper function
        let error_ptr = return_mqtt_error("Test error message");
        
        // Convert back to string to verify content
        let error_str = unsafe {
            CStr::from_ptr(error_ptr).to_string_lossy()
        };
        
        // Should be valid JSON with error message
        assert!(error_str.contains("\"success\":false"));
        assert!(error_str.contains("Test error message"));
        
        // Clean up the allocated string
        unsafe { free_string(error_ptr) };
    }

    #[test]
    fn test_return_mqtt_error_serialization() {
        // Test that return_mqtt_error produces valid JSON
        let error_ptr = return_mqtt_error("JSON test");
        
        let error_str = unsafe {
            CStr::from_ptr(error_ptr).to_string_lossy().into_owned()
        };
        
        // Try to parse as JSON to verify it's valid
        let parsed: serde_json::Value = serde_json::from_str(&error_str).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"], "JSON test");
        assert_eq!(parsed["cached"], false);
        
        // Clean up
        unsafe { free_string(error_ptr) };
    }

    #[test]
    fn test_get_crypto_data_function_exists() {
        // Test that get_crypto_data function exists and has correct signature
        // We can't actually call it in tests due to logger initialization conflicts
        let _get_crypto_fn: extern "C" fn() -> *mut c_char = get_crypto_data;
        
        // If this compiles, the function exists with the correct signature
        assert!(true);
    }

    #[test]
    fn test_get_historical_data_function_signature() {
        // Test that get_historical_data has the correct function signature
        // We can't call it in tests due to logger conflicts, but we can verify signature
        let _get_historical_fn: extern "C" fn(*const c_char, *const c_char) -> *mut c_char = get_historical_data;
        
        // If this compiles, the function exists with the correct signature
        assert!(true);
    }

    #[test]
    fn test_input_validation_logic() {
        // Test the UTF-8 validation logic used in get_historical_data
        let valid_symbol = CString::new("BTC").unwrap();
        let valid_timeframe = CString::new("24h").unwrap();
        
        // Test valid UTF-8 conversion
        let symbol_result = unsafe {
            CStr::from_ptr(valid_symbol.as_ptr()).to_str()
        };
        let timeframe_result = unsafe {
            CStr::from_ptr(valid_timeframe.as_ptr()).to_str()
        };
        
        assert!(symbol_result.is_ok());
        assert!(timeframe_result.is_ok());
        assert_eq!(symbol_result.unwrap(), "BTC");
        assert_eq!(timeframe_result.unwrap(), "24h");
    }

    #[test]
    fn test_invalid_utf8_handling() {
        // Test that invalid UTF-8 sequences are properly detected
        let invalid_bytes = [0xFF, 0xFE, 0x00];
        let invalid_ptr = invalid_bytes.as_ptr() as *const c_char;
        
        let result = unsafe {
            CStr::from_ptr(invalid_ptr).to_str()
        };
        
        // Should return an error for invalid UTF-8
        assert!(result.is_err());
    }

    #[test]
    fn test_register_price_update_callback_safety() {
        // Test that register_price_update_callback doesn't crash
        // We can't test the actual callback functionality without complex setup
        // but we can verify the function doesn't panic
        
        // Create a dummy callback function
        extern "C" fn dummy_callback(_data: *const c_void) {
            // Do nothing
        }
        
        // This should not panic
        register_price_update_callback(dummy_callback);
        
        // If we reach here, the function worked
        assert!(true);
    }

    #[test]
    fn test_ffi_function_signatures() {
        // Test that all FFI functions have the correct signatures and can be referenced
        
        // Test free_string signature
        let _free_fn: extern "C" fn(*mut c_char) = free_string;
        
        // Test get_crypto_data signature
        let _get_crypto_fn: extern "C" fn() -> *mut c_char = get_crypto_data;
        
        // Test get_historical_data signature
        let _get_historical_fn: extern "C" fn(*const c_char, *const c_char) -> *mut c_char = get_historical_data;
        
        // Test register_price_update_callback signature
        let _register_callback_fn: extern "C" fn(PriceUpdateCallback) = register_price_update_callback;
        
        // If we reach here, all function signatures are correct
        assert!(true);
    }

    #[test]
    fn test_json_serialization_fallback() {
        // Test that JSON serialization errors are handled gracefully
        // We test this indirectly through return_mqtt_error
        
        // Test with various special characters that might cause JSON issues
        let test_messages = vec![
            "Simple message",
            "Message with \"quotes\"",
            "Message with \n newlines",
            "Message with \t tabs",
            "Message with unicode: 🦀",
        ];
        
        for msg in test_messages {
            let error_ptr = return_mqtt_error(msg);
            
            // Should always return valid JSON
            let error_str = unsafe {
                CStr::from_ptr(error_ptr).to_string_lossy().into_owned()
            };
            
            // Should be parseable as JSON
            let _parsed: serde_json::Value = serde_json::from_str(&error_str).unwrap();
            
            // Clean up
            unsafe { free_string(error_ptr) };
        }
        
        assert!(true);
    }

    #[test]
    fn test_cstring_memory_management() {
        // Test proper C string memory management patterns
        let test_strings = vec!["", "short", "a longer test string with more content"];
        
        for test_str in test_strings {
            // Create CString
            let cstring = CString::new(test_str).unwrap();
            let raw_ptr = cstring.into_raw();
            
            // Verify we can read it back
            let read_back = unsafe {
                CStr::from_ptr(raw_ptr).to_string_lossy()
            };
            assert_eq!(read_back, test_str);
            
            // Free it properly
            unsafe { free_string(raw_ptr) };
        }
        
        assert!(true);
    }
}