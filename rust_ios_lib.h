#ifndef rust_ios_lib_h
#define rust_ios_lib_h

#ifdef __cplusplus
extern "C" {
#endif

char* hello_rust_world(void);
char* get_latest_crypto_prices(const char* endpoint);
void free_string(char* s);

#ifdef __cplusplus
}
#endif

#endif /* rust_ios_lib_h */