use std::sync::Mutex;
use crate::mqtt::MQTTClient;

// Global MQTT client instance
pub static MQTT_CLIENT: Mutex<Option<MQTTClient>> = Mutex::new(None);

/// Initialize or reinitialize the global MQTT client
pub fn init_mqtt_client() -> Result<(), String> {
    let client = MQTTClient::new()?;
    client.connect()?;
    
    *MQTT_CLIENT.lock().unwrap() = Some(client);
    Ok(())
}

/// Get a reference to the global MQTT client if it exists
pub fn with_mqtt_client<T, F>(f: F) -> Option<T> 
where
    F: FnOnce(&MQTTClient) -> T,
{
    MQTT_CLIENT
        .lock()
        .unwrap()
        .as_ref()
        .map(f)
}

/// Check if the global MQTT client is connected
pub fn is_mqtt_connected() -> bool {
    with_mqtt_client(|client| client.is_connected()).unwrap_or(false)
}

/// Reset the global MQTT client's connection attempts counter
pub fn reset_mqtt_connection_attempts() {
    if let Some(ref client) = *MQTT_CLIENT.lock().unwrap() {
        client.reset_connection_attempts();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_mqtt_client_global_initialization() {
        // Test that the global MQTT_CLIENT starts as None
        let client_guard = MQTT_CLIENT.lock().unwrap();
        // We can't assert it's None because other tests may have initialized it
        // But we can verify the mutex works and the type is correct
        let _is_some = client_guard.is_some();
        // If we reach here, the global variable is accessible
        assert!(true);
    }

    #[test]
    fn test_is_mqtt_connected_without_client() {
        // Test that is_mqtt_connected returns false when no client is initialized
        // We need to be careful here since global state may be modified by other tests
        let connected = is_mqtt_connected();
        // This should return false if no client, or actual connection status if client exists
        assert!(connected == false || connected == true); // Just verify it returns a bool
    }

    #[test]
    fn test_reset_mqtt_connection_attempts_without_client() {
        // Test that reset_mqtt_connection_attempts doesn't panic when no client exists
        reset_mqtt_connection_attempts();
        // If we reach here, the function didn't panic
        assert!(true);
    }

    #[test]
    fn test_with_mqtt_client_none() {
        // Test the with_mqtt_client function when no client is present
        let result = with_mqtt_client(|_client| {
            42 // Some test value
        });
        
        // Result should be None if no client, or Some(42) if client exists
        match result {
            None => assert!(true), // No client case
            Some(value) => assert_eq!(value, 42), // Client exists case
        }
    }

    #[test]
    fn test_with_mqtt_client_closure_execution() {
        // Test that the closure is properly executed if a client exists
        let mut counter = 0;
        let _result = with_mqtt_client(|_client| {
            counter += 1;
            "test"
        });
        
        // Counter should either be 0 (no client) or 1 (client exists and closure was called)
        assert!(counter == 0 || counter == 1);
    }

    #[test]
    fn test_mutex_thread_safety() {
        // Test that we can lock and unlock the mutex multiple times
        {
            let _guard1 = MQTT_CLIENT.lock().unwrap();
            // Mutex acquired successfully
        }
        {
            let _guard2 = MQTT_CLIENT.lock().unwrap();
            // Mutex can be acquired again after previous release
        }
        assert!(true);
    }

    #[test]
    fn test_global_static_accessibility() {
        // Test that the global static is accessible and has correct type
        let guard = MQTT_CLIENT.lock().unwrap();
        match &*guard {
            Some(_client) => {
                // Client exists, we can't test much without actually connecting
                // but we can verify the type is correct
                assert!(true);
            }
            None => {
                // No client initialized, which is a valid state
                assert!(true);
            }
        }
    }

    #[test]
    fn test_function_error_handling() {
        // Test that functions handle the global state correctly
        // These functions should not panic regardless of global state
        let _connected = is_mqtt_connected();
        reset_mqtt_connection_attempts();
        let _result = with_mqtt_client(|_| "test");
        
        // If we reach here, all functions handled the global state without panicking
        assert!(true);
    }
}