#!/usr/bin/env rust-script
//! Test all timeframes to make sure they work after endpoint fixes

use std::ffi::{CStr, CString};

extern "C" {
    fn get_historical_crypto_data(endpoint: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
}

fn test_timeframe(timeframe: &str, symbol: &str) {
    println!("\n🔍 Testing: {} for {}", timeframe, symbol.to_uppercase());
    
    dotenv::dotenv().ok();
    let api_key = std::env::var("CMC_API_KEY").expect("CMC_API_KEY environment variable is required");
    let endpoint = format!("cmc://historical/{}?timeframe={}&api_key={}", symbol, timeframe, api_key);
    let endpoint_cstring = CString::new(endpoint).unwrap();
    
    let result_ptr = unsafe { get_historical_crypto_data(endpoint_cstring.as_ptr()) };
    
    if result_ptr.is_null() {
        println!("   ❌ Function returned null");
        return;
    }
    
    let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
    let result_string = result_cstr.to_string_lossy();
    
    match serde_json::from_str::<serde_json::Value>(&result_string) {
        Ok(json) => {
            if let Some(success) = json.get("success") {
                if success.as_bool() == Some(true) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("   ✅ Success: {} data points", data.len());
                        
                        if let Some(first) = data.first() {
                            if let Some(price) = first.get("price").and_then(|p| p.as_f64()) {
                                println!("   💰 First price: ${:.2}", price);
                            }
                        }
                    }
                } else {
                    println!("   ❌ Success=false");
                    if let Some(error) = json.get("error") {
                        println!("      Error: {}", error);
                    }
                }
            }
        }
        Err(e) => {
            println!("   ❌ JSON parsing failed: {}", e);
            if result_string.len() < 200 {
                println!("   Response: {}", result_string);
            }
        }
    }
    
    // Free memory
    unsafe { libc::free(result_ptr as *mut libc::c_void) };
}

fn main() {
    println!("🧪 Testing all iOS timeframes after endpoint fixes...");
    
    let timeframes = ["1h", "24h", "7d", "30d", "90d", "365d", "all"];
    let symbols = ["btc", "eth"];
    
    for symbol in symbols {
        println!("\n" + "=".repeat(50).as_str());
        println!("Testing {} ({})", symbol.to_uppercase(), match symbol {
            "btc" => "Bitcoin",
            "eth" => "Ethereum", 
            _ => "Unknown"
        });
        println!("=" + "=".repeat(49).as_str());
        
        for timeframe in timeframes {
            test_timeframe(timeframe, symbol);
        }
    }
    
    println!("\n🎉 Testing complete!");
}