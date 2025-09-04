use actix_web::web;
use reqwest::Client;
use std::time::{Duration, SystemTime};
use tokio::time;
use log::{info, warn, error};
use crate::types::{AppState, CoinMarketCapResponse};
use crate::mqtt::{publish_crypto_data_to_mqtt, publish_historical_data_to_mqtt, publish_empty_retained_message};
use shared::{HistoricalDataPoint, HistoricalDataResult};

pub async fn fetch_data_periodically(state: web::Data<AppState>) {
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

pub async fn clear_mqtt_cache_periodically(state: web::Data<AppState>) {
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

pub async fn publish_initial_priority_data(state: &web::Data<AppState>) {
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

pub async fn fetch_historical_data_server(
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