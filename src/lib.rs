use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

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
pub struct ApiResponse {
    pub data: Vec<CryptoCurrency>,
    pub last_updated: String,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CryptoClientResult {
    pub success: bool,
    pub data: Option<Vec<CryptoCurrency>>,
    pub error: Option<String>,
    pub last_updated: Option<String>,
    pub cached: bool,
}

async fn fetch_crypto_data(endpoint: &str) -> CryptoClientResult {
    let client = reqwest::Client::new();
    
    match client.get(endpoint).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<ApiResponse>().await {
                    Ok(api_response) => CryptoClientResult {
                        success: true,
                        data: Some(api_response.data),
                        error: None,
                        last_updated: Some(api_response.last_updated),
                        cached: api_response.cached,
                    },
                    Err(e) => CryptoClientResult {
                        success: false,
                        data: None,
                        error: Some(format!("JSON parsing error: {}", e)),
                        last_updated: None,
                        cached: false,
                    },
                }
            } else {
                CryptoClientResult {
                    success: false,
                    data: None,
                    error: Some(format!("HTTP error: {}", response.status())),
                    last_updated: None,
                    cached: false,
                }
            }
        }
        Err(e) => CryptoClientResult {
            success: false,
            data: None,
            error: Some(format!("Network error: {}", e)),
            last_updated: None,
            cached: false,
        },
    }
}

async fn fetch_historical_data(endpoint: &str) -> HistoricalDataResult {
    let client = reqwest::Client::new();
    
    match client.get(endpoint).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<HistoricalApiResponse>().await {
                    Ok(api_response) => HistoricalDataResult {
                        success: true,
                        data: api_response.data,
                        error: None,
                        symbol: Some(api_response.symbol),
                        timeframe: Some(api_response.timeframe),
                    },
                    Err(e) => HistoricalDataResult {
                        success: false,
                        data: Vec::new(),
                        error: Some(format!("JSON parsing error: {}", e)),
                        symbol: None,
                        timeframe: None,
                    },
                }
            } else {
                HistoricalDataResult {
                    success: false,
                    data: Vec::new(),
                    error: Some(format!("HTTP error: {}", response.status())),
                    symbol: None,
                    timeframe: None,
                }
            }
        }
        Err(e) => HistoricalDataResult {
            success: false,
            data: Vec::new(),
            error: Some(format!("Network error: {}", e)),
            symbol: None,
            timeframe: None,
        },
    }
}

#[no_mangle]
pub extern "C" fn get_latest_crypto_prices(endpoint: *const c_char) -> *mut c_char {
    let c_str = unsafe { CStr::from_ptr(endpoint) };
    let endpoint_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return CString::new("Error: Invalid endpoint string").unwrap().into_raw(),
    };
    
    let rt = Runtime::new().unwrap();
    let result = rt.block_on(fetch_crypto_data(endpoint_str));
    
    match serde_json::to_string(&result) {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(e) => CString::new(format!("Error serializing result: {}", e)).unwrap().into_raw(),
    }
}

#[no_mangle]
pub extern "C" fn hello_rust_world() -> *mut c_char {
    let hello = CString::new("Hello, New Rust World!").unwrap();
    hello.into_raw()
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if s.is_null() { return }
        let _ = CString::from_raw(s);
    };
}