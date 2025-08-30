# 🦀 Coin Crab

A modern iOS cryptocurrency tracking app built with Rust and SwiftUI, featuring real-time price updates, animated price changes, and professional market data visualization.

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.70+-red.svg)](https://rust-lang.org/)
[![Swift](https://img.shields.io/badge/Swift-5.9-orange.svg)](https://swift.org/)
[![iOS](https://img.shields.io/badge/iOS-17.0+-blue.svg)](https://developer.apple.com/ios/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

## Features

### **Modern iOS Interface**
- **Dark Theme**: Professional cryptocurrency market aesthetic
- **Real-time Data**: Live price updates every 30 seconds
- **Animated Prices**: Color-coded price change animations (green ↑, red ↓)
- **Market Overview**: Statistics cards for market cap, indices, and sentiment
- **Tab Navigation**: Full app structure with Markets, Alpha, Search, Portfolio, Community

### **Real Cryptocurrency Icons**
- Authentic crypto logos loaded from multiple reliable sources
- Intelligent fallback system with brand-accurate colors
- Smart caching for optimal performance
- Support for 20+ major cryptocurrencies

### **Advanced Market Features**
- **Market Insights**: AI-powered market analysis prompts
- **Filter Controls**: Sort by price, market cap, 24h changes
- **Mini Charts**: Price trend visualizations for each coin
- **Professional Layout**: Ranking, market caps, and percentage changes

### **Secure Server Architecture**
- MQTT-based real-time communication
- Server-side CoinMarketCap API integration for security
- No API keys stored on client devices
- High-performance Rust MQTT broker with rumqttd

## Tech Stack

- **Frontend**: SwiftUI, iOS 17+
- **Client & Server**: **100% Rust** - Both MQTT client (iOS library) and server written in Rust
- **Communication**: MQTT message broker for real-time updates
- **APIs**: CoinMarketCap (server-side only), multiple icon providers
- **Architecture**: Rust-to-Rust MQTT communication with Swift FFI bridge
- **Build System**: Xcode + Cargo

### Rust Dependencies

**Core Libraries:**
- `tokio` - Async runtime for all async operations
- `serde` + `serde_json` - JSON serialization/deserialization
- `chrono` - Date/time handling with timestamps
- `libc` - C FFI bindings for iOS integration
- `dotenv` - Environment variable loading

**MQTT Communication:**
- `rumqttc` - High-performance MQTT client
- `rumqttd` - Embedded MQTT broker
- `toml` - MQTT broker configuration parsing

**HTTP & API:**
- `reqwest` - HTTP client for CoinMarketCap API
- `actix-web` - Web framework (development server)

**Utilities:**
- `log` + `env_logger` - Structured logging system

## Project Structure

```
coin-crab-app/
├── ios_app/                    # iOS Xcode project
│   ├── CoinCrab.xcodeproj/     # Xcode project file
│   └── CoinCrab/               # iOS app source code
│       ├── CoinCrabApp.swift   # App entry point
│       ├── ContentView.swift   # Main SwiftUI views
│       ├── CoinCrab-Bridging-Header.h
│       └── Assets.xcassets/    # App icons and assets
├── src/                        # Rust backend source
│   ├── lib.rs                  # MQTT client library for iOS
│   ├── server.rs               # MQTT server with CMC API integration
│   └── test_api.rs             # API testing utilities
├── target/                     # Rust build artifacts
│   └── universal/release/      # iOS universal libraries
├── .env.client                 # Client environment (MQTT broker host)
├── .env.server                 # Server environment (API keys - git ignored)
├── .env.example                # Environment template
├── rumqttd.toml                # MQTT broker configuration
├── build_ios.sh               # iOS library build script
├── rust_ios_lib.h             # C header for Swift interop
└── Cargo.toml                 # Rust dependencies
```

## Quick Start

### Prerequisites

- **macOS** with Xcode 15+ installed
- **Rust toolchain**: Install from [rustup.rs](https://rustup.rs/)
- **iOS development** setup and provisioning
- **CoinMarketCap API key** (optional, for extended features)

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/coin-crab-app.git
   cd coin-crab-app
   ```

2. **Build the Rust library**
   ```bash
   chmod +x build_ios.sh
   ./build_ios.sh
   ```
   This will:
   - Install iOS targets for Rust
   - Build libraries for device and simulator
   - Create universal binaries

3. **Open the iOS project**
   ```bash
   open ios_app/CoinCrab.xcodeproj
   ```

4. **Set up environment files**
   ```bash
   # Copy and configure server environment (with your API key)
   cp .env.example .env.server
   # Edit .env.server and add your CoinMarketCap API key
   
   # Client environment is already configured in .env.client
   ```

5. **Start the MQTT server**
   ```bash
   cargo run --bin crypto_server
   ```
   This will start both the MQTT broker and the data publishing service.

6. **Build and run the iOS app**
   - Select your target device or simulator
   - Press `⌘+R` to build and run
   - The app will connect to the MQTT broker for real-time updates

## Configuration

### Environment Variables

The app uses separate environment files for security:

**Client Configuration** (`.env.client` - safe to commit):
```env
# MQTT Broker Configuration
# For iOS device testing, set this to your machine's IP address
# For local development/simulator, use 127.0.0.1
MQTT_BROKER_HOST=127.0.0.1
```

**Server Configuration** (`.env.server` - git ignored):
```env
# CoinMarketCap API Configuration
CMC_API_KEY=your_coinmarketcap_api_key_here

# MQTT Broker Configuration
MQTT_BROKER_HOST=127.0.0.1
```

**Important Security Notes:**
- The `.env.server` file is git-ignored and contains sensitive API keys
- Only the server needs the CMC API key - clients never see it
- Use `.env.example` as a template
- Get your API key from: https://coinmarketcap.com/api/

### API Configuration
The app uses multiple cryptocurrency data sources:
- **CoinMarketCap**: Primary price data
- **CryptoIcons.org**: Cryptocurrency logos
- **CoinCap**: Backup price source

## Architecture

### Rust-Powered MQTT Client-Server Architecture
```
┌─────────────────┐    MQTT     ┌──────────────────┐
│   iOS Client    │◄──────────►│   Rust Server    │
│                 │             │                  │
│ SwiftUI Views   │             │ MQTT Broker      │
│       ↓         │             │ (rumqttd)        │
│ 🦀 Rust MQTT    │             │       ↓          │
│    Client       │             │ 🦀 CMC API       │
│   (rumqttc)     │             │    Client        │
│       ↓         │             │       ↓          │
│ FFI Bridge      │             │ 🦀 Data Publisher│
│       ↓         │             │                  │
│ 🦀 Rust Library │             │                  │
└─────────────────┘             └──────────────────┘
```

### Security Benefits
- **No API keys on client**: All CoinMarketCap requests from server only
- **Real-time updates**: MQTT provides instant price notifications
- **Scalable**: Multiple clients can connect to one server
- **Offline resilience**: Client maintains last known data when disconnected

### Key Components

- **`CryptoDataManager`**: Handles data fetching and state management
- **`AnimatedPriceView`**: Smooth price change animations
- **`CryptoIcon`**: Dynamic cryptocurrency logo loading
- **`PriceChangeTracker`**: Monitors and animates price movements
- **`IconCache`**: Efficient logo caching system

## Design Philosophy

### Color System
- **Background**: Pure black (#000000)
- **Text**: White primary, gray secondary
- **Accents**: Blue for interactive elements
- **Success**: Green for price increases
- **Danger**: Red for price decreases

### Animation Principles
- **Fast attention**: 0.15s color flash
- **Smooth transitions**: 2.5s fade back to neutral
- **Subtle effects**: 5% scale increase for emphasis
- **Performance first**: Optimized for smooth scrolling

## Development

### Running Tests
```bash
# Test Rust backend
cargo test

# Test API endpoints
cargo run --bin test_api
```

### Building for Release
```bash
# Build optimized Rust library
cargo build --release --target aarch64-apple-ios
cargo build --release --target x86_64-apple-ios
cargo build --release --target aarch64-apple-ios-sim

# Create universal library
./build_ios.sh
```

### Adding New Cryptocurrencies
Update the symbol mappings in `ContentView.swift`:

```swift
// Add to getCoinMarketCapId function
"NEWCOIN": "coin_id_number"

// Add to brandColors dictionary
"NEWCOIN": Color.purple
```

## Performance

- **Startup time**: < 2 seconds
- **Price updates**: 30-second intervals
- **Memory usage**: ~50MB typical
- **Network**: Efficient API caching
- **Animations**: 60 FPS smooth transitions

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines
- Follow Swift coding conventions
- Use Rust best practices for backend code
- Add tests for new features
- Update documentation for API changes

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **CoinMarketCap** for cryptocurrency data
- **CryptoIcons.org** for cryptocurrency logos
- **Rust Community** for excellent async libraries
- **Apple** for SwiftUI and iOS development tools

## Issues & Support

Found a bug or have a feature request? Please [open an issue](https://github.com/yourusername/coin-crab-app/issues).

For questions and discussions, check out our [Discussions](https://github.com/yourusername/coin-crab-app/discussions) page.

---

<div align="center">

[Report Bug](https://github.com/yourusername/coin-crab-app/issues) •
[Request Feature](https://github.com/yourusername/coin-crab-app/issues) •
[Contribute](CONTRIBUTING.md)

</div>
