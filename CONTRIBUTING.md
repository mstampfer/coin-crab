# Contributing to Coin Crab 

Thank you for your interest in contributing to Coin Crab! This guide will help you get started with contributing to our modern iOS cryptocurrency tracking app built with Rust and SwiftUI.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Contributing Guidelines](#contributing-guidelines)
- [Pull Request Process](#pull-request-process)
- [Testing](#testing)
- [Code Style](#code-style)
- [Issue Reporting](#issue-reporting)

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful, inclusive, and constructive in all interactions.

## Getting Started

### Prerequisites

Before contributing, ensure you have:

- **macOS** with Xcode 15+ installed
- **Rust toolchain** (install from [rustup.rs](https://rustup.rs/))
- **Git** for version control
- **CoinMarketCap API key** (for testing server features)

### Fork and Clone

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/coin-crab.git
   cd coin-crab
   ```
3. **Add upstream** remote:
   ```bash
   git remote add upstream https://github.com/mstampfer/coin-crab.git
   ```

## Development Setup

### 1. Build the Workspace

```bash
# Build iOS library
chmod +x build_ios.sh
./build_ios.sh

# Test workspace builds
cargo check
cargo build --workspace
```

### 2. Set Up Environment

```bash
# Copy environment template
cp .env.example crates/server/.env.server

# Edit with your CMC API key
# crates/server/.env.server
CMC_API_KEY=your_api_key_here
```

### 3. Run the App

```bash
# Start MQTT server
cargo run -p coin-crab-server

# Open iOS project
open ios_app/CoinCrab.xcodeproj
```

## Project Structure

Our Rust workspace is organized as follows:

```
coin-crab-app/
├── crates/
│   ├── server/         # MQTT server with CMC API integration
│   ├── ios_lib/        # Rust library for iOS FFI
│   └── shared/         # Common data structures and utilities
├── ios_app/            # SwiftUI iOS application
└── target/             # Build artifacts
```

## Contributing Guidelines

### Areas for Contribution

We welcome contributions in these areas:

#### **Frontend (SwiftUI)**
- UI/UX improvements
- New chart features and visualizations
- Performance optimizations
- Accessibility enhancements

#### **Backend (Rust)**
- MQTT broker optimizations
- New cryptocurrency data sources
- API rate limiting and caching
- Error handling improvements

#### **Charts & Data**
- TradingView chart customizations
- New technical indicators
- Data processing improvements
- Historical data analysis

#### **Infrastructure**
- Build system improvements
- CI/CD pipeline enhancements
- Documentation updates
- Testing framework expansion

### Contribution Types

- **Bug Fixes**: Fix existing issues or broken functionality
- **Features**: Add new functionality or enhance existing features
- **Performance**: Improve app speed, memory usage, or efficiency
- **Documentation**: Improve README, code comments, or guides
- **Tests**: Add or improve test coverage
- **Refactoring**: Improve code structure without changing functionality

## Pull Request Process

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-number-description
```

### 2. Make Changes

- Follow the [code style guidelines](#code-style)
- Add tests for new functionality
- Update documentation as needed
- Ensure all tests pass

### 3. Commit Changes

Use descriptive commit messages:

```bash
git commit -m "Add volume histogram color customization

- Allow users to customize volume bar colors
- Add color picker in settings view
- Update TradingView chart integration
- Add tests for color validation"
```

### 4. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub with:

- **Clear title** describing the change
- **Detailed description** of what was changed and why
- **Screenshots** for UI changes
- **Testing instructions** for reviewers
- **Reference to related issues** (if applicable)

### 5. PR Review Process

- Maintainers will review your PR
- Address any requested changes
- Once approved, your PR will be merged

## Testing

### Running Tests

```bash
# Test entire workspace
cargo test

# Test specific crates
cargo test -p shared
cargo test -p coin-crab-server
cargo test -p rust_ios_lib

# iOS app testing
# Run tests in Xcode or use xcodebuild
xcodebuild test -project ios_app/CoinCrab.xcodeproj -scheme CoinCrab -destination 'platform=iOS Simulator,name=iPhone 16'
```

### Test Guidelines

- **Unit Tests**: Test individual functions and components
- **Integration Tests**: Test crate interactions and MQTT communication
- **UI Tests**: Test user interface functionality (iOS)
- **Performance Tests**: Ensure changes don't degrade performance

### Test Coverage

- Aim for at least 80% code coverage on new code
- Include both positive and negative test cases
- Test error conditions and edge cases

## Code Style

### Rust Code Style

Follow standard Rust conventions:

```rust
// Use descriptive names
fn calculate_volume_weighted_average_price(prices: &[f64], volumes: &[f64]) -> f64 {
    // Implementation
}

// Document public APIs
/// Calculates the 24-hour price change percentage
/// 
/// # Arguments
/// * `current_price` - Current cryptocurrency price
/// * `previous_price` - Price 24 hours ago
/// 
/// # Returns
/// Percentage change as f64 (-100.0 to +∞)
pub fn calculate_24h_change(current_price: f64, previous_price: f64) -> f64 {
    ((current_price - previous_price) / previous_price) * 100.0
}
```

### Swift Code Style

Follow Swift conventions and SwiftUI best practices:

```swift
// Use descriptive names
func formatCurrencyValue(_ value: Double) -> String {
    // Implementation
}

// MARK: - View Components
struct CryptoListView: View {
    @State private var selectedTimeframe: TimeFrame = .day
    
    var body: some View {
        // Implementation
    }
}
```

### Formatting

- **Rust**: Use `cargo fmt` to format code
- **Swift**: Use Xcode's built-in formatting (Ctrl+I)
- **Comments**: Write clear, concise comments explaining the "why"
- **Documentation**: Update relevant documentation for API changes

## Issue Reporting

### Before Creating an Issue

1. **Search existing issues** to avoid duplicates
2. **Check documentation** for solutions
3. **Test with latest version** to ensure it's not already fixed

### Creating Good Issues

Include the following information:

#### Bug Reports

```markdown
## Bug Description
Clear description of what went wrong

## Steps to Reproduce
1. Step one
2. Step two  
3. Step three

## Expected Behavior
What you expected to happen

## Actual Behavior
What actually happened

## Environment
- iOS Version: 17.0
- Device: iPhone 15 Pro
- App Version: 1.0.0
- Server Version: Latest

## Screenshots/Logs
Include relevant screenshots or log output
```

#### Feature Requests

```markdown
## Feature Description
Clear description of the proposed feature

## Use Case
Why is this feature needed?

## Proposed Solution
How should this feature work?

## Alternatives Considered
Other approaches you've considered

## Additional Context
Any other relevant information
```

## Development Tips

### Debugging

- **Rust**: Use `cargo run -p coin-crab-server` with debug logging
- **iOS**: Use Xcode debugger and console logs
- **MQTT**: Monitor MQTT traffic with tools like MQTT Explorer

### Performance

- **Profile regularly** to catch performance regressions
- **Test on real devices** for accurate performance metrics
- **Monitor memory usage** especially in chart rendering

### Architecture

- **Keep crates focused** - each crate should have a single responsibility
- **Use shared crate** for common functionality
- **Follow MQTT patterns** for real-time communication

## Getting Help

If you need help:

1. **Check the documentation** in README.md
2. **Search existing issues** on GitHub
3. **Create a discussion** for general questions
4. **Join our community** discussions

## Recognition

Contributors will be recognized in:

- Project README
- Release notes
- GitHub contributors list

Thank you for contributing to Coin Crab! 

---

For questions about contributing, please [open an issue](https://github.com/mstampfer/coin-crab/issues) or start a discussion.