#ifndef RUST_IOS_LIB_H
#define RUST_IOS_LIB_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

// Callback function type for real-time price updates
typedef void (*PriceUpdateCallback)(const void* context);

// Generic data fetching functions (used by Swift)
char* get_crypto_data(void);
char* get_historical_data(const char* symbol, const char* timeframe);

// Real-time callback registration
void register_price_update_callback(PriceUpdateCallback callback);

// Memory management
void free_string(char* s);

#endif // RUST_IOS_LIB_H