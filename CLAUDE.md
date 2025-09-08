# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CoinCrab is a professional iOS cryptocurrency tracking app built with **Rust workspace architecture** and **SwiftUI**. The core architecture uses **MQTT client-server communication** where a Rust server handles CoinMarketCap API integration and publishes data to iOS clients via an embedded MQTT broker.

## Build Commands

### iOS Development
```bash
# Build iOS library for all targets (simulator + device)
chmod +x build_ios.sh
./build_ios.sh

# Open Xcode project for iOS development
open ios_app/CoinCrab.xcodeproj
```

### Server Development
```bash
# Build and run server with MQTT broker
cargo build -p coin-crab-server --release
cargo run -p coin-crab-server

# Run server in background for iOS testing
nohup cargo run -p coin-crab-server &
```

### Testing
```bash
# Test entire workspace
cargo test

# Test specific crates
cargo test -p shared
cargo test -p coin-crab-server  
cargo test -p rust_ios_lib

# Workspace validation
cargo check --workspace
```

## Architecture

### Rust Workspace Structure
- **3 main crates**: `server` (coin-crab-server), `ios_lib` (rust_ios_lib), `shared`
- **Modular design**: Each crate uses clean module architecture (`types.rs`, `config.rs`, `mqtt/mod.rs`, etc.)
- **Workspace resolver = "2"** for modern dependency management

### MQTT Client-Server Flow
```
iOS App (SwiftUI) ↔ Rust iOS Library (FFI) ↔ MQTT (port 1883) ↔ Rust Server ↔ CoinMarketCap API
```

**Key Integration Points:**
- **FFI Bridge**: `CoinCrab-Bridging-Header.h` connects Swift to Rust via C interface
- **Core FFI Functions**: `get_crypto_data()`, `register_price_update_callback()`, `free_string()`
- **Universal Binaries**: `lipo` creates combined simulator/device libraries
- **Real-time Updates**: MQTT provides live price streaming without exposing API keys to client

### Cryptocurrency Logo System
- **Server-side caching**: 24-hour logo cache with CoinMarketCap symbol-to-ID mapping
- **Fallback UI**: Colored circles with ticker symbols when no CMC mapping exists
- **Logo endpoint**: `GET /api/logo/{symbol}` serves cached logos or returns 404

## Environment Setup

### Required Files
```bash
# Server config (git-ignored)
crates/server/.env.server
CMC_API_KEY=your_coinmarketcap_api_key_here
MQTT_BROKER_HOST=127.0.0.1
LOG_LEVEL=INFO

# Client config (committed)
crates/ios_lib/.env.client
MQTT_BROKER_HOST=127.0.0.1  # Use your machine's IP for device testing
LOG_LEVEL=ERROR
```

### iOS Build Process
1. **Rust targets**: `aarch64-apple-ios` (device), `x86_64-apple-ios` + `aarch64-apple-ios-sim` (simulator)
2. **Universal binary creation**: `lipo` combines simulator architectures
3. **Output libraries**: `target/universal/release/librust_ios_lib.a`
4. **Xcode integration**: Library linked via project settings, FFI via bridging header

## Development Practices

### Code Style (from CLAUDE.local.md)
- **No emojis in source code** - maintains professional appearance and avoids encoding issues
- **Clean commit messages** - exclude Claude Code references and generated signatures

### Cryptocurrency Data Flow
1. **Server startup**: Loads CoinMarketCap symbol-to-ID mapping
2. **Price updates**: Periodic CMC API calls published via MQTT
3. **Logo requests**: iOS app requests `/api/logo/{symbol}`, server fetches and caches from CMC
4. **Fallback handling**: 404 responses trigger colored circle display with ticker symbol

### Key Dependencies
- **MQTT**: `rumqttd` (embedded broker), `rumqttc` (client)
- **Async Runtime**: `tokio` for all async operations
- **HTTP**: `actix-web` (server), `reqwest` (CMC client)
- **iOS FFI**: `libc` for C interface compatibility

### Deployment
- **CI/CD**: GitHub Actions deploys server to AWS EC2 (production: 100.26.107.175:1883)
- **Cross-compilation**: ARM64 Linux target for AWS deployment
- **Process management**: Server runs with PID tracking and health checks

## Common Issues

### iOS Build Failures
- Ensure `build_ios.sh` has execute permissions
- Verify Rust iOS targets are installed: `rustup target list --installed`
- Check that universal libraries exist in `target/universal/release/`

### MQTT Connection Issues
- Server must be running before launching iOS app
- For device testing, update `MQTT_BROKER_HOST` to machine's IP address
- Verify port 1883 is not blocked by firewall

### Logo Display Problems
- Server must have valid CMC API key for logo fetching
- Check logo cache in server logs
- Fallback colored circles should appear for unmapped symbols