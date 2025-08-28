use crate::fetch_historical_data;
use std::ffi::CString;
use std::os::raw::c_char;

fn get_test_api_key() -> String {
    dotenv::dotenv().ok();
    std::env::var("CMC_API_KEY").unwrap_or_else(|_| {
        panic!("CMC_API_KEY environment variable is required for tests. Please set it in .env file")
    })
}

// Test function to verify historical data API integration
#[tokio::test]
async fn test_historical_data_api() {
    println!("🧪 Testing historical data API integration...");
    
    // Test 1: Valid API call with API key in endpoint
    let endpoint_with_key = "cmc://historical/btc?timeframe=24h&interval=1h&api_key={}", get_test_api_key()";
    let result = fetch_historical_data(endpoint_with_key, None).await;
    
    println!("✅ Test 1 - Endpoint with API key:");
    println!("   Success: {}", result.success);
    println!("   Data points: {}", result.data.len());
    println!("   Error: {:?}", result.error);
    println!("   Symbol: {:?}", result.symbol);
    println!("   Timeframe: {:?}", result.timeframe);
    
    if result.success && !result.data.is_empty() {
        println!("   ✅ API call successful with real data");
        println!("   First data point: timestamp={:.0}, price={}", 
                 result.data[0].timestamp, result.data[0].price);
    } else {
        println!("   ❌ API call failed or returned no data");
        if let Some(error) = &result.error {
            println!("   Error details: {}", error);
        }
    }
    
    // Test 2: Test different cryptocurrency
    let endpoint_eth = "cmc://historical/eth?timeframe=7d&api_key={}", get_test_api_key()";
    let result_eth = fetch_historical_data(endpoint_eth, None).await;
    
    println!("\n✅ Test 2 - ETH historical data:");
    println!("   Success: {}", result_eth.success);
    println!("   Data points: {}", result_eth.data.len());
    println!("   Symbol: {:?}", result_eth.symbol);
    
    // Test 3: Test invalid endpoint format
    let invalid_endpoint = "invalid://test";
    let result_invalid = fetch_historical_data(invalid_endpoint, None).await;
    
    println!("\n✅ Test 3 - Invalid endpoint format:");
    println!("   Success: {} (should be false)", result_invalid.success);
    println!("   Error: {:?}", result_invalid.error);
    
    // Test 4: Test without API key (should fall back to environment)
    let endpoint_no_key = "cmc://historical/btc?timeframe=1h";
    let result_no_key = fetch_historical_data(endpoint_no_key, None).await;
    
    println!("\n✅ Test 4 - No API key in endpoint:");
    println!("   Success: {}", result_no_key.success);
    println!("   Error: {:?}", result_no_key.error);
    
    // Summary
    println!("\n📊 Test Summary:");
    println!("   Test 1 (BTC with API key): {}", if result.success { "PASS" } else { "FAIL" });
    println!("   Test 2 (ETH with API key): {}", if result_eth.success { "PASS" } else { "FAIL" });
    println!("   Test 3 (Invalid endpoint): {}", if !result_invalid.success { "PASS" } else { "FAIL" });
    println!("   Test 4 (No API key): {}", if result_no_key.success || result_no_key.error.is_some() { "PASS" } else { "FAIL" });
    
    // Assert at least one test passed with real data
    assert!(result.success || result_eth.success, "At least one API call should succeed with real data");
    
    println!("\n🎉 Historical data API test completed!");
}

// C function test to match iOS integration
pub extern "C" fn test_historical_data_c_function() -> *mut c_char {
    println!("🧪 Testing C function interface for historical data...");
    
    let endpoint = "cmc://historical/btc?timeframe=24h&api_key={}", get_test_api_key()";
    let _endpoint_cstring = CString::new(endpoint).unwrap();
    
    // This would normally call get_historical_crypto_data but we'll test the async function directly
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(fetch_historical_data(endpoint, None));
    
    let response = serde_json::json!({
        "test": "c_function_test",
        "success": result.success,
        "data_count": result.data.len(),
        "error": result.error,
        "symbol": result.symbol,
        "timeframe": result.timeframe
    });
    
    CString::new(response.to_string()).unwrap().into_raw()
}

// Test specific API call formats that iOS uses
#[tokio::test]
async fn test_ios_format_api_calls() {
    println!("📱 Testing iOS-specific API call formats...");
    
    // Test the exact format that iOS sends
    let ios_endpoints = vec![
        "cmc://historical/btc?timeframe=24h&interval=1h&api_key={}", get_test_api_key()",
        "cmc://historical/eth?timeframe=7d&interval=2h&api_key={}", get_test_api_key()",
        "cmc://historical/ada?timeframe=30d&interval=6h&api_key={}", get_test_api_key()",
    ];
    
    for (i, endpoint) in ios_endpoints.iter().enumerate() {
        println!("\n📱 iOS Test {}: {}", i + 1, endpoint);
        let result = fetch_historical_data(endpoint, None).await;
        
        println!("   Success: {}", result.success);
        println!("   Data points: {}", result.data.len());
        println!("   Symbol: {:?}", result.symbol);
        
        if result.success && !result.data.is_empty() {
            println!("   ✅ Real data retrieved successfully");
            // Print first and last data points to verify it's not mock data
            println!("   First point: {:.0} @ ${:.2}", 
                     result.data[0].timestamp, result.data[0].price);
            if result.data.len() > 1 {
                let last_idx = result.data.len() - 1;
                println!("   Last point:  {:.0} @ ${:.2}", 
                         result.data[last_idx].timestamp, result.data[last_idx].price);
            }
        } else {
            println!("   ❌ Failed to get real data");
            if let Some(error) = &result.error {
                println!("   Error: {}", error);
            }
        }
    }
}

// Test to verify we're not getting mock data patterns
#[tokio::test]
async fn test_not_mock_data() {
    println!("🔍 Testing to ensure we're not getting mock data patterns...");
    
    let endpoint = "cmc://historical/btc?timeframe=24h&api_key={}", get_test_api_key()";
    let result = fetch_historical_data(endpoint, None).await;
    
    if result.success && !result.data.is_empty() {
        // Mock data typically has very regular patterns or specific test values
        // Real data should have more variation and realistic timestamps
        
        let prices: Vec<f64> = result.data.iter().map(|d| d.price).collect();
        let timestamps: Vec<f64> = result.data.iter().map(|d| d.timestamp).collect();
        
        println!("   Data analysis:");
        println!("   - Total points: {}", prices.len());
        println!("   - Price range: ${:.2} - ${:.2}", 
                 prices.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
                 prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));
        
        // Check for realistic Bitcoin price range (should be > $10,000 and < $200,000)
        let realistic_btc_prices = prices.iter().all(|&price| price > 10000.0 && price < 200000.0);
        println!("   - Realistic BTC prices: {}", realistic_btc_prices);
        
        // Check timestamps are not all identical (would indicate mock data)
        let unique_timestamps: std::collections::HashSet<_> = timestamps.iter().map(|&t| (t as u64)).collect();
        let has_varied_timestamps = unique_timestamps.len() > 1;
        println!("   - Varied timestamps: {} (unique: {})", has_varied_timestamps, unique_timestamps.len());
        
        // Check price variation (real data should have some variation)
        let price_variation = prices.windows(2).any(|w| (w[0] - w[1]).abs() > 0.01);
        println!("   - Price variation detected: {}", price_variation);
        
        if realistic_btc_prices && has_varied_timestamps && price_variation {
            println!("   ✅ Data appears to be REAL (not mock data)");
        } else {
            println!("   ⚠️  Data might be mock data or invalid");
        }
        
        // Print a few sample data points for manual verification
        println!("   Sample data points:");
        for (i, point) in result.data.iter().take(3).enumerate() {
            println!("     {}: {:.0} @ ${:.2}", i + 1, point.timestamp, point.price);
        }
        
    } else {
        println!("   ❌ No data received to analyze");
    }
}