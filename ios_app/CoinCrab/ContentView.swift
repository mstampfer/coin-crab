import SwiftUI
import Foundation
import Combine


// MARK: - NotificationCenter Extension
extension NSNotification.Name {
    static let mqttPriceUpdate = NSNotification.Name("mqttPriceUpdate")
}

// MARK: - Data Models
struct CryptoCurrency: Codable, Identifiable {
    let id: Int32
    let name: String
    let symbol: String
    let quote: Quote
}

struct Quote: Codable {
    let USD: UsdQuote
}

struct UsdQuote: Codable {
    let price: Double
    let percent_change_1h: Double
    let percent_change_24h: Double
    let percent_change_7d: Double
    let market_cap: Double
    let volume_24h: Double
    let last_updated: String
    
    enum CodingKeys: String, CodingKey {
        case price
        case percent_change_1h
        case percent_change_24h
        case percent_change_7d
        case market_cap
        case volume_24h
        case last_updated
    }
}

struct CryptoClientResult: Codable {
    let success: Bool
    let data: [CryptoCurrency]?
    let error: String?
    let last_updated: String?
    let cached: Bool
}

// Server response structure (matches server.rs ApiResponse)
struct ApiResponse: Codable {
    let data: [CryptoCurrency]
    let last_updated: String
    let cached: Bool
}

// MARK: - Simplified Data Manager using Rust FFI delegation
class CryptoDataManager: ObservableObject {
    @Published var cryptocurrencies: [CryptoCurrency] = []
    @Published var isLoading = false
    @Published var errorMessage: String?
    @Published var lastUpdated: String?
    @Published var isDataCached = false
    
    private var refreshTimer: Timer?
    private var retryAttempts = 0
    private let maxRetryAttempts = 5
    
    init() {
        print("CryptoDataManager: Initializing with real-time MQTT callback architecture")
        setupRealTimeUpdates()
    }
    
    deinit {
        refreshTimer?.invalidate()
    }
    
    func fetchCryptoPrices() {
        print("fetchCryptoPrices: Calling Rust get_crypto_data() function (attempt \(retryAttempts + 1)/\(maxRetryAttempts))")
        isLoading = true
        errorMessage = nil
        
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            
            // Call Rust function that handles all networking internally (will use MQTT)
            guard let resultPtr = get_crypto_data() else {
                DispatchQueue.main.async {
                    self.handleFetchError("Failed to call Rust function")
                }
                return
            }
            
            let resultString = String(cString: resultPtr)
            free_string(resultPtr)
            
            print("fetchCryptoPrices: Got result from Rust: \(resultString.prefix(100))...")
            
            do {
                let result = try JSONDecoder().decode(CryptoClientResult.self, from: resultString.data(using: .utf8) ?? Data())
                
                DispatchQueue.main.async {
                    if result.success, let data = result.data {
                        // Success - reset retry counter
                        self.retryAttempts = 0
                        self.cryptocurrencies = data
                        self.lastUpdated = result.last_updated ?? DateFormatter.localizedString(from: Date(), dateStyle: .none, timeStyle: .medium)
                        self.isDataCached = result.cached
                        self.errorMessage = nil
                        self.isLoading = false
                        print("SUCCESS: Updated \(data.count) cryptocurrencies from Rust")
                    } else {
                        // Handle error with retry logic
                        let errorMsg = result.error ?? "Unknown error from Rust"
                        print("ERROR from Rust: \(errorMsg)")
                        self.handleFetchError(errorMsg)
                    }
                }
            } catch {
                DispatchQueue.main.async {
                    let errorMsg = "Failed to parse Rust response: \(error.localizedDescription)"
                    print("ERROR parsing Rust response: \(error)")
                    self.handleFetchError(errorMsg)
                }
            }
        }
    }
    
    private func handleFetchError(_ errorMsg: String) {
        retryAttempts += 1
        
        if retryAttempts <= maxRetryAttempts {
            // Calculate exponential backoff delay: 2^attempt seconds (2, 4, 8, 16, 32)
            let delay = TimeInterval(min(pow(2.0, Double(retryAttempts)), 32.0))
            print("fetchCryptoPrices: Attempt \(retryAttempts) failed, retrying in \(Int(delay)) seconds...")
            
            // Show retry message to user
            self.errorMessage = "Loading crypto prices... (attempt \(retryAttempts)/\(maxRetryAttempts))"
            
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
                self.fetchCryptoPrices()
            }
        } else {
            // Max retries exceeded
            print("fetchCryptoPrices: All \(maxRetryAttempts) attempts failed, giving up")
            self.errorMessage = "Unable to load crypto prices. Please check your connection and try again."
            self.isLoading = false
            self.retryAttempts = 0 // Reset for next manual refresh
        }
    }
    
    private func startPeriodicRefresh() {
        // Initial fetch
        fetchCryptoPrices()
        
        // Set up timer for periodic refresh (every 30 seconds)
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 30.0, repeats: true) { [weak self] _ in
            print("Timer triggered - calling Rust for crypto prices")
            self?.fetchCryptoPrices()
        }
    }
    
    func manualRefresh() {
        print("Manual refresh requested - calling Rust")
        retryAttempts = 0 // Reset retry counter for manual refresh
        fetchCryptoPrices()
    }
    
    private func setupRealTimeUpdates() {
        print("CryptoDataManager: Setting up real-time MQTT push notifications - NO POLLING")
        
        // Initial fetch to get data immediately AND initialize MQTT client
        fetchCryptoPrices()
        
        // Wait a moment for MQTT client to initialize, then register callback
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            self.setupMQTTCallback()
            print("CryptoDataManager: Real-time MQTT callback registered after client initialization")
        }
        
        print("CryptoDataManager: Waiting for real-time MQTT push notifications only")
        print("CryptoDataManager: No polling timers active")
    }
    
    private func setupMQTTCallback() {
        print("CryptoDataManager: Registering MQTT callback for real-time updates")
        
        // Simple callback that triggers data fetch when MQTT receives new data
        let callback: @convention(c) (UnsafeRawPointer?) -> Void = { context in
            DispatchQueue.main.async {
                print("MQTT Callback: Received real-time price update - fetching new data")
                // Post notification to trigger data fetch
                NotificationCenter.default.post(name: .mqttPriceUpdate, object: nil)
            }
        }
        
        // Register the callback with Rust FFI - MUST work for real-time updates
        register_price_update_callback(callback)
        print("CryptoDataManager: Real-time MQTT callback registered - NO POLLING!")
        
        // Listen for MQTT notifications
        NotificationCenter.default.addObserver(
            forName: .mqttPriceUpdate,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            print("CryptoDataManager: Processing MQTT price update notification")
            self?.fetchCryptoPrices()
        }
        
        print("CryptoDataManager: MQTT callback system registered successfully")
        print("CryptoDataManager: NO POLLING - Waiting for real-time MQTT callbacks only!")
    }
}

struct ContentView: View {
    @StateObject private var cryptoManager = CryptoDataManager()
    @State private var selectedTab = "Markets"
    
    init() {
        print("ContentView: init() called with simplified architecture")
    }
    
    var body: some View {
        VStack(spacing: 0) {
            // Main content area
            ZStack {
                switch selectedTab {
                case "Markets":
                    MarketsView(cryptoManager: cryptoManager)
                case "Alpha":
                    AlphaView()
                case "Search":
                    SearchView()
                case "Portfolio":
                    PortfolioView()
                case "Community":
                    CommunityView()
                default:
                    MarketsView(cryptoManager: cryptoManager)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            
            // Custom tab bar with text below
            VStack(spacing: 0) {
                // Tab bar
                HStack(spacing: 0) {
                    CustomTabBarItem(
                        icon: "chart.line.uptrend.xyaxis",
                        title: "Markets",
                        isSelected: selectedTab == "Markets",
                        action: { selectedTab = "Markets" }
                    )
                    
                    CustomTabBarItem(
                        icon: "brain.head.profile",
                        title: "Alpha",
                        isSelected: selectedTab == "Alpha",
                        action: { selectedTab = "Alpha" }
                    )
                    
                    CustomTabBarItem(
                        icon: "magnifyingglass",
                        title: "Search",
                        isSelected: selectedTab == "Search",
                        action: { selectedTab = "Search" }
                    )
                    
                    CustomTabBarItem(
                        icon: "chart.pie",
                        title: "Portfolio",
                        isSelected: selectedTab == "Portfolio",
                        action: { selectedTab = "Portfolio" }
                    )
                    
                    CustomTabBarItem(
                        icon: "person.2",
                        title: "Community",
                        isSelected: selectedTab == "Community",
                        action: { selectedTab = "Community" }
                    )
                }
                .padding(.top, 4)
                .padding(.horizontal, 16)
                .background(Color.black)
                
                // "powered by Rust" text below tab bar
                Text("powered by Rust")
                    .font(.caption2)
                    .foregroundColor(.gray.opacity(0.6))
                    .padding(.top, 2)
                    .padding(.bottom, 4)
                    .background(Color.black)
            }
        }
        .background(Color.black)
        .ignoresSafeArea(.keyboard, edges: .bottom)
    }
}

struct CustomTabBarItem: View {
    let icon: String
    let title: String
    let isSelected: Bool
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            VStack(spacing: 2) {
                Image(systemName: icon)
                    .font(.system(size: 20))
                    .foregroundColor(isSelected ? .blue : .gray)
                
                Text(title)
                    .font(.caption2)
                    .foregroundColor(isSelected ? .blue : .gray)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

struct MarketsView: View {
    @ObservedObject var cryptoManager: CryptoDataManager
    @State private var showingSearch = false
    @State private var showingAccount = false
    
    var body: some View {
        NavigationView {
            ZStack {
                Color.black.ignoresSafeArea()
                
                VStack(spacing: 0) {
                    // Coin List
                    CoinListHeaderView()
                    
                    if cryptoManager.isLoading && cryptoManager.cryptocurrencies.isEmpty {
                        LoadingStateView(errorMessage: cryptoManager.errorMessage)
                    } else if cryptoManager.cryptocurrencies.isEmpty && !cryptoManager.isLoading {
                        EmptyStateView()
                    } else {
                        ModernCryptoListView(cryptocurrencies: cryptoManager.cryptocurrencies, cryptoManager: cryptoManager)
                    }
                }
            }
            .navigationTitle("Markets")
            .navigationBarTitleDisplayMode(.large)
            .preferredColorScheme(.dark)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    HStack {
                        Button(action: {
                            showingSearch = true
                        }) {
                            Image(systemName: "magnifyingglass")
                                .foregroundColor(.white)
                        }
                        Button(action: {
                            showingAccount = true
                        }) {
                            Image(systemName: "person.crop.circle")
                                .foregroundColor(.gray)
                        }
                    }
                }
            }
            .sheet(isPresented: $showingSearch) {
                SearchView()
            }
            .sheet(isPresented: $showingAccount) {
                AccountView()
            }
        }
    }
}

struct MarketStatsHeaderView: View {
    var body: some View {
        HStack(spacing: 12) {
            MarketStatCard(title: "Market Cap", value: "$3.90T", change: "2.17%", isPositive: true)
            MarketStatCard(title: "CMC100", value: "$241.93", change: "2.40%", isPositive: true)
            MarketStatCard(title: "Altcoin Index", value: "44", subtitle: "/100", showSlider: true)
            FearGreedCard(value: 47)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }
}

struct MarketStatCard: View {
    let title: String
    let value: String
    let change: String?
    let isPositive: Bool
    let subtitle: String?
    let showSlider: Bool
    
    init(title: String, value: String, change: String? = nil, isPositive: Bool = true, subtitle: String? = nil, showSlider: Bool = false) {
        self.title = title
        self.value = value
        self.change = change
        self.isPositive = isPositive
        self.subtitle = subtitle
        self.showSlider = showSlider
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.caption)
                .foregroundColor(.gray)
            
            HStack(spacing: 2) {
                Text(value)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundColor(.white)
                if let subtitle = subtitle {
                    Text(subtitle)
                        .font(.caption)
                        .foregroundColor(.gray)
                }
            }
            
            if let change = change {
                HStack(spacing: 2) {
                    Image(systemName: isPositive ? "arrowtriangle.up.fill" : "arrowtriangle.down.fill")
                        .font(.system(size: 8))
                    Text(change)
                        .font(.caption)
                        .fontWeight(.medium)
                }
                .foregroundColor(isPositive ? .green : .red)
            }
            
            if showSlider {
                HStack(spacing: 0) {
                    Rectangle()
                        .fill(Color.orange)
                        .frame(width: 20, height: 3)
                    Rectangle()
                        .fill(Color.blue)
                        .frame(width: 30, height: 3)
                }
                .cornerRadius(2)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(Color.gray.opacity(0.1))
        .cornerRadius(12)
    }
}

struct FearGreedCard: View {
    let value: Int
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Fear & Greed")
                .font(.caption)
                .foregroundColor(.gray)
            
            HStack {
                ZStack {
                    Circle()
                        .stroke(Color.gray.opacity(0.3), lineWidth: 3)
                        .frame(width: 30, height: 30)
                    
                    Circle()
                        .trim(from: 0, to: CGFloat(value) / 100)
                        .stroke(value > 50 ? Color.green : Color.red, lineWidth: 3)
                        .frame(width: 30, height: 30)
                        .rotationEffect(.degrees(-90))
                }
                
                VStack(alignment: .leading) {
                    Text("\(value)")
                        .font(.system(size: 14, weight: .bold))
                        .foregroundColor(.white)
                    Text(value > 50 ? "Greed" : "Fear")
                        .font(.caption)
                        .foregroundColor(.gray)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(Color.gray.opacity(0.1))
        .cornerRadius(12)
    }
}

struct MarketInsightsView: View {
    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 12) {
                MarketInsightCard(icon: "brain", text: "Why is the market up today?")
                MarketInsightCard(icon: "brain", text: "Are altcoins outperforming?")
            }
            .padding(.horizontal, 16)
        }
        .padding(.vertical, 8)
    }
}

struct MarketInsightCard: View {
    let icon: String
    let text: String
    
    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .foregroundColor(.blue)
                .font(.system(size: 16))
            
            Text(text)
                .font(.system(size: 14))
                .foregroundColor(.white)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(Color.gray.opacity(0.1))
        .cornerRadius(20)
    }
}

struct CoinTabNavigationView: View {
    @Binding var selectedTab: String
    let tabs = ["Coins", "Watchlists", "DexScan", "Overview", "Yield"]
    
    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(tabs, id: \.self) { tab in
                    Button(action: { selectedTab = tab }) {
                        VStack(spacing: 8) {
                            Text(tab)
                                .font(.system(size: 16, weight: selectedTab == tab ? .semibold : .regular))
                                .foregroundColor(selectedTab == tab ? .blue : .gray)
                            
                            Rectangle()
                                .fill(selectedTab == tab ? Color.blue : Color.clear)
                                .frame(height: 2)
                        }
                    }
                    .frame(minWidth: 80)
                    .padding(.horizontal, 8)
                }
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

struct FilterControlsView: View {
    @Binding var selectedCurrency: String
    @Binding var selectedSort: String
    @Binding var selectedCategory: String
    
    var body: some View {
        HStack(spacing: 12) {
            FilterButton(title: selectedCurrency, subtitle: "/ BTC")
            FilterButton(title: selectedSort, hasDropdown: true)
            FilterButton(title: selectedCategory, hasDropdown: true)
            
            Spacer()
            
            Button(action: {}) {
                Image(systemName: "line.horizontal.3.decrease")
                    .foregroundColor(.white)
                    .font(.system(size: 16))
                    .padding(8)
                    .background(Color.gray.opacity(0.2))
                    .cornerRadius(8)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

struct FilterButton: View {
    let title: String
    let subtitle: String?
    let hasDropdown: Bool
    
    init(title: String, subtitle: String? = nil, hasDropdown: Bool = false) {
        self.title = title
        self.subtitle = subtitle
        self.hasDropdown = hasDropdown
    }
    
    var body: some View {
        Button(action: {}) {
            HStack(spacing: 4) {
                Text(title)
                    .font(.system(size: 14, weight: .medium))
                    .foregroundColor(.white)
                
                if let subtitle = subtitle {
                    Text(subtitle)
                        .font(.system(size: 14))
                        .foregroundColor(.gray)
                }
                
                if hasDropdown {
                    Image(systemName: "chevron.down")
                        .font(.system(size: 12))
                        .foregroundColor(.gray)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Color.gray.opacity(0.2))
            .cornerRadius(8)
        }
    }
}

struct CoinListHeaderView: View {
    var body: some View {
        HStack {
            HStack(spacing: 4) {
                Text("#")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundColor(.gray)
                
                Image(systemName: "arrow.down")
                    .font(.system(size: 10))
                    .foregroundColor(.blue)
                
                Text("Market Cap")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundColor(.gray)
            }
            
            Spacer()
            
            Text("Price")
                .font(.system(size: 12, weight: .medium))
                .foregroundColor(.gray)
            
            Spacer()
            
            Text("24h %")
                .font(.system(size: 12, weight: .medium))
                .foregroundColor(.gray)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

struct ModernCryptoListView: View {
    let cryptocurrencies: [CryptoCurrency]
    @ObservedObject var cryptoManager: CryptoDataManager
    
    var body: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(Array(cryptocurrencies.enumerated()), id: \.element.id) { index, crypto in
                    ModernCryptoCurrencyRowView(cryptocurrency: crypto, rank: index + 1, cryptoManager: cryptoManager)
                    if index < cryptocurrencies.count - 1 {
                        Divider()
                            .background(Color.gray.opacity(0.2))
                            .padding(.leading, 60)
                    }
                }
            }
        }
    }
}

// Price change tracker to store previous prices
class PriceChangeTracker: ObservableObject {
    static let shared = PriceChangeTracker()
    private var previousPrices: [String: Double] = [:]
    
    func updatePrice(for cryptoId: String, newPrice: Double) -> PriceChange {
        let previousPrice = previousPrices[cryptoId]
        previousPrices[cryptoId] = newPrice
        
        guard let previous = previousPrice else {
            return .none
        }
        
        if newPrice > previous {
            return .increased
        } else if newPrice < previous {
            return .decreased
        } else {
            return .none
        }
    }
}

enum PriceChange {
    case increased, decreased, none
}

struct AnimatedPriceView: View {
    let price: Double
    let cryptoId: String
    
    @State private var animationColor: Color = .white
    @State private var isAnimating = false
    @StateObject private var priceTracker = PriceChangeTracker.shared
    
    var body: some View {
        Text("$\(price, specifier: "%.2f")")
            .font(.system(size: 16, weight: .semibold))
            .foregroundColor(animationColor)
            .onChange(of: price) { oldValue, newValue in
                animatePriceChange(newPrice: newValue)
            }
            .onAppear {
                // Initialize tracking for this crypto
                _ = priceTracker.updatePrice(for: cryptoId, newPrice: price)
            }
            .onTapGesture {
                // Test animation by simulating a small price change
                let testChange = Double.random(in: -0.5...0.5)
                animatePriceChange(newPrice: price + testChange)
            }
    }
    
    private func animatePriceChange(newPrice: Double) {
        let changeType = priceTracker.updatePrice(for: cryptoId, newPrice: newPrice)
        
        guard changeType != .none && !isAnimating else { return }
        
        isAnimating = true
        
        let changeColor: Color = changeType == .increased ? .green : .red
        
        // Immediate transition to green/red
        withAnimation(.easeInOut(duration: 0.1)) {
            animationColor = changeColor
        }
        
        // Gradually transition back to white over 3 seconds
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            withAnimation(.easeOut(duration: 3.0)) {
                animationColor = .white
            }
            
            // Reset animation flag after total duration
            DispatchQueue.main.asyncAfter(deadline: .now() + 3.0) {
                isAnimating = false
            }
        }
    }
}

struct ModernCryptoCurrencyRowView: View {
    let cryptocurrency: CryptoCurrency
    let rank: Int
    @ObservedObject var cryptoManager: CryptoDataManager
    @State private var showingChart = false
    
    var body: some View {
        Button(action: {
            showingChart = true
        }) {
        HStack(spacing: 12) {
            // Rank
            Text("\(rank)")
                .font(.system(size: 14, weight: .medium))
                .foregroundColor(.gray)
                .frame(width: 20)
            
            // Crypto icon and info
            HStack(spacing: 12) {
                CryptoIcon(symbol: cryptocurrency.symbol)
                
                VStack(alignment: .leading, spacing: 2) {
                    Text(cryptocurrency.symbol)
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundColor(.white)
                    
                    Text(formatMarketCap(cryptocurrency.quote.USD.market_cap))
                        .font(.system(size: 12))
                        .foregroundColor(.gray)
                }
            }
            
            Spacer()
            
            // Price and change with animation
            VStack(alignment: .trailing, spacing: 4) {
                AnimatedPriceView(
                    price: cryptocurrency.quote.USD.price,
                    cryptoId: cryptocurrency.symbol
                )
                
                HStack(spacing: 4) {
                    let change24h = cryptocurrency.quote.USD.percent_change_24h
                    
                    Image(systemName: change24h >= 0 ? "arrowtriangle.up.fill" : "arrowtriangle.down.fill")
                        .font(.system(size: 8))
                    
                    Text("\(abs(change24h), specifier: "%.2f")%")
                        .font(.system(size: 12, weight: .medium))
                }
                .foregroundColor(cryptocurrency.quote.USD.percent_change_24h >= 0 ? .green : .red)
            }
            
            // Mini chart placeholder
            MiniChartView(isPositive: cryptocurrency.quote.USD.percent_change_24h >= 0)
                .frame(width: 60, height: 30)
        }
        }
        .buttonStyle(PlainButtonStyle())
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .background(Color.clear)
        .sheet(isPresented: $showingChart) {
            CryptoChartView(initialCryptocurrency: cryptocurrency, cryptoManager: cryptoManager)
        }
    }
    
    private func formatMarketCap(_ marketCap: Double) -> String {
        if marketCap >= 1_000_000_000_000 {
            return String(format: "$%.2fT", marketCap / 1_000_000_000_000)
        } else if marketCap >= 1_000_000_000 {
            return String(format: "$%.2fB", marketCap / 1_000_000_000)
        } else if marketCap >= 1_000_000 {
            return String(format: "$%.2fM", marketCap / 1_000_000)
        } else {
            return String(format: "$%.0f", marketCap)
        }
    }
}

// Client configuration reader for .env.client file
class ClientConfig {
    static let shared = ClientConfig()
    
    private var config: [String: String] = [:]
    
    init() {
        loadConfig()
    }
    
    private func loadConfig() {
        // Look for .env.client in the main bundle
        guard let path = Bundle.main.path(forResource: ".env", ofType: "client") else {
            print("ClientConfig: .env.client not found in bundle, using defaults")
            // Set defaults if file not found
            config["MQTT_BROKER_HOST"] = "127.0.0.1"
            config["HTTPS_ICON_HOST"] = "127.0.0.1"
            config["HTTP_ICON_PORT"] = "8080"
            return
        }
        
        do {
            let contents = try String(contentsOfFile: path, encoding: .utf8)
            let lines = contents.components(separatedBy: .newlines)
            
            for line in lines {
                let trimmed = line.trimmingCharacters(in: .whitespaces)
                
                // Skip empty lines and comments
                if trimmed.isEmpty || trimmed.hasPrefix("#") {
                    continue
                }
                
                // Parse KEY=VALUE pairs
                if let equalsIndex = trimmed.firstIndex(of: "=") {
                    let key = String(trimmed[..<equalsIndex]).trimmingCharacters(in: .whitespaces)
                    var value = String(trimmed[trimmed.index(after: equalsIndex)...]).trimmingCharacters(in: .whitespaces)
                    
                    // Handle inline comments by stopping at first #
                    if let commentIndex = value.firstIndex(of: "#") {
                        value = String(value[..<commentIndex]).trimmingCharacters(in: .whitespaces)
                    }
                    
                    config[key] = value
                    print("ClientConfig: Loaded \(key) = \(value)")
                }
            }
        } catch {
            print("ClientConfig: Error reading .env.client: \(error)")
            // Set defaults on error
            config["MQTT_BROKER_HOST"] = "127.0.0.1"
            config["HTTPS_ICON_HOST"] = "127.0.0.1"
            config["HTTP_ICON_PORT"] = "8080"
        }
    }
    
    var serverHost: String {
        return config["MQTT_BROKER_HOST"] ?? "127.0.0.1"
    }
    
    var serverPort: Int {
        if let portStr = config["MQTT_BROKER_PORT"], let port = Int(portStr) {
            return port
        }
        return 1883
    }
    
    var httpIconPort: Int {
        if let portStr = config["HTTP_ICON_PORT"], let port = Int(portStr) {
            return port
        }
        return 8080
    }
    
    var httpsIconHost: String {
        return config["HTTPS_ICON_HOST"] ?? "127.0.0.1"
    }
}

// Simple icon cache to avoid repeated downloads
class IconCache {
    static let shared = IconCache()
    private var cache: [String: Data] = [:]
    private var failedSymbols: Set<String> = []
    
    func getIcon(for symbol: String) -> Data? {
        let key = symbol.uppercased()
        return cache[key]
    }
    
    func setIcon(for symbol: String, data: Data) {
        let key = symbol.uppercased()
        cache[key] = data
        failedSymbols.remove(key)
        print("IconCache: Cached icon for \(key), cache size: \(cache.count)")
    }
    
    func markFailed(for symbol: String) {
        let key = symbol.uppercased()
        failedSymbols.insert(key)
        print("IconCache: Marked \(key) as failed, failed count: \(failedSymbols.count)")
    }
    
    func hasFailed(for symbol: String) -> Bool {
        return failedSymbols.contains(symbol.uppercased())
    }
    
    func clearCache() {
        cache.removeAll()
        failedSymbols.removeAll()
        print("IconCache: Cleared all cached data")
    }
}

struct CryptoIcon: View {
    let symbol: String
    @State private var imageData: Data?
    @State private var isLoading = true
    
    var body: some View {
        ZStack {
            if let imageData = imageData, let uiImage = UIImage(data: imageData) {
                Image(uiImage: uiImage)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(width: 32, height: 32)
                    .clipShape(Circle())
            } else {
                // Fallback to colored circle with symbol
                ZStack {
                    Circle()
                        .fill(colorForSymbol(symbol))
                        .frame(width: 32, height: 32)
                    
                    if isLoading {
                        ProgressView()
                            .scaleEffect(0.5)
                            .progressViewStyle(CircularProgressViewStyle(tint: .white))
                    } else {
                        Text(symbol)
                            .font(.system(size: symbol.count > 4 ? 8 : (symbol.count > 3 ? 9 : 11), weight: .bold))
                            .foregroundColor(.white)
                            .minimumScaleFactor(0.5)
                            .lineLimit(1)
                    }
                }
                .onTapGesture {
                    print("DEBUG: \(symbol) - isLoading: \(isLoading), imageData: \(imageData?.count ?? 0) bytes, hasFailed: \(IconCache.shared.hasFailed(for: symbol))")
                }
            }
        }
        .onAppear {
            loadCryptoIcon()
        }
    }
    
    private func loadCryptoIcon() {
        // Check cache first
        if let cachedData = IconCache.shared.getIcon(for: symbol) {
            self.imageData = cachedData
            self.isLoading = false
            return
        }
        
        // If previously failed, skip server request and show fallback
        if IconCache.shared.hasFailed(for: symbol) {
            self.isLoading = false
            return
        }
        
        // Load from local server's logo endpoint
        loadIconFromServer()
    }
    
    private func loadIconFromServer() {
        let httpsHost = ClientConfig.shared.httpsIconHost
        guard let url = URL(string: "https://\(httpsHost)/api/logo/\(symbol)") else {
            print("CryptoIcon: Invalid URL for symbol \(symbol)")
            IconCache.shared.markFailed(for: symbol)
            isLoading = false
            return
        }
        
        print("CryptoIcon: Loading icon for \(symbol) from server: \(url)")
        
        URLSession.shared.dataTask(with: url) { data, response, error in
            DispatchQueue.main.async {
                if let error = error {
                    print("CryptoIcon: Network error for \(self.symbol): \(error)")
                    IconCache.shared.markFailed(for: self.symbol)
                    self.isLoading = false
                    return
                }
                
                if let httpResponse = response as? HTTPURLResponse {
                    print("CryptoIcon: HTTP response for \(self.symbol): \(httpResponse.statusCode)")
                    
                    if httpResponse.statusCode == 200, let data = data, !data.isEmpty {
                        print("CryptoIcon: Successfully loaded \(data.count) bytes for \(self.symbol)")
                        IconCache.shared.setIcon(for: self.symbol, data: data)
                        self.imageData = data
                    } else if httpResponse.statusCode == 404 {
                        print("CryptoIcon: No logo mapping found for \(self.symbol) - marking as failed and using fallback colored circle")
                        IconCache.shared.markFailed(for: self.symbol)
                    } else {
                        print("CryptoIcon: Failed to load icon for \(self.symbol) - status: \(httpResponse.statusCode), data size: \(data?.count ?? 0)")
                        IconCache.shared.markFailed(for: self.symbol)
                    }
                } else {
                    print("CryptoIcon: Invalid HTTP response for \(self.symbol)")
                    IconCache.shared.markFailed(for: self.symbol)
                }
                
                self.isLoading = false
            }
        }.resume()
    }
    
    
    private func colorForSymbol(_ symbol: String) -> Color {
        // Use brand colors for popular cryptocurrencies
        let brandColors: [String: Color] = [
            "BTC": Color.orange,
            "ETH": Color.blue,
            "USDT": Color.green,
            "BNB": Color.yellow,
            "SOL": Color.purple,
            "USDC": Color.blue,
            "XRP": Color.blue,
            "DOGE": Color.yellow,
            "ADA": Color.blue,
            "SHIB": Color.orange,
            "AVAX": Color.red,
            "DOT": Color.pink,
            "LINK": Color.blue,
            "BCH": Color.green,
            "NEAR": Color.black,
            "MATIC": Color.purple,
            "UNI": Color.pink,
            "LTC": Color.gray,
            "ICP": Color.orange,
            "LEO": Color.orange,
            "ONDO": Color.blue,
            "WLD": Color.gray,
            "ARB": Color.blue,
            "POL": Color.purple,
            "PI": Color.orange,
            "USD1": Color.green,
            "IP": Color.purple,
            "KAS": Color.cyan,
            "ATOM": Color.purple
        ]
        
        return brandColors[symbol.uppercased()] ?? {
            let colors: [Color] = [.orange, .blue, .purple, .green, .red, .yellow, .pink, .indigo]
            let index = abs(symbol.hashValue) % colors.count
            return colors[index]
        }()
    }
}

struct MiniChartView: View {
    let isPositive: Bool
    
    var body: some View {
        Path { path in
            let points = generateRandomPoints()
            guard !points.isEmpty else { return }
            
            path.move(to: points[0])
            for point in points.dropFirst() {
                path.addLine(to: point)
            }
        }
        .stroke(isPositive ? Color.green : Color.red, lineWidth: 1.5)
        .frame(width: 60, height: 30)
    }
    
    private func generateRandomPoints() -> [CGPoint] {
        let width: CGFloat = 60
        let height: CGFloat = 30
        var points: [CGPoint] = []
        
        for i in 0..<8 {
            let x = CGFloat(i) * (width / 7)
            let y = CGFloat.random(in: 5...(height - 5))
            points.append(CGPoint(x: x, y: y))
        }
        
        return points
    }
}

struct LoadingStateView: View {
    let errorMessage: String?
    
    var body: some View {
        VStack(spacing: 20) {
            ProgressView()
                .progressViewStyle(CircularProgressViewStyle(tint: .blue))
                .scaleEffect(1.5)
            
            Text("Loading crypto prices...")
                .font(.title2)
                .fontWeight(.medium)
                .foregroundColor(.white)
            
            if let errorMessage = errorMessage, !errorMessage.isEmpty {
                Text(errorMessage)
                    .font(.caption)
                    .foregroundColor(.gray)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 20)
            } else {
                Text("Connecting to MQTT broker...")
                    .font(.caption)
                    .foregroundColor(.gray)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.black)
    }
}

struct EmptyStateView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "bitcoinsign.circle")
                .font(.system(size: 64))
                .foregroundColor(.gray)
            
            Text("No crypto data available")
                .font(.title2)
                .fontWeight(.medium)
                .foregroundColor(.white)
            
            Text("Pull to refresh or check your connection")
                .font(.caption)
                .foregroundColor(.gray)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.black)
    }
}

// Placeholder views for other tabs
struct AlphaView: View {
    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            
            VStack(spacing: 16) {
                Image(systemName: "brain.head.profile")
                    .font(.system(size: 64))
                    .foregroundColor(.gray)
                
                Text("Alpha")
                    .font(.title)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                
                Text("Coming soon...")
                    .font(.body)
                    .foregroundColor(.gray)
            }
        }
        .preferredColorScheme(.dark)
    }
}

struct SearchView: View {
    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            
            VStack(spacing: 16) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 64))
                    .foregroundColor(.gray)
                
                Text("Search")
                    .font(.title)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                
                Text("Coming soon...")
                    .font(.body)
                    .foregroundColor(.gray)
            }
        }
        .preferredColorScheme(.dark)
    }
}

struct PortfolioView: View {
    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            
            VStack(spacing: 16) {
                Image(systemName: "chart.pie")
                    .font(.system(size: 64))
                    .foregroundColor(.gray)
                
                Text("Portfolio")
                    .font(.title)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                
                Text("Coming soon...")
                    .font(.body)
                    .foregroundColor(.gray)
            }
        }
        .preferredColorScheme(.dark)
    }
}

struct CommunityView: View {
    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            
            VStack(spacing: 16) {
                Image(systemName: "person.2")
                    .font(.system(size: 64))
                    .foregroundColor(.gray)
                
                Text("Community")
                    .font(.title)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                
                Text("Coming soon...")
                    .font(.body)
                    .foregroundColor(.gray)
            }
        }
        .preferredColorScheme(.dark)
    }
}

struct AccountView: View {
    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            
            VStack(spacing: 16) {
                Image(systemName: "person.crop.circle")
                    .font(.system(size: 64))
                    .foregroundColor(.gray)
                
                Text("Account")
                    .font(.title)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                
                Text("Coming soon...")
                    .font(.body)
                    .foregroundColor(.gray)
            }
        }
        .preferredColorScheme(.dark)
    }
}