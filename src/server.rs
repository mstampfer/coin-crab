use actix_web::{web, App, HttpServer, Responder, get, middleware::Logger};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, Duration};
use tokio::time;
use log::{info, warn, error};

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
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}