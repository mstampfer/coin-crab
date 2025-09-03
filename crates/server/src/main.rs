use actix_web::{web, App, HttpServer, Responder, get, middleware::Logger};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, Duration};
use tokio::time;
use log::{debug, info, warn, error};
// MQTT imports
use rumqttd::{Broker, Config as BrokerConfig};
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use std::collections::HashMap;
use std::thread;
use std::path::Path;
// Import shared data structures
use shared::{CryptoCurrency, Quote, UsdQuote, HistoricalDataPoint, HistoricalDataResult};


#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinMarketCapResponse {
    data: Vec<CryptoCurrency>,
}

#[derive(Debug, Clone, Serialize)]
struct ApiResponse {
    data: Vec<CryptoCurrency>,
    last_updated: String,
    cached: bool,
}

struct AppState {
    cache: Arc<Mutex<Option<Vec<CryptoCurrency>>>>,
    last_fetch: Arc<Mutex<SystemTime>>,
    client: Client,
    api_key: String,
    mqtt_client: Arc<AsyncClient>,
    historical_cache: Arc<Mutex<HashMap<String, (HistoricalDataResult, SystemTime)>>>,
}

#[get("/api/crypto-prices")]
async fn get_prices(data: web::Data<AppState>) -> impl Responder {
    let cache = data.cache.lock().unwrap();
    let last_fetch = data.last_fetch.lock().unwrap();
    
    match cache.as_ref() {
        Some(crypto_data) => {
            let age = last_fetch.elapsed().unwrap_or(Duration::from_secs(0));
            let cached = age > Duration::from_secs(30);
            
            let response = ApiResponse {
                data: crypto_data.clone(),
                last_updated: format!("{:?}", *last_fetch),
                cached,
            };
            
            web::Json(response)
        }
        None => {
            warn!("No cached data available");
            let response = ApiResponse {
                data: vec![],
                last_updated: "Never".to_string(),
                cached: false,
            };
            web::Json(response)
        }
    }
}

async fn fetch_data_periodically(state: web::Data<AppState>) {
    let mut interval = time::interval(Duration::from_secs(15));
    
    loop {
        interval.tick().await;
        
        info!("Fetching data from CoinMarketCap API");
        info!("Using API key: {}...", &state.api_key[..8.min(state.api_key.len())]);
        
        let response = state.client
            .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest")
            .query(&[("limit", "100"), ("convert", "USD")])
            .header("X-CMC_PRO_API_KEY", &state.api_key)
            .header("Accept", "application/json")
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<CoinMarketCapResponse>().await {
                        Ok(cmc_data) => {
                            info!("Successfully fetched {} cryptocurrencies", cmc_data.data.len());
                            
                            // Clone data for MQTT publishing before updating cache
                            let crypto_data_for_mqtt = cmc_data.data.clone();
                            
                            // Update cache (scoped to release locks before await)
                            {
                                let mut cache = state.cache.lock().unwrap();
                                *cache = Some(cmc_data.data);
                                
                                let mut last_fetch = state.last_fetch.lock().unwrap();
                                *last_fetch = SystemTime::now();
                            }
                            
                            // Publish to MQTT (locks are now released) - ignore errors for now
                            let _ = tokio::time::timeout(
                                Duration::from_millis(100),
                                publish_crypto_data_to_mqtt(&state.mqtt_client, &crypto_data_for_mqtt)
                            ).await;
                        }
                        Err(e) => {
                            error!("Failed to parse CoinMarketCap response: {}", e);
                        }
                    }
                } else {
                    error!("CoinMarketCap API returned status: {}", status);
                    if let Ok(error_text) = resp.text().await {
                        error!("Error response: {}", error_text);
                    }
                    if status.as_u16() == 429 {
                        warn!("Rate limit reached, using cached data");
                    } else if status.as_u16() == 401 {
                        error!("API key authentication failed - check your CMC_API_KEY");
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch data from CoinMarketCap: {}", e);
            }
        }
    }
}

async fn clear_mqtt_cache_periodically(state: web::Data<AppState>) {
    info!("Starting periodic MQTT cache clearing task");
    
    // Wait 5 minutes before starting periodic cache clearing
    tokio::time::sleep(Duration::from_secs(300)).await;
    
    let timeframes = ["1h", "24h", "7d", "30d", "90d", "365d"];
    let symbols = ["BTC", "ETH", "ADA", "SOL", "DOT", "MATIC", "LINK", "XRP", "LTC", "BCH"];
    
    loop {
        for &timeframe in &timeframes {
            // Get the update interval for this timeframe
            let interval_secs = match timeframe {
                "1h" => 300,    // 5 minutes
                "24h" => 3600,  // 1 hour  
                "7d" => 7200,   // 2 hours
                "30d" => 21600, // 6 hours
                "90d" => 86400, // 1 day
                "365d" => 86400, // 1 day
                _ => 3600,
            };
            
            info!("Clearing MQTT cache for timeframe {} (interval: {}s)", timeframe, interval_secs);
            
            // Clear MQTT retained messages for this timeframe
            for &symbol in &symbols {
                let topic = format!("crypto/historical/{}/{}", symbol, timeframe);
                
                // Publish empty retained message to clear the topic
                if let Err(_) = tokio::time::timeout(
                    Duration::from_millis(1000),
                    publish_empty_retained_message(&state.mqtt_client, &topic)
                ).await {
                    warn!("Timeout clearing MQTT cache for {}", topic);
                }
            }
            
            info!("Cleared MQTT cache for timeframe {}, next clear in {}s", timeframe, interval_secs);
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    }
}

async fn publish_empty_retained_message(mqtt_client: &AsyncClient, topic: &str) {
    match mqtt_client.publish(topic, rumqttc::QoS::AtLeastOnce, true, "").await {
        Ok(_) => info!("Cleared MQTT retained message for topic: {}", topic),
        Err(e) => warn!("Failed to clear MQTT retained message for {}: {}", topic, e),
    }
}

async fn publish_initial_priority_data(state: &web::Data<AppState>) {
    // Only fetch data for the most popular cryptocurrencies to avoid rate limits
    let priority_symbols = ["BTC", "ETH"];
    let priority_timeframes = ["24h", "7d"]; // Most commonly used timeframes
    
    info!("Fetching priority historical data to avoid rate limits on startup");
    
    let mut failed_requests = Vec::new();
    
    for &symbol in &priority_symbols {
        for &timeframe in &priority_timeframes {
            info!("Fetching and publishing initial historical data for {} {}", symbol, timeframe);
            
            match fetch_historical_data_server(symbol, timeframe, &state.api_key, &state.client).await {
                result if result.success => {
                    // Cache the result
                    let cache_key = format!("{}:{}", symbol, timeframe);
                    {
                        let mut hist_cache = state.historical_cache.lock().unwrap();
                        hist_cache.insert(cache_key, (result.clone(), SystemTime::now()));
                    }
                    
                    // Publish to MQTT with retain=true for immediate availability
                    if let Err(_) = tokio::time::timeout(
                        Duration::from_millis(1000),
                        publish_historical_data_to_mqtt(&state.mqtt_client, symbol, timeframe, &result)
                    ).await {
                        warn!("MQTT publish timeout for initial {} {}", symbol, timeframe);
                    }
                }
                result => {
                    warn!("Failed to fetch initial historical data for {} {}: {:?}", symbol, timeframe, result.error);
                    failed_requests.push((symbol, timeframe));
                }
            }
            
            // Small delay between requests to avoid rate limiting
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    
    // Retry failed requests after a longer delay if there are any
    if !failed_requests.is_empty() {
        info!("Retrying {} failed historical data requests after rate limit cooldown", failed_requests.len());
        tokio::time::sleep(Duration::from_secs(70)).await; // Wait for rate limit to reset
        
        for (symbol, timeframe) in failed_requests {
            info!("Retrying historical data for {} {}", symbol, timeframe);
            
            match fetch_historical_data_server(symbol, timeframe, &state.api_key, &state.client).await {
                result if result.success => {
                    // Cache the result
                    let cache_key = format!("{}:{}", symbol, timeframe);
                    {
                        let mut hist_cache = state.historical_cache.lock().unwrap();
                        hist_cache.insert(cache_key, (result.clone(), SystemTime::now()));
                    }
                    
                    // Publish to MQTT with retain=true for immediate availability
                    if let Err(_) = tokio::time::timeout(
                        Duration::from_millis(1000),
                        publish_historical_data_to_mqtt(&state.mqtt_client, symbol, timeframe, &result)
                    ).await {
                        warn!("MQTT publish timeout for retry {} {}", symbol, timeframe);
                    }
                    info!("Successfully published historical data for {} {} on retry", symbol, timeframe);
                }
                result => {
                    warn!("Retry also failed for {} {}: {:?}", symbol, timeframe, result.error);
                }
            }
            
            // Longer delay between retries to be more cautious
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    
    info!("Completed initial historical data publishing");
}

#[get("/health")]
async fn health_check() -> impl Responder {
    web::Json(serde_json::json!({
        "status": "ok",
        "timestamp": SystemTime::now()
    }))
}

#[derive(Deserialize)]
struct HistoricalQuery {
    timeframe: String,
}

#[get("/api/historical/{symbol}")]
async fn get_historical_data(
    path: web::Path<String>,
    query: web::Query<HistoricalQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let symbol = path.into_inner();
    let timeframe = &query.timeframe;
    
    info!("Historical data request: {} with timeframe {}", symbol, timeframe);
    
    // Implement the actual CMC historical data fetching
    let result = fetch_historical_data_server(&symbol, &timeframe, &data.api_key, &data.client).await;
    
    // Cache the result and publish to MQTT for future requests
    let cache_key = format!("{}:{}", symbol, timeframe);
    {
        let mut hist_cache = data.historical_cache.lock().unwrap();
        hist_cache.insert(cache_key.clone(), (result.clone(), SystemTime::now()));
    }
    
    // Publish to MQTT so subsequent requests can use MQTT instead of HTTP
    if result.success {
        info!("Publishing {} {} to MQTT for caching", symbol, timeframe);
        if let Err(_) = tokio::time::timeout(
            Duration::from_millis(1000),
            publish_historical_data_to_mqtt(&data.mqtt_client, &symbol, &timeframe, &result)
        ).await {
            warn!("MQTT publish timeout for {} {}", symbol, timeframe);
        }
    }
    
    web::Json(result)
}

// Helper functions for historical data processing
fn get_start_time(days: u32) -> String {
    let now = chrono::Utc::now();
    let start_time = now - chrono::Duration::days(days as i64);
    start_time.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string()
}

fn get_current_time() -> String {
    let now = chrono::Utc::now();
    now.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string()
}

fn get_interval_for_timeframe(timeframe: &str) -> &str {
    match timeframe {
        "1h" => "5m",
        "24h" | "1d" => "1h",
        "7d" => "2h",
        "30d" => "6h",
        "90d" => "1d",  // Use daily intervals for 90d
        "365d" | "1y" => "1d",
        "all" => "1d",  // Use daily intervals for all time
        _ => "1h",
    }
}

async fn fetch_historical_data_server(
    symbol: &str, 
    timeframe: &str, 
    api_key: &str, 
    client: &Client
) -> HistoricalDataResult {
    let symbol = symbol.to_uppercase();
    
    // Convert timeframe to days for CMC API
    let days = match timeframe {
        "1h" => 1,
        "24h" | "1d" => 1,
        "7d" => 7,
        "30d" => 30,
        "90d" => 90,
        "365d" | "1y" => 365,
        "all" => 365,  // Limit "all" to 1 year due to CMC API constraints
        _ => 30,
    };
    
    info!("Fetching historical data for {} with timeframe {} ({} days)", symbol, timeframe, days);
    
    // First get the cryptocurrency ID from symbol
    let quotes_url = format!(
        "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}&convert=USD",
        symbol
    );
    
    let crypto_id = match client
        .get(&quotes_url)
        .header("X-CMC_PRO_API_KEY", api_key)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(data) = json.get("data").and_then(|d| d.get(&symbol)) {
                            if let Some(id) = data.get("id").and_then(|id| id.as_u64()) {
                                id
                            } else {
                                return HistoricalDataResult {
                                    success: false,
                                    data: Vec::new(),
                                    error: Some("Could not find cryptocurrency ID".to_string()),
                                    symbol: Some(symbol),
                                    timeframe: Some(timeframe.to_string()),
                                };
                            }
                        } else {
                            return HistoricalDataResult {
                                success: false,
                                data: Vec::new(),
                                error: Some("Invalid symbol or no data found".to_string()),
                                symbol: Some(symbol),
                                timeframe: Some(timeframe.to_string()),
                            };
                        }
                    }
                    Err(e) => {
                        return HistoricalDataResult {
                            success: false,
                            data: Vec::new(),
                            error: Some(format!("JSON parsing error: {}", e)),
                            symbol: Some(symbol),
                            timeframe: Some(timeframe.to_string()),
                        };
                    }
                }
            } else {
                return HistoricalDataResult {
                    success: false,
                    data: Vec::new(),
                    error: Some(format!("HTTP error getting crypto ID: {}", response.status())),
                    symbol: Some(symbol),
                    timeframe: Some(timeframe.to_string()),
                };
            }
        }
        Err(e) => {
            return HistoricalDataResult {
                success: false,
                data: Vec::new(),
                error: Some(format!("Network error getting crypto ID: {}", e)),
                symbol: Some(symbol),
                timeframe: Some(timeframe.to_string()),
            };
        }
    };
    
    // Now get historical data using the cryptocurrency ID
    let interval = get_interval_for_timeframe(timeframe);
    let start_time = get_start_time(days);
    let end_time = get_current_time();
    
    let historical_url = format!(
        "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/historical?id={}&time_start={}&time_end={}&interval={}",
        crypto_id,
        start_time,
        end_time,
        interval
    );
    
    info!("CMC API URL: {}", historical_url);
    
    match client
        .get(&historical_url)
        .header("X-CMC_PRO_API_KEY", api_key)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        let mut historical_points = Vec::new();
                        
                        if let Some(data) = json.get("data").and_then(|d| d.get("quotes").and_then(|q| q.as_array())) {
                            for quote in data {
                                if let (Some(timestamp_str), Some(price_data)) = (
                                    quote.get("timestamp").and_then(|t| t.as_str()),
                                    quote.get("quote").and_then(|q| q.get("USD"))
                                ) {
                                    if let (Ok(timestamp), Some(price)) = (
                                        chrono::DateTime::parse_from_rfc3339(timestamp_str),
                                        price_data.get("price").and_then(|p| p.as_f64())
                                    ) {
                                        historical_points.push(HistoricalDataPoint {
                                            timestamp: timestamp.timestamp() as f64,
                                            price,
                                            volume: price_data.get("volume_24h").and_then(|v| v.as_f64()),
                                        });
                                    }
                                }
                            }
                        }
                        
                        if historical_points.is_empty() {
                            HistoricalDataResult {
                                success: false,
                                data: Vec::new(),
                                error: Some("No historical data points found".to_string()),
                                symbol: Some(symbol),
                                timeframe: Some(timeframe.to_string()),
                            }
                        } else {
                            info!("Successfully fetched {} historical data points", historical_points.len());
                            HistoricalDataResult {
                                success: true,
                                data: historical_points,
                                error: None,
                                symbol: Some(symbol),
                                timeframe: Some(timeframe.to_string()),
                            }
                        }
                    }
                    Err(e) => HistoricalDataResult {
                        success: false,
                        data: Vec::new(),
                        error: Some(format!("JSON parsing error: {}", e)),
                        symbol: Some(symbol),
                        timeframe: Some(timeframe.to_string()),
                    },
                }
            } else {
                HistoricalDataResult {
                    success: false,
                    data: Vec::new(),
                    error: Some(format!("HTTP error: {}", response.status())),
                    symbol: Some(symbol),
                    timeframe: Some(timeframe.to_string()),
                }
            }
        }
        Err(e) => HistoricalDataResult {
            success: false,
            data: Vec::new(),
            error: Some(format!("Network error: {}", e)),
            symbol: Some(symbol),
            timeframe: Some(timeframe.to_string()),
        },
    }
}

// MQTT Publishing Functions
async fn publish_crypto_data_to_mqtt(mqtt_client: &AsyncClient, crypto_data: &[CryptoCurrency]) {
    // Publish all crypto data to main topic with retention
    let payload = match serde_json::to_string(crypto_data) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize crypto data for MQTT: {}", e);
            return;
        }
    };
    
    if let Err(e) = mqtt_client.publish("crypto/prices/latest", QoS::AtLeastOnce, true, payload).await {
        error!("Failed to publish to crypto/prices/latest: {}", e);
    } else {
        info!("Published {} cryptocurrencies to MQTT topic crypto/prices/latest", crypto_data.len());
    }
    
    // Publish individual symbol updates
    for crypto in crypto_data {
        let individual_payload = match serde_json::to_string(crypto) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize {} for MQTT: {}", crypto.symbol, e);
                continue;
            }
        };
        
        let topic = format!("crypto/prices/{}", crypto.symbol);
        if let Err(e) = mqtt_client.publish(topic, QoS::AtLeastOnce, true, individual_payload).await {
            error!("Failed to publish to crypto/prices/{}: {}", crypto.symbol, e);
        }
    }
}

async fn publish_historical_data_to_mqtt(
    mqtt_client: &AsyncClient, 
    symbol: &str, 
    timeframe: &str, 
    data: &HistoricalDataResult
) {
    let payload = match serde_json::to_string(data) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize historical data for MQTT: {}", e);
            return;
        }
    };
    
    let topic = format!("crypto/historical/{}/{}", symbol.to_uppercase(), timeframe);
    
    // Use QoS 0 for historical data (less critical than live prices)
    // Set retain=true so clients get immediate data when subscribing
    if let Err(e) = mqtt_client.publish(&topic, QoS::AtMostOnce, true, payload).await {
        error!("Failed to publish historical data to {}: {}", topic, e);
    } else {
        info!("Published historical data for {} {} to MQTT", symbol, timeframe);
    }
}

// MQTT Broker Setup
async fn setup_mqtt_broker() -> Result<AsyncClient, String> {
    let broker_host = std::env::var("MQTT_BROKER_HOST").unwrap_or_else(|_| {
        warn!("MQTT_BROKER_HOST not set in .env file, using localhost (127.0.0.1)");
        "127.0.0.1".to_string()
    });
    
    info!("Starting embedded MQTT broker on {}:1883", broker_host);
    
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
    let mut mqttoptions = MqttOptions::new("crypto-server-publisher", &broker_host, 1883);
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
    
    Ok(client_clone)
}

// Setup MQTT request handling - call after AppState is created
async fn setup_mqtt_request_handling(state: web::Data<AppState>) -> Result<(), String> {
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
    let mut mqttoptions = MqttOptions::new("crypto-server-subscriber", &broker_host, 1883);
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file first
    dotenv::from_filename(".env.server").ok();
    
    // Initialize configurable logging with rumqttd suppression
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
    
    let mut builder = env_logger::Builder::from_default_env();
    
    // Set base log level from environment
    let level_filter = match log_level.to_uppercase().as_str() {
        "OFF" => log::LevelFilter::Off,
        "ERROR" => log::LevelFilter::Error,
        "WARN" => log::LevelFilter::Warn,
        "INFO" => log::LevelFilter::Info,
        "DEBUG" => log::LevelFilter::Debug,
        "TRACE" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };
    
    builder.filter_level(level_filter);
    
    // Always suppress rumqttd logs regardless of main log level
    builder.filter_module("rumqttd", log::LevelFilter::Off);
    builder.filter_module("rumqttd::router", log::LevelFilter::Off);
    builder.filter_module("rumqttd::router::routing", log::LevelFilter::Off);
    
    builder.init();
    
    let api_key = std::env::var("CMC_API_KEY")
        .unwrap_or_else(|_| {
            warn!("CMC_API_KEY environment variable not set, using placeholder");
            "YOUR_API_KEY_HERE".to_string()
        });
    
    // Setup MQTT broker and client  
    let mqtt_client = match setup_mqtt_broker().await {
        Ok(client) => {
            info!("MQTT broker and client setup complete");
            Arc::new(client)
        }
        Err(e) => {
            error!("Failed to setup MQTT broker: {}", e);
            warn!("Falling back to HTTP-only mode");
            // Create a dummy client as fallback
            let broker_host = std::env::var("MQTT_BROKER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let mqttoptions = MqttOptions::new("dummy-client", &broker_host, 1884);
            let (dummy_client, _) = AsyncClient::new(mqttoptions, 10);
            Arc::new(dummy_client)
        }
    };
    
    let state = web::Data::new(AppState {
        cache: Arc::new(Mutex::new(None)),
        last_fetch: Arc::new(Mutex::new(SystemTime::now())),
        client: Client::new(),
        api_key,
        mqtt_client,
        historical_cache: Arc::new(Mutex::new(HashMap::new())),
    });
    
    // Setup MQTT request handling now that AppState is created
    if let Err(e) = setup_mqtt_request_handling(state.clone()).await {
        error!("Failed to setup MQTT request handling: {}", e);
        warn!("MQTT requests will not be processed");
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
    
    info!("Starting crypto market data server on http://127.0.0.1:8080");
    info!("MQTT broker listening on 0.0.0.0:1883");
    info!("MQTT broker console on 127.0.0.1:3030");
    info!("Ready to accept connections...");
    
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(Logger::default())
            .service(get_prices)
            .service(health_check)
            .service(get_historical_data)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}