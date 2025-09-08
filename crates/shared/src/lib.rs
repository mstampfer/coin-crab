// Shared Library - Modular Architecture
// Main library interface that provides shared types and utilities

// Module declarations
mod types;
mod logging;

// Re-export public types and functions for external use
pub use types::{
    CryptoCurrency,
    Quote, 
    UsdQuote,
    HistoricalDataPoint,
    HistoricalDataResult,
};

pub use logging::{
    debug_log,
    init_logging,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_re_exports() {
        // Test that all main types are re-exported and accessible
        // This ensures the pub use statements are working correctly
        
        // Test that types can be constructed and are accessible
        let usd_quote = UsdQuote {
            price: 50000.0,
            percent_change_1h: 0.5,
            percent_change_24h: 2.5,
            percent_change_7d: 10.0,
            market_cap: 900000000000.0,
            volume_24h: 50000000000.0,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        
        let quote = Quote { usd: usd_quote };
        
        let crypto = CryptoCurrency {
            id: 1,
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            quote,
        };
        
        // Test basic properties
        assert_eq!(crypto.id, 1);
        assert_eq!(crypto.name, "Bitcoin");
        assert_eq!(crypto.symbol, "BTC");
        assert_eq!(crypto.quote.usd.price, 50000.0);
        
        // Test historical data types
        let historical_point = HistoricalDataPoint {
            timestamp: 1704067200.0,
            price: 45000.0,
            volume: Some(1000000000.0),
        };
        
        let historical_result = HistoricalDataResult {
            success: true,
            data: vec![historical_point],
            error: None,
            symbol: Some("BTC".to_string()),
            timeframe: Some("24h".to_string()),
        };
        
        assert!(historical_result.success);
        assert_eq!(historical_result.data.len(), 1);
        assert_eq!(historical_result.data[0].price, 45000.0);
        assert_eq!(historical_result.symbol, Some("BTC".to_string()));
        assert_eq!(historical_result.timeframe, Some("24h".to_string()));
        assert!(historical_result.error.is_none());
    }

    #[test]
    fn test_logging_functions_accessible() {
        // Test that logging functions are re-exported and accessible
        // We can't actually test the logging behavior without side effects,
        // but we can verify the functions exist and are properly imported
        
        // These should compile and not panic
        debug_log("Test message");
        
        // init_logging would initialize the global logger, which we don't want in tests
        // but we can verify it exists
        let _init_fn = init_logging;
        
        assert!(true); // If we reach here, all functions are accessible
    }

    #[test]
    fn test_library_version_info() {
        // Test basic library information
        let lib_name = env!("CARGO_PKG_NAME");
        let lib_version = env!("CARGO_PKG_VERSION");
        
        assert_eq!(lib_name, "shared");
        assert!(!lib_version.is_empty());
    }
}

