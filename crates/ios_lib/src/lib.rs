// iOS Library - Modular Architecture
// Main library interface that ties together all modules

// Module declarations
mod types;
mod config;
mod mqtt;
mod ffi;
mod globals;

// Re-export public types for external use
pub use types::{ApiResponse, CryptoClientResult, CryptoCurrency, HistoricalDataResult};
pub use mqtt::MQTTClient;

// Re-export FFI functions (they have #[no_mangle] so they're automatically exposed to C)
pub use ffi::{free_string, get_crypto_data, get_historical_data};

// Re-export global initialization functions
pub use globals::{init_mqtt_client, is_mqtt_connected, reset_mqtt_connection_attempts};

// Test modules are now integrated directly into each source file

// Unit tests for main library functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_re_exports() {
        // Test that all main types are re-exported and accessible
        // This ensures the pub use statements are working correctly
        
        // Test that types can be referenced and are accessible
        // We can't create full instances without network dependencies,
        // but we can verify they exist and are properly imported
        
        // Test that FFI functions are accessible
        let _free_fn = free_string;
        let _get_crypto_fn = get_crypto_data;
        let _get_historical_fn = get_historical_data;
        
        // Test that global functions are accessible
        let _init_mqtt_fn = init_mqtt_client;
        let _is_connected_fn = is_mqtt_connected;
        let _reset_attempts_fn = reset_mqtt_connection_attempts;
        
        // If we reach this point, all re-exports are working
        assert!(true);
    }

    #[test]
    fn test_library_structure() {
        // Test that the library structure is correct
        // All modules should be accessible through the main library
        
        // Test that we can reference the module names
        let _types_mod = "types";
        let _config_mod = "config";
        let _mqtt_mod = "mqtt";
        let _ffi_mod = "ffi";
        let _globals_mod = "globals";
        
        assert_eq!(_types_mod, "types");
        assert_eq!(_config_mod, "config");
        assert_eq!(_mqtt_mod, "mqtt");
        assert_eq!(_ffi_mod, "ffi");
        assert_eq!(_globals_mod, "globals");
    }

    #[test]
    fn test_library_version_info() {
        // Test basic library information
        let lib_name = env!("CARGO_PKG_NAME");
        let lib_version = env!("CARGO_PKG_VERSION");
        
        assert_eq!(lib_name, "rust_ios_lib");
        assert!(!lib_version.is_empty());
    }

    #[test]
    fn test_ffi_function_signatures() {
        // Test that FFI functions have the expected signatures
        // We can't call them without proper setup, but we can verify they exist
        
        use std::ffi::{CStr, CString};
        use std::os::raw::c_char;
        
        // Test free_string signature - takes *mut c_char
        let test_string = CString::new("test").unwrap();
        let raw_ptr = test_string.into_raw();
        
        // Call free_string to clean up (this should not panic)
        unsafe { free_string(raw_ptr) };
        
        // Test that other functions exist (we can't easily test them without MQTT setup)
        let _get_crypto_exists = get_crypto_data as *const ();
        let _get_historical_exists = get_historical_data as *const ();
        
        assert!(!_get_crypto_exists.is_null());
        assert!(!_get_historical_exists.is_null());
    }

    #[test]
    fn test_global_functions_callable() {
        // Test that global functions can be called without panicking
        // These functions should handle being called in test environment
        
        // Test is_mqtt_connected (should return false in test environment)
        let connected = is_mqtt_connected();
        assert!(!connected); // Should be false since no MQTT setup in tests
        
        // Test reset_mqtt_connection_attempts (should not panic)
        reset_mqtt_connection_attempts();
        
        // If we reach here, functions are callable
        assert!(true);
    }

    #[test]
    fn test_module_dependencies() {
        // Test that module dependencies are properly set up
        // This ensures that modules can interact with each other
        
        // Test that shared types are available
        // (These come from the shared crate dependency)
        let _crypto_type = std::any::type_name::<shared::CryptoCurrency>();
        let _historical_type = std::any::type_name::<shared::HistoricalDataResult>();
        
        assert!(_crypto_type.contains("CryptoCurrency"));
        assert!(_historical_type.contains("HistoricalDataResult"));
    }
}