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

