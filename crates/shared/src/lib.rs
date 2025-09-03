use serde::{Deserialize, Serialize};

// Shared data structures used by both server and iOS library

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoCurrency {
    pub id: i32,
    pub name: String,
    pub symbol: String,
    pub quote: Quote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    #[serde(rename = "USD")]
    pub usd: UsdQuote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdQuote {
    pub price: f64,
    pub percent_change_1h: f64,
    pub percent_change_24h: f64,
    pub percent_change_7d: f64,
    pub market_cap: f64,
    pub volume_24h: f64,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataPoint {
    pub timestamp: f64,
    pub price: f64,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataResult {
    pub success: bool,
    pub data: Vec<HistoricalDataPoint>,
    pub error: Option<String>,
    pub symbol: Option<String>,
    pub timeframe: Option<String>,
}

// Utility functions that can be shared between crates
pub fn debug_log(message: &str) {
    // Check environment variable for debug logging
    let enable_debug = std::env::var("ENABLE_DEBUG_LOGGING").unwrap_or_else(|_| "false".to_string());
    
    if enable_debug.to_lowercase() == "true" {
        println!("[DEBUG] {}", message);
        
        // Also write to a debug file if possible
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug.log")
        {
            use std::io::Write;
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
            writeln!(file, "[{}] {}", timestamp, message).ok();
        }
    }
}

pub fn init_logging() {
    // Initialize logging based on environment variables
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
    
    let level_filter = match log_level.to_uppercase().as_str() {
        "OFF" => log::LevelFilter::Off,
        "ERROR" => log::LevelFilter::Error,
        "WARN" => log::LevelFilter::Warn,
        "INFO" => log::LevelFilter::Info,
        "DEBUG" => log::LevelFilter::Debug,
        "TRACE" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };
    
    env_logger::Builder::from_default_env()
        .filter_level(level_filter)
        .init();
        
    debug_log("Logging system initialized");
}