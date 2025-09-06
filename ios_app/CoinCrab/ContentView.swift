import SwiftUI
import Foundation
import Combine

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
        print("CryptoDataManager: Initializing with Rust FFI delegation architecture")
        startPeriodicRefresh()
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
}

struct ContentView: View {
    @StateObject private var cryptoManager = CryptoDataManager()
    @State private var selectedTab = "Markets"
    
    init() {
        print("ContentView: init() called with simplified architecture")
    }
    
    var body: some View {
        TabView(selection: $selectedTab) {
            MarketsView(cryptoManager: cryptoManager)
            .tabItem {
                Image(systemName: "chart.line.uptrend.xyaxis")
                Text("Markets")
            }
            .tag("Markets")
            
            AlphaView()
            .tabItem {
                Image(systemName: "brain.head.profile")
                Text("Alpha")
            }
            .tag("Alpha")
            
            SearchView()
            .tabItem {
                Image(systemName: "magnifyingglass")
                Text("Search")
            }
            .tag("Search")
            
            PortfolioView()
            .tabItem {
                Image(systemName: "chart.pie")
                Text("Portfolio")
            }
            .tag("Portfolio")
            
            CommunityView()
            .tabItem {
                Image(systemName: "person.2")
                Text("Community")
            }
            .tag("Community")
        }
        .accentColor(.blue)
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
                        ModernCryptoListView(cryptocurrencies: cryptoManager.cryptocurrencies)
                    }
                }
            }
            .navigationTitle("Markets")
            .navigationBarTitleDisplayMode(.large)
            .preferredColorScheme(.dark)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Image("AppLogo")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(width: 30, height: 30)
                }
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
    
    var body: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(Array(cryptocurrencies.enumerated()), id: \.element.id) { index, crypto in
                    ModernCryptoCurrencyRowView(cryptocurrency: crypto, rank: index + 1)
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
    @State private var animationScale: CGFloat = 1.0
    @State private var isAnimating = false
    @StateObject private var priceTracker = PriceChangeTracker.shared
    
    var body: some View {
        Text("$\(price, specifier: "%.2f")")
            .font(.system(size: 16, weight: .semibold))
            .foregroundColor(animationColor)
            .scaleEffect(animationScale)
            .onChange(of: price) { oldValue, newValue in
                animatePriceChange(newPrice: newValue)
            }
            .onAppear {
                // Initialize tracking for this crypto
                _ = priceTracker.updatePrice(for: cryptoId, newPrice: price)
            }
    }
    
    private func animatePriceChange(newPrice: Double) {
        let changeType = priceTracker.updatePrice(for: cryptoId, newPrice: newPrice)
        
        guard changeType != .none && !isAnimating else { return }
        
        isAnimating = true
        
        let flashColor: Color = changeType == .increased ? .green : .red
        
        // Quick flash to green/red with subtle scale effect (0.15 seconds)
        withAnimation(.easeInOut(duration: 0.15)) {
            animationColor = flashColor
            animationScale = 1.05
        }
        
        // Scale back to normal quickly
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) {
            withAnimation(.easeOut(duration: 0.15)) {
                animationScale = 1.0
            }
        }
        
        // Slower transition back to white (2.5 seconds)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            withAnimation(.easeOut(duration: 2.5)) {
                animationColor = .white
            }
            
            // Reset animation flag after total duration
            DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
                isAnimating = false
            }
        }
    }
}

struct ModernCryptoCurrencyRowView: View {
    let cryptocurrency: CryptoCurrency
    let rank: Int
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
            CryptoChartView(cryptocurrency: cryptocurrency)
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

// Simple icon cache to avoid repeated downloads
class IconCache {
    static let shared = IconCache()
    private var cache: [String: Data] = [:]
    
    func getIcon(for symbol: String) -> Data? {
        return cache[symbol.uppercased()]
    }
    
    func setIcon(for symbol: String, data: Data) {
        cache[symbol.uppercased()] = data
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
                        Text(symbol.prefix(2))
                            .font(.system(size: 10, weight: .bold))
                            .foregroundColor(.white)
                    }
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
        
        // No direct network calls - use fallback icon immediately
        // Icon loading should be delegated to Rust layer if needed
        isLoading = false
    }
    
    private func getCoinMarketCapId(for symbol: String) -> String {
        // Map common symbols to their CoinMarketCap IDs for better accuracy
        let symbolMap: [String: String] = [
            "BTC": "1",
            "ETH": "1027",
            "USDT": "825",
            "BNB": "1839",
            "SOL": "5426",
            "USDC": "3408",
            "XRP": "52",
            "DOGE": "74",
            "ADA": "2010",
            "SHIB": "5994",
            "AVAX": "5805",
            "DOT": "6636",
            "LINK": "1975",
            "BCH": "1831",
            "NEAR": "6535",
            "MATIC": "3890",
            "UNI": "7083",
            "LTC": "2",
            "ICP": "8916",
            "LEO": "3957"
        ]
        
        return symbolMap[symbol.uppercased()] ?? "1"
    }
    
    private func getCoinGeckoId(for symbol: String) -> String {
        // Map common symbols to their CoinGecko image IDs
        let symbolMap: [String: String] = [
            "BTC": "1",
            "ETH": "279",
            "USDT": "825",
            "BNB": "825",
            "SOL": "4128",
            "USDC": "6319",
            "XRP": "44",
            "DOGE": "5",
            "ADA": "975",
            "SHIB": "11939",
            "AVAX": "12559",
            "DOT": "12171",
            "LINK": "1975",
            "BCH": "1831",
            "NEAR": "14803",
            "MATIC": "4713",
            "UNI": "12504",
            "LTC": "2",
            "ICP": "8916",
            "LEO": "11150"
        ]
        
        return symbolMap[symbol.uppercased()] ?? "1" // Default to Bitcoin if not found
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
            "LEO": Color.orange
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