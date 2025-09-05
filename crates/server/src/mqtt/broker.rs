use rumqttd::{Broker, Config as BrokerConfig};
use rumqttc::{MqttOptions, AsyncClient, Event, Packet};
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use log::{info, warn, error, debug};

pub async fn setup_mqtt_broker(broker_host: &str, broker_port: u16) -> Result<Arc<AsyncClient>, String> {
    info!("Starting embedded MQTT broker on {}:{}", broker_host, broker_port);
    
    // Load configuration from file
    let config_path = "rumqttd.toml";
    if !Path::new(config_path).exists() {
        return Err(format!("MQTT broker config file {} not found", config_path));
    }
    
    let config_content = std::fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read broker config: {}", e))?;
    
    let config: BrokerConfig = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse broker config: {}", e))?;
    
    // Start broker in background thread (broker.start() is blocking)
    thread::spawn(move || {
        let mut broker = Broker::new(config);
        info!("MQTT broker thread starting...");
        if let Err(e) = broker.start() {
            error!("MQTT broker failed: {}", e);
        }
    });
    
    // Give broker time to start
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Create MQTT client for publishing
    let mut mqttoptions = MqttOptions::new("crypto-server-publisher", broker_host, broker_port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_clean_session(true);
    mqttoptions.set_max_packet_size(102400, 102400); // Match broker config
    
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    
    // Start eventloop for the main MQTT client to enable publishing
    let client_clone = client.clone();
    tokio::spawn(async move {
        info!("Starting MQTT client eventloop for publishing");
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    info!("MQTT publisher client connected to broker");
                }
                Ok(Event::Incoming(Packet::PingResp)) => {
                    // Normal keepalive, no need to log
                }
                Ok(event) => {
                    debug!("MQTT publisher event: {:?}", event);
                }
                Err(e) => {
                    error!("MQTT publisher error: {}", e);
                    // Attempt to reconnect after error
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });
    
    // Wait for connection
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    Ok(Arc::new(client_clone))
}