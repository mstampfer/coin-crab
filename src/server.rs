use actix_web::{web, App, HttpServer, Responder, get, middleware::Logger};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, Duration};
use tokio::time;
use log::{info, warn, error};
// Historical data structures - duplicated from lib.rs for server use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataPoint {
    pub timestamp: f64,
    pub price: f64,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoricalDataResult {
    pub success: bool,
    pub data: Vec<HistoricalDataPoint>,
    pub error: Option<String>,
    pub symbol: Option<String>,
    pub timeframe: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CryptoCurrency {
    id: i32,
    name: String,
    symbol: String,
    quote: Quote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Quote {
    #[serde(rename = "USD")]
    usd: UsdQuote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsdQuote {
    price: f64,
    percent_change_1h: f64,
    percent_change_24h: f64,
    percent_change_7d: f64,
    market_cap: f64,
    volume_24h: f64,
    last_updated: String,
}

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
                            
                            let mut cache = state.cache.lock().unwrap();
                            *cache = Some(cmc_data.data);
                            
                            let mut last_fetch = state.last_fetch.lock().unwrap();
                            *last_fetch = SystemTime::now();
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
        "90d" => "1d",  // Use daily intervals for 90d (maps to 365d data)
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
        "90d" => 365,  // CMC doesn't support 90d directly, use 1 year
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    
    // Load .env file
    dotenv::dotenv().ok();
    
    let api_key = std::env::var("CMC_API_KEY")
        .unwrap_or_else(|_| {
            warn!("CMC_API_KEY environment variable not set, using placeholder");
            "YOUR_API_KEY_HERE".to_string()
        });
    
    let state = web::Data::new(AppState {
        cache: Arc::new(Mutex::new(None)),
        last_fetch: Arc::new(Mutex::new(SystemTime::now())),
        client: Client::new(),
        api_key,
    });
    
    let state_clone = state.clone();
    tokio::spawn(async move {
        fetch_data_periodically(state_clone).await;
    });
    
    info!("Starting crypto market data server on http://127.0.0.1:8080");
    
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(Logger::default())
            .service(get_prices)
            .service(health_check)
            .service(get_historical_data)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}