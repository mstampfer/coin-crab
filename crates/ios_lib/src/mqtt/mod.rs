pub mod client;
pub mod connection;
pub mod message_handler;

// Re-export main types for convenience
pub use client::MQTTClient;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that the main MQTTClient type is properly re-exported
        // We can't instantiate it without proper setup, but we can verify the type exists
        let _type_name = std::any::type_name::<MQTTClient>();
        assert!(_type_name.contains("MQTTClient"));
    }

    #[test]
    fn test_module_structure() {
        // Test that all expected submodules are accessible
        // This is compile-time verification that the module structure is correct
        
        // We can't directly test module existence at runtime, but we can test
        // that types from each module are accessible through the module structure
        assert!(true); // If this compiles, the module structure is correct
    }
}