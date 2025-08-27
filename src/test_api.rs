use reqwest::Client;
use serde_json::Value;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // Load .env file
    dotenv::dotenv().ok();
    
    let api_key = env::var("CMC_API_KEY")
        .expect("CMC_API_KEY environment variable must be set");
    
    if api_key == "YOUR_API_KEY_HERE" || api_key.is_empty() {
        eprintln!("❌ Please set a valid CMC_API_KEY in your .env file");
        eprintln!("   Get your free API key from: https://pro.coinmarketcap.com/account");
        std::process::exit(1);
    }
    
    println!("🔑 Testing API key: {}...", &api_key[..8.min(api_key.len())]);
    
    let client = Client::new();
    
    // Test 1: Simple API info call (uses minimal quota)
    println!("\n📊 Testing API info endpoint...");
    let info_response = client
        .get("https://pro-api.coinmarketcap.com/v1/key/info")
        .header("X-CMC_PRO_API_KEY", &api_key)
        .header("Accept", "application/json")
        .send()
        .await?;
    
    println!("Status: {}", info_response.status());
    
    if info_response.status().is_success() {
        let info_data: Value = info_response.json().await?;
        println!("✅ API Key is valid!");
        println!("Plan: {}", info_data["data"]["plan"]["name"].as_str().unwrap_or("Unknown"));
        println!("Credits used today: {}", info_data["data"]["usage"]["current_day"]["credits_used"]);
        println!("Credits left today: {}", info_data["data"]["usage"]["current_day"]["credits_left"]);
    } else {
        let error_text = info_response.text().await?;
        println!("❌ API Key test failed: {}", error_text);
        return Ok(());
    }
    
    // Test 2: Try the actual endpoint we use in the server
    println!("\n🪙 Testing cryptocurrency listings endpoint...");
    let listings_response = client
        .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest")
        .query(&[("limit", "5"), ("convert", "USD")])  // Just 5 coins to save quota
        .header("X-CMC_PRO_API_KEY", &api_key)
        .header("Accept", "application/json")
        .send()
        .await?;
    
    println!("Status: {}", listings_response.status());
    
    if listings_response.status().is_success() {
        let listings_data: Value = listings_response.json().await?;
        println!("✅ Cryptocurrency data fetched successfully!");
        
        if let Some(data) = listings_data["data"].as_array() {
            println!("📈 First {} cryptocurrencies:", data.len());
            for (i, crypto) in data.iter().take(3).enumerate() {
                let name = crypto["name"].as_str().unwrap_or("Unknown");
                let symbol = crypto["symbol"].as_str().unwrap_or("?");
                let price = crypto["quote"]["USD"]["price"].as_f64().unwrap_or(0.0);
                println!("  {}. {} (${}) - ${:.2}", i + 1, name, symbol, price);
            }
        }
    } else {
        let error_text = listings_response.text().await?;
        println!("❌ Cryptocurrency listings test failed: {}", error_text);
        
        // Try to parse the error for more details
        if let Ok(error_json) = serde_json::from_str::<Value>(&error_text) {
            if let Some(status) = error_json["status"].as_object() {
                println!("Error code: {}", status["error_code"]);
                println!("Error message: {}", status["error_message"]);
            }
        }
    }
    
    Ok(())
}