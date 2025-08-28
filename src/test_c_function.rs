use crate::get_historical_crypto_data;
use std::ffi::{CStr, CString};

fn get_test_api_key() -> String {
    dotenv::dotenv().ok();
    std::env::var("CMC_API_KEY").unwrap_or_else(|_| {
        panic!("CMC_API_KEY environment variable is required for tests. Please set it in .env file")
    })
}

#[test]
fn test_c_function_directly() {
    println!("🧪 Testing C function exactly as iOS would call it...");
    
    // Test the exact endpoint that iOS sends for SOL 24H
    let api_key = get_test_api_key();
    let endpoint = format!("cmc://historical/sol?timeframe=24h&interval=1h&api_key={}", api_key);
    let endpoint_cstring = CString::new(endpoint).unwrap();
    
    println!("   Endpoint: {}", endpoint);
    println!("   Calling get_historical_crypto_data...");
    
    // This is the exact call that iOS makes
    let result_ptr = get_historical_crypto_data(endpoint_cstring.as_ptr());
    
    if result_ptr.is_null() {
        println!("   ❌ Function returned NULL pointer - this is why iOS shows mock data!");
        return;
    }
    
    // Convert result back to string
    let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
    let result_string = result_cstr.to_string_lossy();
    
    println!("   ✅ Function returned data (length: {} chars)", result_string.len());
    println!("   First 200 chars: {}", &result_string[..200.min(result_string.len())]);
    
    // Free the string (this is important!)
    unsafe { libc::free(result_ptr as *mut libc::c_void) };
    
    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(&result_string) {
        Ok(json) => {
            println!("   ✅ JSON parsing successful");
            if let Some(success) = json.get("success") {
                println!("   Success field: {}", success);
            }
            if let Some(data) = json.get("data") {
                if let Some(array) = data.as_array() {
                    println!("   Data points: {}", array.len());
                } else {
                    println!("   Data is not an array: {}", data);
                }
            } else {
                println!("   No data field found");
            }
        }
        Err(e) => {
            println!("   ❌ JSON parsing failed: {}", e);
            println!("   Raw response: {}", result_string);
        }
    }
}

#[test] 
fn test_multiple_symbols() {
    println!("🧪 Testing multiple cryptocurrency symbols...");
    
    let symbols = vec![
        ("btc", "Bitcoin"),
        ("eth", "Ethereum"), 
        ("sol", "Solana"),
        ("ada", "Cardano")
    ];
    
    for (symbol, name) in symbols {
        println!("\n   Testing {}: {}", symbol.to_uppercase(), name);
        
        let endpoint = format!("cmc://historical/{}?timeframe=24h&api_key={}", get_test_api_key()", symbol);
        let endpoint_cstring = CString::new(endpoint).unwrap();
        
        let result_ptr = get_historical_crypto_data(endpoint_cstring.as_ptr());
        
        if result_ptr.is_null() {
            println!("     ❌ NULL returned");
            continue;
        }
        
        let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
        let result_string = result_cstr.to_string_lossy();
        
        // Free immediately after use
        unsafe { libc::free(result_ptr as *mut libc::c_void) };
        
        match serde_json::from_str::<serde_json::Value>(&result_string) {
            Ok(json) => {
                if let (Some(success), Some(data)) = (json.get("success"), json.get("data")) {
                    if success.as_bool() == Some(true) {
                        if let Some(array) = data.as_array() {
                            println!("     ✅ {} data points", array.len());
                        } else {
                            println!("     ⚠️  Success=true but data is not array");
                        }
                    } else {
                        println!("     ❌ Success=false");
                        if let Some(error) = json.get("error") {
                            println!("        Error: {}", error);
                        }
                    }
                } else {
                    println!("     ❌ Missing success/data fields");
                }
            }
            Err(e) => {
                println!("     ❌ JSON error: {}", e);
            }
        }
    }
}

#[test]
fn test_ios_data_structures() {
    println!("🧪 Testing if data structure matches iOS expectations...");
    
    let endpoint = "cmc://historical/btc?timeframe=24h&api_key={}", get_test_api_key()";
    let endpoint_cstring = CString::new(endpoint).unwrap();
    let result_ptr = get_historical_crypto_data(endpoint_cstring.as_ptr());
    
    if result_ptr.is_null() {
        println!("   ❌ Function returned null");
        return;
    }
    
    let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
    let result_string = result_cstr.to_string_lossy();
    unsafe { libc::free(result_ptr as *mut libc::c_void) };
    
    // Define the expected iOS structure
    #[derive(serde::Deserialize, Debug)]
    struct HistoricalDataPoint {
        timestamp: f64,
        price: f64,
    }
    
    #[derive(serde::Deserialize, Debug)]
    struct HistoricalDataResult {
        success: bool,
        data: Vec<HistoricalDataPoint>,
        error: Option<String>,
        symbol: Option<String>,
        timeframe: Option<String>,
    }
    
    match serde_json::from_str::<HistoricalDataResult>(&result_string) {
        Ok(result) => {
            println!("   ✅ Matches iOS structure exactly!");
            println!("   Success: {}", result.success);
            println!("   Data points: {}", result.data.len());
            println!("   Symbol: {:?}", result.symbol);
            println!("   Timeframe: {:?}", result.timeframe);
            
            if !result.data.is_empty() {
                let first = &result.data[0];
                println!("   First point: timestamp={:.0}, price=${:.2}", first.timestamp, first.price);
                
                // Check if timestamps are realistic (should be recent Unix timestamps)
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as f64;
                    
                if first.timestamp > (now - 86400.0 * 30.0) { // Within last 30 days
                    println!("   ✅ Timestamps look realistic");
                } else {
                    println!("   ⚠️  Timestamps might be too old: {} vs current {}", first.timestamp, now);
                }
            }
            
        }
        Err(e) => {
            println!("   ❌ Structure mismatch: {}", e);
            println!("   Raw JSON: {}", &result_string[..200.min(result_string.len())]);
        }
    }
}