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
        .unwrap_or_else(|_| {
            warn!("MQTT_BROKER_PORT not set in .env file, using default (1883)");
            1883
        });
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