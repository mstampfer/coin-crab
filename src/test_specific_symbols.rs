fn get_test_api_key() -> String {
    dotenv::dotenv().ok();
    std::env::var("CMC_API_KEY").unwrap_or_else(|_| {
        panic!("CMC_API_KEY environment variable is required for tests. Please set it in .env file")
    })
}

#[test]
fn test_btc_vs_eth_direct_comparison() {
    println!("🧪 Direct comparison: BTC vs ETH");
    
    let symbols = vec![
        ("btc", "Bitcoin"),
        ("eth", "Ethereum")
    ];
    
    for (symbol, name) in symbols {
        println!("\n🔍 Testing {}: {}", symbol.to_uppercase(), name);
        
        // Test the exact same parameters for both
        let api_key = get_test_api_key();
        let endpoint = format!("cmc://historical/{}?timeframe=24h&api_key={}", symbol, api_key);
        
        // Use the C function directly to simulate iOS call
        let endpoint_cstring = std::ffi::CString::new(endpoint).unwrap();
        let result_ptr = crate::get_historical_crypto_data(endpoint_cstring.as_ptr());
        
        if result_ptr.is_null() {
            println!("   ❌ Function returned null!");
            continue;
        }
        
        let result_cstr = unsafe { std::ffi::CStr::from_ptr(result_ptr) };
        let result_string = result_cstr.to_string_lossy();
        
        // Print first 200 characters to see the structure
        println!("   Response start: {}", &result_string[..200.min(result_string.len())]);
        
        // Try to parse as JSON
        match serde_json::from_str::<serde_json::Value>(&result_string) {
            Ok(json) => {
                if let Some(success) = json.get("success") {
                    println!("   Success: {}", success);
                    
                    if let Some(data) = json.get("data") {
                        if let Some(array) = data.as_array() {
                            println!("   Data points: {}", array.len());
                            
                            // Check first data point structure
                            if let Some(first_point) = array.first() {
                                println!("   First point: {}", first_point);
                                
                                // Check required fields
                                if let (Some(timestamp), Some(price)) = 
                                   (first_point.get("timestamp"), first_point.get("price")) {
                                    println!("   ✅ Has timestamp and price fields");
                                } else {
                                    println!("   ❌ Missing timestamp or price fields");
                                }
                            }
                        }
                    }
                } else {
                    println!("   ❌ No success field in response");
                }
            }
            Err(e) => {
                println!("   ❌ JSON parsing failed: {}", e);
                println!("   Raw response length: {}", result_string.len());
            }
        }
        
        // Free memory
        unsafe { libc::free(result_ptr as *mut libc::c_void) };
    }
}

#[test]
fn test_different_symbol_formats() {
    println!("🧪 Testing different symbol formats and edge cases");
    
    let test_cases = vec![
        ("btc", "Standard BTC"),
        ("eth", "Standard ETH"), 
        ("sol", "Standard SOL"),
        ("BTC", "Uppercase BTC"),
        ("invalid", "Invalid symbol"),
    ];
    
    for (symbol, description) in test_cases {
        println!("\n🔍 Testing: {} ({})", symbol, description);
        
        let api_key = get_test_api_key();
        let endpoint = format!("cmc://historical/{}?timeframe=24h&api_key={}", symbol, api_key);
        let endpoint_cstring = std::ffi::CString::new(endpoint).unwrap();
        let result_ptr = crate::get_historical_crypto_data(endpoint_cstring.as_ptr());
        
        if result_ptr.is_null() {
            println!("   ❌ Null response");
            continue;
        }
        
        let result_cstr = unsafe { std::ffi::CStr::from_ptr(result_ptr) };
        let result_string = result_cstr.to_string_lossy();
        
        match serde_json::from_str::<serde_json::Value>(&result_string) {
            Ok(json) => {
                if let Some(success) = json.get("success") {
                    if success.as_bool() == Some(true) {
                        println!("   ✅ Success");
                    } else {
                        println!("   ❌ Success=false");
                        if let Some(error) = json.get("error") {
                            println!("      Error: {}", error);
                        }
                    }
                } else {
                    println!("   ❌ Invalid JSON structure");
                }
            }
            Err(_) => {
                println!("   ❌ JSON parsing failed");
                // Check if it might be an error response
                if result_string.contains("error") || result_string.contains("Error") {
                    println!("   Error response: {}", &result_string[..100.min(result_string.len())]);
                }
            }
        }
        
        unsafe { libc::free(result_ptr as *mut libc::c_void) };
    }
}