# 🦀 Coin Crab

A modern iOS cryptocurrency tracking app built with SwiftUI frontend and Rust backend, featuring real-time price updates, animated price changes, and professional market data visualization.

<div align="center">
  
[![Swift](https://img.shields.io/badge/Swift-5.9-orange.svg)](https://swift.org/)
[![Rust](https://img.shields.io/badge/Rust-1.70+-red.svg)](https://rust-lang.org/)
[![iOS](https://img.shields.io/badge/iOS-17.0+-blue.svg)](https://developer.apple.com/ios/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

## ✨ Features

### 📱 **Modern iOS Interface**
- **Dark Theme**: Professional cryptocurrency market aesthetic
- **Real-time Data**: Live price updates every 30 seconds
- **Animated Prices**: Color-coded price change animations (green ↑, red ↓)
- **Market Overview**: Statistics cards for market cap, indices, and sentiment
- **Tab Navigation**: Full app structure with Markets, Alpha, Search, Portfolio, Community

### 🚀 **Real Cryptocurrency Icons**
- Authentic crypto logos loaded from multiple reliable sources
- Intelligent fallback system with brand-accurate colors
- Smart caching for optimal performance
- Support for 20+ major cryptocurrencies

### 💹 **Advanced Market Features**
- **Market Insights**: AI-powered market analysis prompts
- **Filter Controls**: Sort by price, market cap, 24h changes
- **Mini Charts**: Price trend visualizations for each coin
- **Professional Layout**: Ranking, market caps, and percentage changes

### ⚡ **Rust-Powered Backend**
- High-performance cryptocurrency API integration
- Concurrent request handling with Actix Web
- Built-in caching and rate limiting
- Cross-platform iOS library compilation

## 🛠 Tech Stack

- **Frontend**: SwiftUI, iOS 17+
- **Backend**: Rust, Actix Web, Tokio
- **APIs**: CoinMarketCap, multiple icon providers
- **Architecture**: FFI bridge between Swift and Rust
- **Build System**: Xcode + Cargo

## 📁 Project Structure

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
│   ├── lib.rs                  # FFI library for iOS
│   ├── server.rs               # Development server
│   └── test_api.rs             # API testing utilities
├── target/                     # Rust build artifacts
│   └── universal/release/      # iOS universal libraries
├── build_ios.sh               # iOS library build script
├── rust_ios_lib.h             # C header for Swift interop
└── Cargo.toml                 # Rust dependencies
```

## 🚀 Quick Start

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

4. **Start the development server** (optional)
   ```bash
   cargo run --bin crypto_server
   ```

5. **Build and run the iOS app**
   - Select your target device or simulator
   - Press `⌘+R` to build and run

## ⚙️ Configuration

### Environment Variables
Create a `.env` file in the project root:

```env
COINMARKETCAP_API_KEY=your_api_key_here
SERVER_PORT=8080
CACHE_DURATION=30
```

### API Configuration
The app uses multiple cryptocurrency data sources:
- **CoinMarketCap**: Primary price data
- **CryptoIcons.org**: Cryptocurrency logos
- **CoinCap**: Backup price source

## 📊 Architecture

### iOS Frontend
```
SwiftUI Views
     ↓
AnimatedPriceView ←→ PriceChangeTracker
     ↓
CryptoDataManager
     ↓
FFI Bridge (C)
     ↓
Rust Library
```

### Rust Backend
```
API Requests → Data Processing → Caching → FFI Export → iOS
```

### Key Components

- **`CryptoDataManager`**: Handles data fetching and state management
- **`AnimatedPriceView`**: Smooth price change animations
- **`CryptoIcon`**: Dynamic cryptocurrency logo loading
- **`PriceChangeTracker`**: Monitors and animates price movements
- **`IconCache`**: Efficient logo caching system

## 🎨 Design Philosophy

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

## 🔧 Development

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

## 📈 Performance

- **Startup time**: < 2 seconds
- **Price updates**: 30-second intervals
- **Memory usage**: ~50MB typical
- **Network**: Efficient API caching
- **Animations**: 60 FPS smooth transitions

## 🤝 Contributing

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

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **CoinMarketCap** for cryptocurrency data
- **CryptoIcons.org** for cryptocurrency logos
- **Rust Community** for excellent async libraries
- **Apple** for SwiftUI and iOS development tools

## 🐛 Issues & Support

Found a bug or have a feature request? Please [open an issue](https://github.com/yourusername/coin-crab-app/issues).

For questions and discussions, check out our [Discussions](https://github.com/yourusername/coin-crab-app/discussions) page.

---

<div align="center">

**Built with ❤️ using Swift & Rust**

[Report Bug](https://github.com/yourusername/coin-crab-app/issues) •
[Request Feature](https://github.com/yourusername/coin-crab-app/issues) •
[Contribute](CONTRIBUTING.md)

</div>