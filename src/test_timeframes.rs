fn get_test_api_key() -> String {
    dotenv::dotenv().ok();
    std::env::var("CMC_API_KEY").unwrap_or_else(|_| {
        panic!("CMC_API_KEY environment variable is required for tests. Please set it in .env file")
    })
}

#[test]
fn test_all_ios_timeframes() {
    println!("🧪 Testing all iOS timeframes...");
    
    let timeframes = vec![
        ("1h", "1 hour"),
        ("24h", "24 hours - WORKING"), 
        ("7d", "7 days"),
        ("30d", "30 days"),
        ("90d", "90 days"),
        ("365d", "1 year"),
        ("all", "All time")
    ];
    
    for (timeframe, description) in timeframes {
        println!("\n   Testing {}: {}", timeframe, description);
        
        let api_key = get_test_api_key();
        let endpoint = format!("cmc://historical/btc?timeframe={}&api_key={}", timeframe, api_key);
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(crate::fetch_historical_data(&endpoint, None));
        
        if result.success {
            println!("     ✅ Success: {} data points", result.data.len());
            if !result.data.is_empty() {
                println!("     First price: ${:.2}", result.data[0].price);
                println!("     Last price: ${:.2}", result.data.last().unwrap().price);
            }
        } else {
            println!("     ❌ Failed: {:?}", result.error);
        }
    }
}

#[test] 
fn test_problematic_timeframes() {
    println!("🧪 Testing potentially problematic timeframes...");
    
    // Test the ones that might be causing issues
    let problem_timeframes = vec![
        "1h",   // Might be too short for CMC API
        "90d",  // Recently fixed
        "all",  // Newly added support
        "365d"  // Long timeframe
    ];
    
    for timeframe in problem_timeframes {
        println!("\n   🔍 Deep testing: {}", timeframe);
        
        let api_key = get_test_api_key();
        let endpoint = format!("cmc://historical/btc?timeframe={}&api_key={}", timeframe, api_key);
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(crate::fetch_historical_data(&endpoint, None));
        
        println!("     Endpoint: {}", endpoint);
        println!("     Success: {}", result.success);
        println!("     Data points: {}", result.data.len());
        
        if let Some(error) = result.error {
            println!("     Error: {}", error);
        }
        
        if result.success && !result.data.is_empty() {
            println!("     Time range: {:.0} to {:.0}", 
                     result.data[0].timestamp, 
                     result.data.last().unwrap().timestamp);
            println!("     First price: ${:.2}", result.data[0].price);
            println!("     Last price: ${:.2}", result.data.last().unwrap().price);
        }
    }
}

#[test] 
fn test_cmc_intervals() {
    println!("🧪 Testing CoinMarketCap API interval support...");
    
    // Test different intervals to see which ones CMC API actually supports
    let intervals_to_test = vec![
        ("5m", "5 minute intervals"),
        ("15m", "15 minute intervals"), 
        ("1h", "1 hour intervals"),
        ("2h", "2 hour intervals"),
        ("6h", "6 hour intervals"),
        ("12h", "12 hour intervals"),
        ("1d", "1 day intervals"),
    ];
    
    for (interval, description) in intervals_to_test {
        println!("\\n   🔍 Testing interval: {} ({})", interval, description);
        
        // Create a direct API URL to test the interval
        let historical_url = format!(
            "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/historical?id=1&time_start=2025-08-21T00:00:00.000Z&time_end=2025-08-28T00:00:00.000Z&interval={}",
            interval
        );
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        let client = reqwest::Client::new();
        let response = rt.block_on(async {
            client
                .get(&historical_url)
                .header("X-CMC_PRO_API_KEY", &get_test_api_key())
                .header("Accept", "application/json")
                .send()
                .await
        });
        
        match response {
            Ok(resp) => {
                println!("     Status: {}", resp.status());
                if !resp.status().is_success() {
                    if let Ok(text) = rt.block_on(resp.text()) {
                        println!("     Error response: {}", &text[..100.min(text.len())]);
                    }
                } else {
                    println!("     ✅ Interval {} is supported", interval);
                }
            }
            Err(e) => {
                println!("     ❌ Request failed: {}", e);
            }
        }
    }
}