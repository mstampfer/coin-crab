#ifndef rust_ios_lib_h
#define rust_ios_lib_h

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Generic data fetching functions (used by Swift)
char* get_crypto_data(void);
char* get_historical_data(const char* symbol, const char* timeframe);

// Memory management
void free_string(char* s);

#ifdef __cplusplus
}
#endif

#endif /* rust_ios_lib_h */