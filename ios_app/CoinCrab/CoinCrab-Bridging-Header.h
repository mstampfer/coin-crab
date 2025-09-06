#import "rust_ios_lib.h"

// Explicit function declarations to ensure linking works
typedef void (*PriceUpdateCallback)(const void* context);
void register_price_update_callback(PriceUpdateCallback callback);