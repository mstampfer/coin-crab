#ifndef RUST_IOS_LIB_H
#define RUST_IOS_LIB_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

// Generic data fetching functions (used by Swift)
char* get_crypto_data(void);
char* get_historical_data(const char* symbol, const char* timeframe);

// Memory management
void free_string(char* s);

#endif // RUST_IOS_LIB_H