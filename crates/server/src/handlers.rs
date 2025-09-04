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