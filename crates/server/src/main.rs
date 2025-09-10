// Server Main - Modular Architecture
// Main server entry point that coordinates all modules

use actix_web::{web, App, HttpServer, middleware::Logger};
use reqwest::Client;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::collections::HashMap;
use log::{info, error};

// Module declarations
mod types;
mod config;
mod handlers;
mod mqtt;
mod data;

// Import our modules
use types::AppState;
use config::ServerConfig;
use handlers::{get_prices, health_check, get_historical_data, get_cmc_mapping, get_crypto_logo};
use mqtt::{setup_mqtt_broker, setup_mqtt_request_handling};
use data::{fetch_data_periodically, clear_mqtt_cache_periodically, fetch_cmc_mapping};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let config = ServerConfig::load().map_err(|e| {
        eprintln!("Failed to load server configuration: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;
    
    // Setup logging
    config.setup_logging();
    
    // Setup MQTT broker and client  
    let mqtt_client = match setup_mqtt_broker(&config.mqtt_broker_host, config.mqtt_broker_port).await {
        Ok(client) => {
            info!("MQTT broker and client setup complete");
            client
        }
        Err(e) => {
            log::error!("Failed to setup MQTT broker: {}", e);
            log::warn!("Falling back to HTTP-only mode");
            // Create a dummy client as fallback
            use rumqttc::MqttOptions;
            let mqttoptions = MqttOptions::new("dummy-client", &config.mqtt_broker_host, config.mqtt_broker_port + 1);
            let (dummy_client, _) = rumqttc::AsyncClient::new(mqttoptions, 10);
            Arc::new(dummy_client)
        }
    };
    
    let state = web::Data::new(AppState {
        cache: Arc::new(Mutex::new(None)),
        last_fetch: Arc::new(Mutex::new(SystemTime::now())),
        client: Client::new(),
        api_key: config.api_key,
        mqtt_client,
        historical_cache: Arc::new(Mutex::new(HashMap::new())),
        update_interval_seconds: config.update_interval_seconds,
        cmc_mapping: Arc::new(Mutex::new(HashMap::new())),
        logo_cache: Arc::new(Mutex::new(HashMap::new())),
    });
    
    // Setup MQTT request handling now that AppState is created
    if let Err(e) = setup_mqtt_request_handling(state.clone()).await {
        log::error!("Failed to setup MQTT request handling: {}", e);
        log::warn!("MQTT requests will not be processed");
    }
    
    // Fetch CMC mapping at startup
    info!("Fetching CMC mapping data at startup...");
    if let Err(e) = fetch_cmc_mapping(state.clone()).await {
        error!("Failed to fetch CMC mapping at startup: {}", e);
        info!("Server will start with empty mapping - mappings can be updated later");
    }
    
    let state_clone = state.clone();
    tokio::spawn(async move {
        fetch_data_periodically(state_clone).await;
    });
    
    // Spawn historical data publishing task (every hour)
    let state_clone_hist = state.clone();
    tokio::spawn(async move {
        clear_mqtt_cache_periodically(state_clone_hist).await;
    });
    
    info!("Starting crypto market data server on http://127.0.0.1:{}", config.http_icon_port);
    info!("MQTT broker listening on {}:{}", config.mqtt_broker_host, config.mqtt_broker_port);
    info!("MQTT broker console on 127.0.0.1:3030");
    info!("Ready to accept connections...");
    
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(Logger::default())
            .service(get_prices)
            .service(health_check)
            .service(get_historical_data)
            .service(get_cmc_mapping)
            .service(get_crypto_logo)
    })
    .bind(("0.0.0.0", config.http_icon_port))?
    .run()
    .await
}