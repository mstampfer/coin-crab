use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;
use rumqttc::{MqttOptions, AsyncClient, EventLoop, Event, Packet, QoS};
use log::{info, warn, error};

use crate::config::Config;
use crate::types::{CryptoCurrency, HistoricalDataResult};
use shared::debug_log;
use super::message_handler::MessageHandler;
use super::client::PriceUpdateCallback;

pub struct ConnectionManager {
    config: Config,
}

impl ConnectionManager {
    pub fn new(config: &Config) -> Result<Self, String> {
        Ok(ConnectionManager {
            config: config.clone(),
        })
    }
    
    pub fn create_client(&self) -> Result<(AsyncClient, EventLoop), String> {
        let mut mqttoptions = MqttOptions::new("rust-ios-client", &self.config.broker_host, self.config.broker_port);
        mqttoptions.set_keep_alive(Duration::from_secs(60));
        mqttoptions.set_clean_session(true); // Use clean session for faster connections
        mqttoptions.set_max_packet_size(102400, 102400); // Match broker config
        debug_log(&format!("MQTT: Configured MQTT options for {}:{} (keep_alive=60s, clean_session=true, max_packet=102400)",
            self.config.broker_host, self.config.broker_port));
        
        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
        debug_log("MQTT: Created async client and event loop");
        
        Ok((client, eventloop))
    }
    
    pub fn start_event_loop(
        &self,
        mut eventloop: EventLoop,
        client: Arc<AsyncClient>,
        runtime: Arc<Runtime>,
        latest_prices: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
        historical_data: Arc<Mutex<HashMap<String, HistoricalDataResult>>>,
        is_connected: Arc<Mutex<bool>>,
        connection_attempts: Arc<Mutex<u32>>,
        price_update_callback: Arc<Mutex<Option<PriceUpdateCallback>>>,
    ) {
        let message_handler = MessageHandler::new(latest_prices.clone(), historical_data.clone(), price_update_callback.clone());
        
        // Spawn event loop handling in the background
        debug_log("MQTT: About to spawn event loop thread");
        std::thread::spawn(move || {
            debug_log("MQTT: Event loop thread started");
            runtime.block_on(async {
                debug_log("MQTT: Starting event loop polling");
                loop {
                    match eventloop.poll().await {
                        Ok(Event::Incoming(Packet::ConnAck(_))) => {
                            Self::handle_connection_success(&client, &is_connected, &connection_attempts).await;
                        }
                        Ok(Event::Incoming(Packet::Publish(publish))) => {
                            message_handler.handle_message(&publish).await;
                        }
                        Ok(Event::Incoming(Packet::Disconnect)) => {
                            Self::handle_disconnect(&is_connected);
                        }
                        Err(e) => {
                            if Self::handle_connection_error(&is_connected, &connection_attempts, e).await {
                                break; // Exit the event loop after max retries
                            }
                        }
                        _ => {}
                    }
                }
            });
        });
    }
    
    async fn handle_connection_success(
        client: &Arc<AsyncClient>,
        is_connected: &Arc<Mutex<bool>>,
        connection_attempts: &Arc<Mutex<u32>>,
    ) {
        debug_log("MQTT: *** CONNECTION SUCCESSFUL *** Connected to broker!");
        info!("MQTT: Connected to broker");
        *is_connected.lock().unwrap() = true;
        *connection_attempts.lock().unwrap() = 0; // Reset retry counter on successful connection
        
        // Subscribe to topics
        debug_log("MQTT: Subscribing to crypto/prices/latest");
        if let Err(e) = client.subscribe("crypto/prices/latest", QoS::AtLeastOnce).await {
            debug_log(&format!("MQTT: Failed to subscribe to latest prices: {}", e));
        }
        debug_log("MQTT: Subscribing to crypto/historical/+/+");
        if let Err(e) = client.subscribe("crypto/historical/+/+", QoS::AtMostOnce).await {
            debug_log(&format!("MQTT: Failed to subscribe to historical data: {}", e));
        }
        debug_log("MQTT: All subscription requests sent");
    }
    
    fn handle_disconnect(is_connected: &Arc<Mutex<bool>>) {
        debug_log("MQTT: *** DISCONNECT RECEIVED *** Broker initiated disconnect");
        warn!("MQTT: Disconnected from broker");
        *is_connected.lock().unwrap() = false;
    }
    
    async fn handle_connection_error(
        is_connected: &Arc<Mutex<bool>>,
        connection_attempts: &Arc<Mutex<u32>>,
        error: rumqttc::ConnectionError,
    ) -> bool {
        error!("MQTT: Connection error: {}", error);
        *is_connected.lock().unwrap() = false;
        
        let mut attempts = connection_attempts.lock().unwrap();
        *attempts += 1;
        
        if *attempts <= 5 {
            // Exponential backoff: 2^attempt seconds (2, 4, 8, 16, 32 seconds)
            let delay_secs = 2u64.pow((*attempts - 1).min(5));  // Cap at 32 seconds
            debug_log(&format!("MQTT: Connection attempt {} failed, retrying in {} seconds", attempts, delay_secs));
            drop(attempts); // Release the lock before sleeping
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            false // Continue trying
        } else {
            debug_log(&format!("MQTT: All {} connection attempts failed, giving up", attempts));
            error!("MQTT: Maximum retry attempts exceeded, connection abandoned");
            true // Exit the event loop
        }
    }
}

// Make Config cloneable
impl Clone for Config {
    fn clone(&self) -> Self {
        Config {
            broker_host: self.broker_host.clone(),
            broker_port: self.broker_port,
            log_level: self.log_level.clone(),
        }
    }
}