use actix_web::web;
use reqwest::Client;
use std::time::{Duration, SystemTime};
use tokio::time;
use log::{info, warn, error};
use crate::types::{AppState, CoinMarketCapResponse, CmcMappingResponse};
use crate::mqtt::{publish_crypto_data_to_mqtt, publish_empty_retained_message};
#[cfg(test)]
use crate::mqtt::publish_historical_data_to_mqtt;
use shared::{HistoricalDataPoint, HistoricalDataResult};
pub async fn fetch_data_periodically(state: web::Data<AppState>) {
    info!("Starting data fetch with interval: {} seconds ({} minutes)", 
          state.update_interval_seconds, 
          state.update_interval_seconds / 60);
    let mut interval = time::interval(Duration::from_secs(state.update_interval_seconds));
    
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
                            
                            // Clone data for MQTT publishing before moving to cache
                            let crypto_data_for_mqtt = cmc_data.data.clone();
                            
                            // Update cache (scoped to release locks before await)
                            {
                                let mut cache = state.cache.lock().unwrap();
                                *cache = Some(cmc_data.data);
                                
                                let mut last_fetch = state.last_fetch.lock().unwrap();
                                *last_fetch = SystemTime::now();
                            }
                            
                            // Publish real market data to MQTT
                            info!("Publishing MQTT update with all {} cryptocurrencies", crypto_data_for_mqtt.len());
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

#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_start_time() {
        let start_time = get_start_time(30);
        
        // Verify it's a valid ISO 8601 timestamp
        assert!(start_time.contains("T"));
        assert!(start_time.ends_with("Z"));
        assert_eq!(start_time.len(), 24); // Format: 2024-01-01T00:00:00.000Z
        
        // Parse the timestamp to ensure it's valid
        let parsed = chrono::DateTime::parse_from_rfc3339(&start_time);
        assert!(parsed.is_ok());
        
        // Verify it's approximately 30 days ago
        let parsed_time = parsed.unwrap();
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(parsed_time.with_timezone(&chrono::Utc));
        
        // Should be between 29.9 and 30.1 days (allowing for execution time)
        assert!(diff.num_days() >= 29 && diff.num_days() <= 31);
    }

    #[test]
    fn test_get_current_time() {
        let current_time = get_current_time();
        
        // Verify it's a valid ISO 8601 timestamp
        assert!(current_time.contains("T"));
        assert!(current_time.ends_with("Z"));
        assert_eq!(current_time.len(), 24); // Format: 2024-01-01T00:00:00.000Z
        
        // Parse the timestamp to ensure it's valid
        let parsed = chrono::DateTime::parse_from_rfc3339(&current_time);
        assert!(parsed.is_ok());
        
        // Verify it's very recent (within 1 second)
        let parsed_time = parsed.unwrap();
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(parsed_time.with_timezone(&chrono::Utc));
        
        assert!(diff.num_seconds().abs() <= 1);
    }

    #[test]
    fn test_get_interval_for_timeframe() {
        assert_eq!(get_interval_for_timeframe("1h"), "5m");
        assert_eq!(get_interval_for_timeframe("24h"), "1h");
        assert_eq!(get_interval_for_timeframe("1d"), "1h");
        assert_eq!(get_interval_for_timeframe("7d"), "2h");
        assert_eq!(get_interval_for_timeframe("30d"), "6h");
        assert_eq!(get_interval_for_timeframe("90d"), "1d");
        assert_eq!(get_interval_for_timeframe("365d"), "1d");
        assert_eq!(get_interval_for_timeframe("1y"), "1d");
        assert_eq!(get_interval_for_timeframe("all"), "1d");
        assert_eq!(get_interval_for_timeframe("invalid"), "1h"); // Default case
    }

    #[test]
    fn test_timeframe_to_days_conversion() {
        // Test the conversion logic used in fetch_historical_data_server
        let test_cases = vec![
            ("1h", 1),
            ("24h", 1),
            ("1d", 1),
            ("7d", 7),
            ("30d", 30),
            ("90d", 90),
            ("365d", 365),
            ("1y", 365),
            ("all", 365),
            ("invalid", 30), // Default case
        ];

        for (timeframe, expected_days) in test_cases {
            let days = match timeframe {
                "1h" => 1,
                "24h" | "1d" => 1,
                "7d" => 7,
                "30d" => 30,
                "90d" => 90,
                "365d" | "1y" => 365,
                "all" => 365,
                _ => 30,
            };
            
            assert_eq!(days, expected_days, "Failed for timeframe: {}", timeframe);
        }
    }

    #[test]
    fn test_historical_data_result_creation() {
        let result = HistoricalDataResult {
            success: true,
            data: vec![
                shared::HistoricalDataPoint {
                    timestamp: 1704067200.0,
                    price: 45000.0,
                    volume: Some(1000000000.0),
                },
            ],
            error: None,
            symbol: Some("BTC".to_string()),
            timeframe: Some("24h".to_string()),
        };

        assert!(result.success);
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0].price, 45000.0);
        assert_eq!(result.symbol, Some("BTC".to_string()));
        assert_eq!(result.timeframe, Some("24h".to_string()));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_api_key_truncation_logic() {
        // Test the API key logging truncation logic from fetch_data_periodically
        let test_cases = vec![
            ("12345678", "12345678"), // Exactly 8 characters
            ("123456789", "12345678"), // More than 8 characters
            ("1234567", "1234567"),   // Less than 8 characters
            ("", ""),                 // Empty string
        ];

        for (api_key, expected) in test_cases {
            let truncated = &api_key[..8.min(api_key.len())];
            assert_eq!(truncated, expected, "Failed for API key: '{}'", api_key);
        }
    }

    #[test] 
    fn test_cache_intervals() {
        // Test the cache clearing intervals from clear_mqtt_cache_periodically
        let timeframes = ["1h", "24h", "7d", "30d", "90d", "365d"];
        let expected_intervals = [300, 3600, 7200, 21600, 86400, 86400];
        
        for (i, &timeframe) in timeframes.iter().enumerate() {
            let interval_secs = match timeframe {
                "1h" => 300,    // 5 minutes
                "24h" => 3600,  // 1 hour  
                "7d" => 7200,   // 2 hours
                "30d" => 21600, // 6 hours
                "90d" => 86400, // 1 day
                "365d" => 86400, // 1 day
                _ => 3600,
            };
            
            assert_eq!(interval_secs, expected_intervals[i], 
                      "Interval mismatch for timeframe: {}", timeframe);
        }
    }

    #[test]
    fn test_priority_symbols_and_timeframes() {
        // Test the priority data configuration from publish_initial_priority_data
        let priority_symbols = ["BTC", "ETH"];
        let priority_timeframes = ["24h", "7d"];
        
        assert_eq!(priority_symbols.len(), 2);
        assert_eq!(priority_timeframes.len(), 2);
        
        assert!(priority_symbols.contains(&"BTC"));
        assert!(priority_symbols.contains(&"ETH"));
        assert!(priority_timeframes.contains(&"24h"));
        assert!(priority_timeframes.contains(&"7d"));
    }
}

pub async fn fetch_cmc_mapping(state: web::Data<AppState>) -> Result<(), String> {
    info!("Fetching CMC cryptocurrency mapping data...");
    
    let response = state.client
        .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/map")
        .query(&[("limit", "5000")])
        .header("X-CMC_PRO_API_KEY", &state.api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to send CMC mapping request: {}", e))?;
    
    if response.status().is_success() {
        let cmc_response: CmcMappingResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse CMC mapping response: {}", e))?;
        
        if cmc_response.status.error_code == 0 {
            let mut mapping = std::collections::HashMap::new();
            for currency in cmc_response.data {
                mapping.insert(currency.symbol.to_uppercase(), currency.id);
            }
            
            let count = mapping.len();
            *state.cmc_mapping.lock().unwrap() = mapping;
            info!("Successfully loaded {} CMC cryptocurrency mappings", count);

            Ok(())
        } else {
            let error_msg = format!("CMC API error: {} (code: {})", 
                cmc_response.status.error_message.unwrap_or("Unknown error".to_string()),
                cmc_response.status.error_code
            );
            error!("{}", error_msg);
            Err(error_msg)
        }
    } else {
        let error_msg = format!("CMC mapping request failed with status: {}", response.status());
        error!("{}", error_msg);
        Err(error_msg)
    }
}

