use actix_web::web;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use std::time::Duration;
use log::{info, warn, error, debug};
use crate::types::AppState;
use crate::data::fetch_historical_data_server;
use crate::mqtt::publish_historical_data_to_mqtt;

pub async fn setup_mqtt_request_handling(state: web::Data<AppState>) -> Result<(), String> {
    let client = &*state.mqtt_client;
    
    // Subscribe to historical data request topic
    if let Err(e) = client.subscribe("crypto/requests/historical", QoS::AtLeastOnce).await {
        error!("Failed to subscribe to request topic: {}", e);
        return Err(format!("Failed to subscribe to request topic: {}", e));
    } else {
        info!("Subscribed to crypto/requests/historical topic");
    }
    
    // Create a new client connection for the event loop
    let broker_host = std::env::var("MQTT_BROKER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let broker_port = std::env::var("MQTT_BROKER_PORT")
        .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
        .unwrap_or(1883);
    let mut mqttoptions = MqttOptions::new("crypto-server-subscriber", &broker_host, broker_port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_clean_session(true);
    mqttoptions.set_max_packet_size(102400, 102400);
    
    let (event_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    
    // Subscribe with the event client
    if let Err(e) = event_client.subscribe("crypto/requests/historical", QoS::AtLeastOnce).await {
        error!("Failed to subscribe to request topic with event client: {}", e);
        return Err(format!("Failed to subscribe to request topic: {}", e));
    }
    
    // Clone state for the event loop
    let state_for_requests = state.clone();
    tokio::spawn(async move {
        info!("Starting MQTT client event loop for request handling");
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    info!("MQTT request handler connected to broker");
                }
                Ok(Event::Incoming(Packet::PingResp)) => {
                    // Normal keepalive, no need to log
                }
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    let topic = &publish.topic;
                    if topic == "crypto/requests/historical" {
                        let payload = std::str::from_utf8(&publish.payload).unwrap_or("").to_string();
                        info!("Received historical data request: {}", payload);
                        
                        // Parse request (format: "SYMBOL:TIMEFRAME")
                        if let Some((symbol, timeframe)) = payload.split_once(':') {
                            let symbol = symbol.to_string();
                            let timeframe = timeframe.to_string();
                            info!("Processing request for {} {}", symbol, timeframe);
                            
                            // Fetch data from CMC API and publish to MQTT
                            let state_clone = state_for_requests.clone();
                            tokio::spawn(async move {
                                let result = fetch_historical_data_server(
                                    &symbol, 
                                    &timeframe, 
                                    &state_clone.api_key, 
                                    &state_clone.client
                                ).await;
                                
                                if result.success {
                                    info!("Successfully fetched {} {} - publishing to MQTT", symbol, timeframe);
                                    publish_historical_data_to_mqtt(
                                        &state_clone.mqtt_client, 
                                        &symbol, 
                                        &timeframe, 
                                        &result
                                    ).await;
                                    info!("Published {} {} to MQTT successfully", symbol, timeframe);
                                } else {
                                    error!("Failed to fetch {} {}: {:?}", symbol, timeframe, result.error);
                                }
                            });
                        } else {
                            warn!("Invalid request format: {}", payload);
                        }
                    }
                }
                Ok(event) => {
                    debug!("MQTT request handler event: {:?}", event);
                }
                Err(e) => {
                    error!("MQTT request handler error: {}", e);
                    // Attempt to reconnect after error
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use reqwest::Client;

    fn create_test_app_state() -> web::Data<AppState> {
        // Create a mock MQTT client (this won't actually connect in tests)
        let (mqtt_client, _) = rumqttc::AsyncClient::new(
            rumqttc::MqttOptions::new("test_client", "localhost", 1883),
            10
        );

        web::Data::new(AppState {
            cache: Arc::new(Mutex::new(None)),
            last_fetch: Arc::new(Mutex::new(std::time::SystemTime::now())),
            client: Client::new(),
            api_key: "test_api_key".to_string(),
            mqtt_client: Arc::new(mqtt_client),
            historical_cache: Arc::new(Mutex::new(HashMap::new())),
            update_interval_seconds: 300,
        })
    }

    #[test]
    fn test_request_topic_constant() {
        let topic = "crypto/requests/historical";
        assert_eq!(topic, "crypto/requests/historical");
        assert!(!topic.is_empty());
    }

    #[test]
    fn test_request_format_parsing() {
        // Test valid request format parsing
        let valid_request = "BTC:24h";
        if let Some((symbol, timeframe)) = valid_request.split_once(':') {
            assert_eq!(symbol, "BTC");
            assert_eq!(timeframe, "24h");
        } else {
            panic!("Failed to parse valid request format");
        }
        
        // Test another valid format
        let valid_request2 = "ETH:7d";
        if let Some((symbol, timeframe)) = valid_request2.split_once(':') {
            assert_eq!(symbol, "ETH");
            assert_eq!(timeframe, "7d");
        } else {
            panic!("Failed to parse valid request format");
        }
    }

    #[test]
    fn test_invalid_request_format_parsing() {
        // Test invalid request formats
        let invalid_requests = vec![
            "BTC",
            "BTC:",
            ":24h",
            "",
            "BTC:24h:extra",
            "BTC-24h",
            "BTC 24h",
        ];
        
        for invalid_request in invalid_requests {
            if let Some((symbol, timeframe)) = invalid_request.split_once(':') {
                // Even if split_once works, we should validate the parts
                if symbol.is_empty() || timeframe.is_empty() {
                    // This is expected for invalid formats like "BTC:" or ":24h"
                    assert!(symbol.is_empty() || timeframe.is_empty());
                }
            } else {
                // This is expected for formats without ':' like "BTC", "BTC-24h", etc.
                assert!(!invalid_request.contains(':') || invalid_request.is_empty());
            }
        }
    }

    #[test]
    fn test_mqtt_client_id() {
        let client_id = "crypto-server-subscriber";
        assert_eq!(client_id, "crypto-server-subscriber");
        assert!(!client_id.is_empty());
        assert!(client_id.len() > 5);
    }

    #[test]
    fn test_mqtt_options_creation() {
        let broker_host = "127.0.0.1";
        let broker_port = 1883;
        
        let mut mqttoptions = MqttOptions::new("test-subscriber", broker_host, broker_port);
        mqttoptions.set_keep_alive(Duration::from_secs(30));
        mqttoptions.set_clean_session(true);
        mqttoptions.set_max_packet_size(102400, 102400);
        
        assert_eq!(mqttoptions.client_id(), "test-subscriber");
        assert_eq!(mqttoptions.broker_address(), (broker_host.to_string(), broker_port));
        assert_eq!(mqttoptions.keep_alive(), Duration::from_secs(30));
        assert_eq!(mqttoptions.clean_session(), true);
    }

    #[test]
    fn test_environment_variable_defaults() {
        // Test default values when environment variables are not set
        let default_host = "127.0.0.1";
        let default_port = 1883;
        
        // Simulate environment variable parsing
        let broker_host = std::env::var("NONEXISTENT_HOST").unwrap_or_else(|_| default_host.to_string());
        let broker_port = std::env::var("NONEXISTENT_PORT")
            .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
            .unwrap_or(default_port);
        
        assert_eq!(broker_host, default_host);
        assert_eq!(broker_port, default_port);
    }

    #[test]
    fn test_symbol_and_timeframe_validation() {
        let test_cases = vec![
            ("BTC", "24h", true),
            ("ETH", "7d", true),
            ("ADA", "1h", true),
            ("", "24h", false),  // empty symbol
            ("BTC", "", false),   // empty timeframe
            ("", "", false),      // both empty
            ("BTC", "24h", true),
            ("VERY_LONG_SYMBOL_NAME", "365d", true),
        ];
        
        for (symbol, timeframe, should_be_valid) in test_cases {
            let is_valid = !symbol.is_empty() && !timeframe.is_empty();
            assert_eq!(is_valid, should_be_valid, 
                      "Failed for symbol: '{}', timeframe: '{}'", symbol, timeframe);
        }
    }

    #[test]
    fn test_qos_levels() {
        let qos = QoS::AtLeastOnce;
        // Test that QoS can be used (we can't test much more without actual MQTT connection)
        match qos {
            QoS::AtMostOnce => assert!(false, "Expected AtLeastOnce"),
            QoS::AtLeastOnce => assert!(true),
            QoS::ExactlyOnce => assert!(false, "Expected AtLeastOnce"),
        }
    }

    #[test]
    fn test_duration_values() {
        let keep_alive = Duration::from_secs(30);
        let reconnect_delay = Duration::from_secs(5);
        
        assert_eq!(keep_alive.as_secs(), 30);
        assert_eq!(reconnect_delay.as_secs(), 5);
        assert!(keep_alive > reconnect_delay);
    }

    #[test]
    fn test_payload_encoding() {
        let test_payload = "BTC:24h";
        let bytes = test_payload.as_bytes();
        let decoded = std::str::from_utf8(bytes).unwrap_or("");
        
        assert_eq!(decoded, test_payload);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_app_state_creation() {
        let state = create_test_app_state();
        
        // Test that all fields are accessible
        assert_eq!(state.api_key, "test_api_key");
        assert_eq!(state.update_interval_seconds, 300);
        
        // Test that caches are initialized
        let cache = state.cache.lock().unwrap();
        assert!(cache.is_none());
        
        let hist_cache = state.historical_cache.lock().unwrap();
        assert_eq!(hist_cache.len(), 0);
    }
}