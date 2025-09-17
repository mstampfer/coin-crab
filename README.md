# 🦀 Coin Crab

A professional iOS cryptocurrency tracking app built with Rust and SwiftUI, featuring **TradingView charts with volume analysis**, real-time price updates, and enterprise-grade market data visualization.

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.70+-red.svg)](https://rust-lang.org/)
[![Swift](https://img.shields.io/badge/Swift-5.9-orange.svg)](https://swift.org/)
[![iOS](https://img.shields.io/badge/iOS-17.0+-blue.svg)](https://developer.apple.com/ios/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Discord](https://img.shields.io/badge/discord-Join%20Chat-7289da?logo=discord&logoColor=white)](https://discord.gg/qqpcR5J3)

</div>

## Features

### ** NEW: Professional TradingView Charts**
- **TradingView Lightweight Charts**: Enterprise-grade charting with v5.0 API
- **Volume Histogram**: Trading volume displayed as color-coded bars (green/red based on price movement)
- **Multiple Timeframes**: 1H, 24H, 7D, 30D, 90D, 1Y, All - all visible without scrolling
- **Reference Lines**: Dashed lines showing start and end prices for timeframe context
- **Smart Scaling**: Volume uses bottom 20% of chart with separate scale
- **Interactive Charts**: Fullscreen mode with landscape orientation support

### **Modern iOS Interface**
- **Dark Theme**: Professional cryptocurrency market aesthetic
- **Real-time Data**: Live price updates via MQTT with configurable logging
- **Animated Prices**: Color-coded price change animations (green ↑, red ↓)
- **Market Overview**: Statistics cards for market cap, indices, and sentiment
- **Optimized UI**: Removed grey bars, smaller fullscreen icons, improved spacing

### **Real Cryptocurrency Icons**
- **CoinMarketCap Logo System**: Official cryptocurrency logos fetched from CMC API
- **Symbol-to-ID Mapping**: Intelligent mapping system for accurate logo retrieval
- **24-Hour Caching**: Server-side logo caching with automatic expiry management
- **Professional UI**: Clean interface without app branding distractions

### **Advanced Market Features**
- **Market Insights**: AI-powered market analysis prompts
- **Filter Controls**: Sort by price, market cap, 24h changes
- **Mini Charts**: Price trend visualizations for each coin
- **Professional Layout**: Ranking, market caps, and percentage changes

### ** Rust Workspace Architecture**
- **Modular Design**: All crates use clean module architecture for maintainability
- **Separate Deployments**: Independent server and client builds
- **Workspace Structure**: Organized into `server`, `ios_lib`, and `shared` crates
- **Code Sharing**: Common data structures and utilities in shared crate
- **MQTT Communication**: High-performance real-time updates with rumqttd
- **Configurable Logging**: Single LOG_LEVEL parameter controls all logging
- **Security First**: API keys server-side only, no sensitive data on client

## Tech Stack

- **Frontend**: SwiftUI + TradingView Lightweight Charts v5.0, iOS 17+
- **Backend**: **100% Rust Workspace** - Server, iOS library, and shared code
- **Charts**: TradingView Lightweight Charts with volume histogram analysis
- **Communication**: MQTT message broker for real-time updates (rumqttd)
- **APIs**: CoinMarketCap with volume data and official logos (server-side only)
- **Architecture**: Rust workspace with separate deployable crates
- **Build System**: Xcode + Cargo with workspace support

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

### **Rust Workspace Architecture**

```
coin-crab-app/
├── Cargo.toml                  # Workspace manifest (resolver = "2")
├── crates/                     # All Rust project crates
│   ├── server/                 # Server crate (coin-crab-server) - modularized
│   │   ├── src/
│   │   │   ├── main.rs         # Server entry point (71 lines)
│   │   │   ├── types.rs        # Data structures and types
│   │   │   ├── config.rs       # Configuration management
│   │   │   ├── handlers.rs     # HTTP API endpoints
│   │   │   ├── data.rs         # CoinMarketCap API integration
│   │   │   └── mqtt/           # MQTT functionality
│   │   │       ├── mod.rs      # Module declarations
│   │   │       ├── broker.rs   # MQTT broker setup
│   │   │       ├── publisher.rs # Message publishing
│   │   │       └── request_handler.rs # Request handling
│   │   ├── Cargo.toml          # Server dependencies
│   │   ├── .env.server         # Server config (CMC API key)
│   │   └── rumqttd.toml        # MQTT broker configuration
│   ├── ios_lib/                # iOS library crate (rust_ios_lib) - modularized
│   │   ├── src/
│   │   │   ├── lib.rs          # Main interface (32 lines)
│   │   │   ├── types.rs        # Data structures
│   │   │   ├── config.rs       # iOS configuration
│   │   │   ├── ffi.rs          # C FFI functions for Swift
│   │   │   ├── globals.rs      # Global state management
│   │   │   └── mqtt/           # MQTT client functionality
│   │   │       ├── mod.rs      # Module declarations
│   │   │       ├── client.rs   # MQTT client core
│   │   │       ├── connection.rs # Connection management
│   │   │       └── message_handler.rs # Message processing
│   │   ├── Cargo.toml          # iOS dependencies
│   │   └── .env.client         # Client config (MQTT host)
│   └── shared/                 # Shared crate for common code - modularized
│       ├── src/
│       │   ├── lib.rs          # Main interface (22 lines)
│       │   ├── types.rs        # Shared data structures
│       │   └── logging.rs      # Logging utilities
│       └── Cargo.toml          # Shared dependencies
├── ios_app/                    # iOS Xcode project
│   ├── CoinCrab.xcodeproj/     # Xcode project file
│   └── CoinCrab/               # SwiftUI app source
│       ├── CoinCrabApp.swift   # App entry point
│       ├── ContentView.swift   # Main market view
│       ├── CryptoChartView.swift     # Chart view with timeframes
│       ├── TradingViewChartView.swift # TradingView integration
│       ├── CoinCrab-Bridging-Header.h # Rust FFI bridge
│       └── Assets.xcassets/    # App icons and assets
├── target/                     # Rust build artifacts (workspace-wide)
│   └── universal/release/      # iOS universal libraries
├── .env.example                # Environment template
├── build_ios.sh               # iOS library build script (workspace-aware)
└── README.md                  # This file
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

2. **Build the iOS library from workspace**
   ```bash
   chmod +x build_ios.sh
   ./build_ios.sh
   ```
   This will:
   - Install iOS targets for Rust
   - Build the `ios_lib` crate for device and simulator
   - Create universal binaries with shared dependencies

3. **Open the iOS project**
   ```bash
   open ios_app/CoinCrab.xcodeproj
   ```

4. **Set up environment files**
   ```bash
   # Copy and configure server environment (with your API key)
   cp .env.example crates/server/.env.server
   # Edit crates/server/.env.server and add your CoinMarketCap API key
   
   # Client environment is already configured in crates/ios_lib/.env.client
   ```

5. **Start the MQTT server (workspace)**
   ```bash
   cargo build -p coin-crab-server --release
   cargo run -p coin-crab-server
   ```
   This will start both the MQTT broker and the data publishing service.

6. **Build and run the iOS app**
   - Select your target device or simulator
   - Press `⌘+R` to build and run
   - The app will connect to the MQTT broker for real-time updates

## Configuration

### Environment Variables

The app uses separate environment files for security:

**Client Configuration** (`crates/ios_lib/.env.client` - safe to commit):
```env
# MQTT Broker Configuration
# For iOS device testing, set this to your machine's IP address  
# For local development/simulator, use 127.0.0.1
MQTT_BROKER_HOST=127.0.0.1

# Logging Configuration
# Options: OFF, ERROR, WARN, INFO, DEBUG, TRACE
LOG_LEVEL=ERROR
```

**Server Configuration** (`crates/server/.env.server` - git ignored):
```env
# CoinMarketCap API Configuration
CMC_API_KEY=your_coinmarketcap_api_key_here

# MQTT Broker Configuration
MQTT_BROKER_HOST=127.0.0.1

# Logging Configuration
# Options: OFF, ERROR, WARN, INFO, DEBUG, TRACE
LOG_LEVEL=INFO                   # Set to OFF to suppress all logs
```

**Important Security Notes:**
- The `.env.server` file is git-ignored and contains sensitive API keys
- Only the server needs the CMC API key - clients never see it
- Use `.env.example` as a template
- Get your API key from: https://coinmarketcap.com/api/

### API Configuration
The app uses CoinMarketCap as the primary data source:
- **CoinMarketCap**: Price data, historical charts, symbol-to-ID mapping, and official logos
- **Server-side caching**: 24-hour logo caching and intelligent mapping system

## Architecture

### Modular Crate Architecture

All three crates in the workspace follow a clean modular architecture:

**Benefits of Modularization:**
- **Better Organization**: Each module has a single, clear responsibility
- **Easier Navigation**: Related code is grouped together in focused modules
- **Improved Maintainability**: Changes are isolated to relevant modules
- **Cleaner Dependencies**: Module boundaries make dependencies explicit
- **Reduced Complexity**: Main files reduced from 500-800 lines to <100 lines
- **Consistent Pattern**: All crates follow the same modular structure

**Module Structure:**
- **Server**: `types`, `config`, `handlers`, `data`, `mqtt/*` modules
- **iOS Library**: `types`, `config`, `ffi`, `globals`, `mqtt/*` modules  
- **Shared**: `types`, `logging` modules

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
# Test entire workspace
cargo test

# Test specific crates
cargo test -p shared
cargo test -p coin-crab-server  
cargo test -p rust_ios_lib

# Check all crates
cargo check
```

### Building for Release
```bash
# Build server independently
cargo build -p coin-crab-server --release

# Build iOS library for all targets
./build_ios.sh

# Or build specific iOS targets
cargo build -p rust_ios_lib --release --target aarch64-apple-ios
cargo build -p rust_ios_lib --release --target x86_64-apple-ios  
cargo build -p rust_ios_lib --release --target aarch64-apple-ios-sim
```

### Deployment Options

#### Local Development
```bash
# Use local MQTT broker for development
cp .env.local crates/ios_lib/.env.client

# Deploy server only
cargo build -p coin-crab-server --release
./target/release/coin-crab-server

# Build iOS library only  
cargo build -p rust_ios_lib --target aarch64-apple-ios-sim --release

# Build everything
cargo build --release --workspace
```

#### Dual-Environment Deployment (AWS EC2)
The project includes automated CI/CD with GitHub Actions for both UAT and Production:

```bash
# UAT deployment - triggers on push to uat branch
git push origin uat

# Production deployment - triggers on push to main branch  
git push origin main
```

**CI/CD Pipeline Features:**
- ✅ **Dual Environment Support**: UAT and Production deployments
- ✅ **HTTPS/SSL Security**: Let's Encrypt certificates with nginx reverse proxy
- ✅ **Automated Testing**: Runs all Rust server and shared crate tests
- ✅ **Build Verification**: Cross-compilation for ARM64 architecture
- ✅ **Environment Isolation**: Separate folders and ports for UAT/Production
- ✅ **Health Checks**: Verifies MQTT and HTTP API endpoints
- ✅ **Zero Downtime**: Graceful server restart with process management

**Environment Configuration:**
- **Production**: `coincrab.duckdns.org` (HTTPS), MQTT:1883, HTTP:8080 → nginx:443
- **UAT**: `uat-coincrab.duckdns.org` (HTTPS), MQTT:1882, HTTP:8079 → nginx:443

**Server Management:**
```bash
# SSH into production server
ssh -i ~/.ssh/aws-freetier.pem ec2-user@100.26.107.175

# Check server status
cd coin_crab_server
cat server.pid  # Get process ID
ps -p $(cat server.pid)  # Check if running
tail -f server.log  # View live logs

# Manual server control
./coin-crab-server  # Start manually
pkill coin-crab-server  # Stop server
```

### Adding New Cryptocurrencies
The cryptocurrency logo system automatically handles new coins through the CoinMarketCap mapping system. No manual configuration required - logos are fetched dynamically based on the symbol-to-ID mapping maintained by the server.

For UI customization in `ios_app/CoinCrab/ContentView.swift`:
```swift
// Add to brandColors dictionary for fallback colors only
"NEWCOIN": Color.purple
```

### Customizing Charts
The TradingView charts can be customized in `ios_app/CoinCrab/TradingViewChartView.swift`:

```swift
// Modify chart colors
let chartColor = isPositive ? "#00C851" : "#FF4444"  
let fillColor = isPositive ? "rgba(0, 200, 81, 0.1)" : "rgba(255, 68, 68, 0.1)"

// Adjust volume histogram positioning
chart.priceScale('volume').applyOptions({
    scaleMargins: {
        top: 0.8,     // Volume uses bottom 20% of chart
        bottom: 0,
    },
});
```

## Performance

- **Startup time**: < 2 seconds with workspace architecture
- **Charts**: TradingView Lightweight Charts for 60 FPS rendering
- **Volume data**: Real-time histogram updates from CMC API
- **Price updates**: MQTT real-time streaming with configurable intervals
- **Memory usage**: ~50MB typical (including chart engine)
- **Network**: Efficient API caching with MQTT compression
- **Build time**: Parallel crate builds with workspace optimization
- **Animations**: 60 FPS smooth transitions across all timeframes

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

- **CoinMarketCap** for cryptocurrency data and official logos
- **TradingView** for professional charting components
- **Rust Community** for excellent async libraries
- **Apple** for SwiftUI and iOS development tools

## Issues & Support

Found a bug or have a feature request? _Please_ [open an issue](https://github.com/mstampfer/coin-crab/issues).

---

<div align="center">

[Report Bug](https://github.com/mstampfer/coin-crab/issues) •
[Request Feature](https://github.com/mstampfer/coin-crab/issues) •
[Contribute](CONTRIBUTING.md)

</div>
