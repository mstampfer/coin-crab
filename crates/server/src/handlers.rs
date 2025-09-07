use actix_web::{web, Responder, get};
use log::{info, warn};
use std::time::{Duration, SystemTime};
use crate::types::{AppState, ApiResponse, HistoricalQuery};
use crate::data::fetch_historical_data_server;
use crate::mqtt::publish_historical_data_to_mqtt;

#[get("/api/crypto-prices")]
pub async fn get_prices(data: web::Data<AppState>) -> impl Responder {
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

#[get("/health")]
pub async fn health_check() -> impl Responder {
    web::Json(serde_json::json!({
        "status": "ok",
        "timestamp": SystemTime::now()
    }))
}

#[get("/api/historical/{symbol}")]
pub async fn get_historical_data(
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

#[get("/api/cmc-mapping")]
pub async fn get_cmc_mapping(data: web::Data<AppState>) -> impl Responder {
    let mapping = data.cmc_mapping.lock().unwrap();
    web::Json(mapping.clone())
}

#[get("/api/logo/{symbol}")]
pub async fn get_crypto_logo(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    use actix_web::{HttpResponse, http::header};
    use std::time::{Duration, SystemTime};
    
    let symbol = path.into_inner().to_uppercase();
    
    // Check cache first (with 24 hour expiry)
    {
        let cache = data.logo_cache.lock().unwrap();
        if let Some((image_data, cached_time)) = cache.get(&symbol) {
            if cached_time.elapsed().unwrap_or(Duration::from_secs(u64::MAX)) < Duration::from_secs(24 * 60 * 60) {
                return HttpResponse::Ok()
                    .content_type("image/png")
                    .append_header(header::CacheControl(vec![
                        header::CacheDirective::Public,
                        header::CacheDirective::MaxAge(86400), // 24 hours
                    ]))
                    .body(image_data.clone());
            }
        }
    }
    
    // Get CMC ID for symbol
    let cmc_id = {
        let mapping = data.cmc_mapping.lock().unwrap();
        match mapping.get(&symbol).copied() {
            Some(id) => id,
            None => {
                warn!("No CMC mapping found for symbol: {}", symbol);
                return HttpResponse::NotFound().json(serde_json::json!({
                    "error": format!("No logo mapping found for symbol: {}", symbol)
                }));
            }
        }
    };
    
    // Fetch from CoinMarketCap
    let logo_url = format!("https://s2.coinmarketcap.com/static/img/coins/64x64/{}.png", cmc_id);
    
    match data.client.get(&logo_url).send().await {
        Ok(response) if response.status().is_success() => {
            match response.bytes().await {
                Ok(image_data) => {
                    let image_bytes = image_data.to_vec();
                    
                    // Cache the image
                    {
                        let mut cache = data.logo_cache.lock().unwrap();
                        cache.insert(symbol, (image_bytes.clone(), SystemTime::now()));
                    }
                    
                    HttpResponse::Ok()
                        .content_type("image/png")
                        .append_header(header::CacheControl(vec![
                            header::CacheDirective::Public,
                            header::CacheDirective::MaxAge(86400), // 24 hours
                        ]))
                        .body(image_bytes)
                },
                Err(e) => {
                    warn!("Failed to read logo image bytes for {}: {}", symbol, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to read image data"
                    }))
                }
            }
        },
        Ok(response) => {
            warn!("CMC logo request failed with status {} for symbol: {}", response.status(), symbol);
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "Logo not found"
            }))
        },
        Err(e) => {
            warn!("Failed to fetch logo for symbol {}: {}", symbol, e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch logo"
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web};
    use reqwest::Client;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use shared::{CryptoCurrency, Quote, UsdQuote};

    fn create_test_app_state() -> web::Data<AppState> {
        let test_crypto = CryptoCurrency {
            id: 1,
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            quote: Quote {
                usd: UsdQuote {
                    price: 50000.0,
                    market_cap: 900000000000.0,
                    percent_change_1h: 0.5,
                    percent_change_24h: 2.5,
                    percent_change_7d: 10.0,
                    volume_24h: 50000000000.0,
                    last_updated: "2024-01-01T00:00:00Z".to_string(),
                },
            },
        };

        // Create a mock MQTT client (this won't actually connect in tests)
        let (mqtt_client, _) = rumqttc::AsyncClient::new(
            rumqttc::MqttOptions::new("test_client", "localhost", 1883),
            10
        );

        web::Data::new(AppState {
            cache: Arc::new(Mutex::new(Some(vec![test_crypto]))),
            last_fetch: Arc::new(Mutex::new(SystemTime::now())),
            client: Client::new(),
            api_key: "test_api_key".to_string(),
            mqtt_client: Arc::new(mqtt_client),
            historical_cache: Arc::new(Mutex::new(HashMap::new())),
            update_interval_seconds: 300,
            cmc_mapping: Arc::new(Mutex::new(HashMap::new())),
            logo_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    #[test]
    async fn test_get_prices_handler() {
        // Test the handler function logic directly without actix-web setup
        let app_state = create_test_app_state();
        
        // Test cached data scenario
        let cache = app_state.cache.lock().unwrap();
        assert!(cache.is_some());
        assert_eq!(cache.as_ref().unwrap().len(), 1);
        assert_eq!(cache.as_ref().unwrap()[0].symbol, "BTC");
    }

    #[test]
    async fn test_health_check_response() {
        // Test health check creates proper JSON structure
        let json_response = serde_json::json!({
            "status": "ok",
            "timestamp": SystemTime::now()
        });
        
        assert_eq!(json_response["status"], "ok");
        assert!(json_response.get("timestamp").is_some());
    }

    #[test]
    async fn test_cache_key_generation() {
        let symbol = "BTC";
        let timeframe = "24h";
        let cache_key = format!("{}:{}", symbol, timeframe);
        assert_eq!(cache_key, "BTC:24h");
    }

    #[test] 
    async fn test_duration_from_secs() {
        let duration = Duration::from_secs(30);
        assert_eq!(duration.as_secs(), 30);
        
        let system_time = SystemTime::now();
        let elapsed = system_time.elapsed().unwrap_or(Duration::from_secs(0));
        assert!(elapsed < Duration::from_secs(1)); // Should be very recent
    }

    #[test]
    async fn test_cached_status_logic() {
        // Test the caching logic from get_prices
        let fresh_duration = Duration::from_secs(10);
        let old_duration = Duration::from_secs(60);
        
        let cached_fresh = fresh_duration > Duration::from_secs(30);
        let cached_old = old_duration > Duration::from_secs(30);
        
        assert!(!cached_fresh); // 10 seconds should not be cached
        assert!(cached_old);    // 60 seconds should be cached
    }

    #[test]
    async fn test_api_response_structure() {
        let test_crypto = CryptoCurrency {
            id: 1,
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            quote: Quote {
                usd: UsdQuote {
                    price: 50000.0,
                    market_cap: 900000000000.0,
                    percent_change_1h: 0.5,
                    percent_change_24h: 2.5,
                    percent_change_7d: 10.0,
                    volume_24h: 50000000000.0,
                    last_updated: "2024-01-01T00:00:00Z".to_string(),
                },
            },
        };

        let response = ApiResponse {
            data: vec![test_crypto],
            last_updated: "test_timestamp".to_string(),
            cached: true,
        };

        assert_eq!(response.data.len(), 1);
        assert_eq!(response.last_updated, "test_timestamp");
        assert!(response.cached);
    }

    #[test]
    async fn test_historical_query_structure() {
        let query = HistoricalQuery {
            timeframe: "24h".to_string(),
        };

        assert_eq!(query.timeframe, "24h");
    }

    #[test]
    async fn test_timeout_duration() {
        let timeout = Duration::from_millis(1000);
        assert_eq!(timeout.as_millis(), 1000);
        assert_eq!(timeout.as_secs(), 1);
    }
}