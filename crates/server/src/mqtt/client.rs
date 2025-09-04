// This module can be expanded later for additional client-specific functionality
// Currently, client setup is handled in broker.rs as part of the broker setup process

use rumqttc::{MqttOptions, AsyncClient};
use std::time::Duration;

pub fn create_mqtt_options(client_id: &str, broker_host: &str, broker_port: u16) -> MqttOptions {
    let mut mqttoptions = MqttOptions::new(client_id, broker_host, broker_port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_clean_session(true);
    mqttoptions.set_max_packet_size(102400, 102400);
    mqttoptions
}

pub fn create_async_client(options: MqttOptions, cap: usize) -> (AsyncClient, rumqttc::EventLoop) {
    AsyncClient::new(options, cap)
}