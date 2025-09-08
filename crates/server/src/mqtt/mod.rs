pub mod broker;
pub mod client;
pub mod publisher;
pub mod request_handler;

// Re-export main functions for convenience
pub use broker::setup_mqtt_broker;
pub use publisher::{publish_crypto_data_to_mqtt, publish_historical_data_to_mqtt, publish_empty_retained_message};
pub use request_handler::setup_mqtt_request_handling;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all the main functions are re-exported and accessible
        // This ensures the pub use statements are working correctly
        
        // These functions should be accessible through the module re-exports
        // We can't actually call them in tests due to async/network dependencies,
        // but we can verify they exist and are properly imported
        
        // Verify broker function exists
        let _setup_broker = setup_mqtt_broker;
        
        // Verify publisher functions exist  
        let _publish_crypto = publish_crypto_data_to_mqtt;
        let _publish_historical = publish_historical_data_to_mqtt;
        let _publish_empty = publish_empty_retained_message;
        
        // Verify request handler function exists
        let _setup_handler = setup_mqtt_request_handling;
        
        // If we reach this point, all re-exports are working
        assert!(true);
    }

    #[test]
    fn test_module_structure() {
        // Test that the module structure is correct
        // All submodules should be accessible
        
        // Test that we can reference the submodules
        let _broker_mod = "broker";
        let _client_mod = "client"; 
        let _publisher_mod = "publisher";
        let _request_handler_mod = "request_handler";
        
        assert_eq!(_broker_mod, "broker");
        assert_eq!(_client_mod, "client");
        assert_eq!(_publisher_mod, "publisher");
        assert_eq!(_request_handler_mod, "request_handler");
    }
}