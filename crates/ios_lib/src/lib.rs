// iOS Library - Modular Architecture
// Main library interface that ties together all modules

// Module declarations
mod types;
mod config;
mod mqtt;
mod ffi;
mod globals;

// Re-export public types for external use
pub use types::{ApiResponse, CryptoClientResult, CryptoCurrency, HistoricalDataResult};
pub use mqtt::MQTTClient;

// Re-export FFI functions (they have #[no_mangle] so they're automatically exposed to C)
pub use ffi::{free_string, get_crypto_data, get_historical_data};

// Re-export global initialization functions
pub use globals::{init_mqtt_client, is_mqtt_connected, reset_mqtt_connection_attempts};

// Test modules (kept in main lib.rs for now)
#[cfg(test)]
mod test_historical;

#[cfg(test)]
mod test_c_function;

#[cfg(test)]
mod test_timeframes;

#[cfg(test)]
mod test_specific_symbols;