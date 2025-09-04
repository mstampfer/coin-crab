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