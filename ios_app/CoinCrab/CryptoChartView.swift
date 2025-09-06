import SwiftUI
import Foundation
import WebKit

struct CryptoChartView: View {
    let cryptocurrency: CryptoCurrency
    @State private var selectedTimeframe: TimeFrame = .day
    @State private var historicalData: [ChartDataPoint] = []
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showingFullscreen = false
    @Environment(\.presentationMode) var presentationMode
    
    enum TimeFrame: String, CaseIterable {
        case hour = "1H"
        case day = "24H"
        case week = "7D"
        case month = "30D"
        case quarter = "90D"
        case year = "1Y"
        case all = "All"
        
        var cmcTimeframe: String {
            switch self {
            case .hour: return "1h"
            case .day: return "24h"
            case .week: return "7d"
            case .month: return "30d"
            case .quarter: return "90d"
            case .year: return "365d"
            case .all: return "all"
            }
        }
        
    }
    
    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            
            VStack(spacing: 0) {
                // Custom navigation header
                HStack {
                    Button(action: {
                        presentationMode.wrappedValue.dismiss()
                    }) {
                        HStack(spacing: 8) {
                            Image(systemName: "chevron.left")
                                .font(.system(size: 18, weight: .medium))
                            Text("Back")
                                .font(.system(size: 17, weight: .medium))
                        }
                        .foregroundColor(.white)
                    }
                    
                    Spacer()
                    
                    HStack(spacing: 16) {
                        Button(action: {}) {
                            Image(systemName: "plus")
                                .foregroundColor(.white)
                        }
                        Button(action: {}) {
                            Image(systemName: "bell")
                                .foregroundColor(.white)
                        }
                        Button(action: {}) {
                            Image(systemName: "square.and.arrow.up")
                                .foregroundColor(.white)
                        }
                    }
                }
                .padding(.horizontal, 16)
                .padding(.top, 12)
                .padding(.bottom, 8)
                
                // Header with crypto info
                CryptoHeaderView(cryptocurrency: cryptocurrency)
                
                // Time frame selector
                TimeFrameSelectorView(selectedTimeframe: $selectedTimeframe)
                        .onChange(of: selectedTimeframe) { _, newValue in
                            loadHistoricalDataFromFFI()
                        }
                    
                    // Chart container
                    if isLoading {
                        ChartLoadingView()
                    } else if let errorMessage = errorMessage {
                        ChartErrorView(message: errorMessage) {
                            loadHistoricalDataFromFFI()
                        }
                    } else {
                        VStack(spacing: 0) {
                            // Chart with fullscreen button
                            ZStack(alignment: .bottomTrailing) {
                                let _ = print("📊 COINCRAB: Passing \(historicalData.count) data points to TradingViewChartView")
                                TradingViewChartView(data: historicalData, 
                                                   isPositive: cryptocurrency.quote.USD.percent_change_24h >= 0,
                                                   timeframe: selectedTimeframe.cmcTimeframe)
                                    .frame(maxWidth: .infinity)
                                    .frame(height: 300)
                                
                                // Fullscreen button
                                Button(action: {
                                    showingFullscreen = true
                                }) {
                                    Image(systemName: "arrow.up.left.and.arrow.down.right")
                                        .foregroundColor(.white)
                                        .padding(8)
                                        .background(Color.black.opacity(0.6))
                                        .clipShape(Circle())
                                }
                                .padding(8)
                            }
                            .padding(.horizontal, 16)
                        }
                    }
                    
                    // Chart stats
                    ChartStatsView(cryptocurrency: cryptocurrency, selectedTimeframe: selectedTimeframe)
                    
                    Spacer()
                }
        }
        .preferredColorScheme(.dark)
        .onAppear {
            loadHistoricalDataFromFFI()
            
            // Ensure portrait orientation is allowed for main view
            if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene {
                windowScene.requestGeometryUpdate(.iOS(interfaceOrientations: .allButUpsideDown))
            }
        }
        .fullScreenCover(isPresented: $showingFullscreen) {
            FullscreenChartView(
                data: historicalData,
                isPositive: cryptocurrency.quote.USD.percent_change_24h >= 0,
                timeframe: selectedTimeframe.cmcTimeframe,
                cryptocurrency: cryptocurrency,
                selectedTimeframe: selectedTimeframe
            )
        }
    }
    
    
    private func loadHistoricalDataFromFFI() {
        isLoading = true
        errorMessage = nil
        
        let symbol = cryptocurrency.symbol
        let timeframe = selectedTimeframe.cmcTimeframe
        
        print("📊 COINCRAB: Calling Rust get_historical_data() for \(symbol) (\(timeframe))")
        
        DispatchQueue.global(qos: .userInitiated).async {
            let symbolCStr = symbol.cString(using: .utf8)
            let timeframeCStr = timeframe.cString(using: .utf8)
            
            guard let symbolPtr = symbolCStr, let timeframePtr = timeframeCStr else {
                DispatchQueue.main.async {
                    self.errorMessage = "Failed to convert strings to C format"
                    self.isLoading = false
                }
                return
            }
            
            let resultCStr = get_historical_data(symbolPtr, timeframePtr)
            guard let resultPtr = resultCStr else {
                DispatchQueue.main.async {
                    self.errorMessage = "Failed to get historical data from Rust"
                    self.isLoading = false
                }
                return
            }
            let resultString = String(cString: resultPtr)
            
            print("📊 COINCRAB: Got historical data result from Rust: \(resultString.prefix(100))...")
            
            DispatchQueue.main.async {
                do {
                    let data = resultString.data(using: .utf8) ?? Data()
                    let result = try JSONDecoder().decode(HistoricalDataResult.self, from: data)
                    
                    if result.success {
                        self.historicalData = result.data.map { point in
                            ChartDataPoint(timestamp: point.timestamp, price: point.price, volume: point.volume)
                        }
                        print("📊 COINCRAB: Loaded \(result.data.count) data points for \(result.symbol ?? "Unknown")")
                    } else {
                        self.errorMessage = result.error ?? "Unknown error from server"
                        print("📊 COINCRAB: Server error: \(result.error ?? "Unknown")")
                    }
                } catch {
                    self.errorMessage = "Failed to parse response: \(error.localizedDescription)"
                    print("📊 COINCRAB: JSON parsing error: \(error.localizedDescription)")
                }
                
                self.isLoading = false
            }
        }
    }
    
    
    
}

struct ChartDataPoint {
    let timestamp: TimeInterval
    let price: Double
    let volume: Double?
}

struct HistoricalDataPoint: Codable {
    let timestamp: TimeInterval
    let price: Double
    let volume: Double?
    
    enum CodingKeys: String, CodingKey {
        case timestamp
        case price
        case volume
    }
}

struct HistoricalDataResult: Codable {
    let success: Bool
    let data: [HistoricalDataPoint]
    let error: String?
    let symbol: String?
    let timeframe: String?
}

struct CryptoHeaderView: View {
    let cryptocurrency: CryptoCurrency
    
    var body: some View {
        VStack(spacing: 12) {
            HStack(spacing: 12) {
                CryptoIcon(symbol: cryptocurrency.symbol)
                    .frame(width: 40, height: 40)
                
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 8) {
                        Text(cryptocurrency.symbol)
                            .font(.system(size: 24, weight: .bold))
                            .foregroundColor(.white)
                        
                        Text(cryptocurrency.name)
                            .font(.system(size: 16))
                            .foregroundColor(.gray)
                    }
                    
                    HStack(spacing: 4) {
                        AnimatedPriceView(
                            price: cryptocurrency.quote.USD.price,
                            cryptoId: cryptocurrency.symbol
                        )
                        .font(.system(size: 28, weight: .bold))
                    }
                }
                
                Spacer()
            }
            
            HStack(spacing: 16) {
                let change24h = cryptocurrency.quote.USD.percent_change_24h
                let isPositive = change24h >= 0
                
                HStack(spacing: 4) {
                    Image(systemName: isPositive ? "arrowtriangle.up.fill" : "arrowtriangle.down.fill")
                        .font(.system(size: 12))
                    Text("\(abs(change24h), specifier: "%.2f")%")
                        .font(.system(size: 16, weight: .semibold))
                }
                .foregroundColor(isPositive ? .green : .red)
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(isPositive ? Color.green.opacity(0.1) : Color.red.opacity(0.1))
                )
                
                Spacer()
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 16)
    }
}

struct TimeFrameSelectorView: View {
    @Binding var selectedTimeframe: CryptoChartView.TimeFrame
    
    var body: some View {
        HStack(spacing: 6) {
            ForEach(CryptoChartView.TimeFrame.allCases, id: \.self) { timeframe in
                Button(action: {
                    selectedTimeframe = timeframe
                }) {
                    Text(timeframe.rawValue)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(selectedTimeframe == timeframe ? .black : .gray)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 8)
                        .frame(minWidth: 40)
                        .background(
                            RoundedRectangle(cornerRadius: 16)
                                .fill(selectedTimeframe == timeframe ? Color.white : Color.gray.opacity(0.1))
                        )
                }
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}


struct ChartStatsView: View {
    let cryptocurrency: CryptoCurrency
    let selectedTimeframe: CryptoChartView.TimeFrame
    
    var body: some View {
        VStack(spacing: 16) {
            HStack {
                StatCard(title: "Market Cap", value: formatMarketCap(cryptocurrency.quote.USD.market_cap))
                StatCard(title: "Volume 24h", value: formatVolume(cryptocurrency.quote.USD.volume_24h))
            }
            
            HStack {
                StatCard(title: "1h Change", value: String(format: "%.2f%%", cryptocurrency.quote.USD.percent_change_1h), 
                        isPositive: cryptocurrency.quote.USD.percent_change_1h >= 0)
                StatCard(title: "7d Change", value: String(format: "%.2f%%", cryptocurrency.quote.USD.percent_change_7d), 
                        isPositive: cryptocurrency.quote.USD.percent_change_7d >= 0)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 20)
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
    
    private func formatVolume(_ volume: Double) -> String {
        if volume >= 1_000_000_000 {
            return String(format: "$%.2fB", volume / 1_000_000_000)
        } else if volume >= 1_000_000 {
            return String(format: "$%.2fM", volume / 1_000_000)
        } else {
            return String(format: "$%.0f", volume)
        }
    }
}

struct StatCard: View {
    let title: String
    let value: String
    let isPositive: Bool?
    
    init(title: String, value: String, isPositive: Bool? = nil) {
        self.title = title
        self.value = value
        self.isPositive = isPositive
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.system(size: 14))
                .foregroundColor(.gray)
            
            Text(value)
                .font(.system(size: 16, weight: .semibold))
                .foregroundColor(
                    isPositive == nil ? .white :
                    isPositive! ? .green : .red
                )
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(Color.gray.opacity(0.1))
        )
    }
}

struct ChartLoadingView: View {
    var body: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.2)
                .progressViewStyle(CircularProgressViewStyle(tint: .blue))
            
            Text("Loading chart...")
                .font(.body)
                .foregroundColor(.gray)
        }
        .frame(height: 300)
    }
}

struct ChartErrorView: View {
    let message: String
    let onRetry: () -> Void
    
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 40))
                .foregroundColor(.red)
            
            Text("Error loading chart")
                .font(.headline)
                .foregroundColor(.white)
            
            Text(message)
                .font(.body)
                .foregroundColor(.gray)
                .multilineTextAlignment(.center)
            
            Button("Retry", action: onRetry)
                .font(.system(size: 16, weight: .medium))
                .foregroundColor(.white)
                .padding(.horizontal, 24)
                .padding(.vertical, 12)
                .background(Color.blue)
                .cornerRadius(8)
        }
        .frame(height: 300)
        .padding(.horizontal, 32)
    }
}

struct FullscreenChartView: View {
    let data: [ChartDataPoint]
    let isPositive: Bool
    let timeframe: String
    let cryptocurrency: CryptoCurrency
    @State private var selectedTimeframe: CryptoChartView.TimeFrame
    @State private var historicalData: [ChartDataPoint]
    @State private var isLoading = false
    @State private var errorMessage: String?
    @Environment(\.dismiss) private var dismiss
    
    init(data: [ChartDataPoint], isPositive: Bool, timeframe: String, cryptocurrency: CryptoCurrency, selectedTimeframe: CryptoChartView.TimeFrame) {
        self.data = data
        self.isPositive = isPositive
        self.timeframe = timeframe
        self.cryptocurrency = cryptocurrency
        self._selectedTimeframe = State(initialValue: selectedTimeframe)
        self._historicalData = State(initialValue: data)
    }
    
    var body: some View {
        GeometryReader { geometry in
            ZStack {
                Color.black.ignoresSafeArea()
                
                VStack(spacing: 20) {
                    // Header with close button and crypto info
                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            HStack(spacing: 8) {
                                Image(systemName: "bitcoinsign.circle.fill")
                                    .foregroundColor(.orange)
                                    .font(.title2)
                                
                                Text(cryptocurrency.symbol)
                                    .font(.title2)
                                    .fontWeight(.bold)
                                    .foregroundColor(.white)
                                
                                Text(cryptocurrency.name)
                                    .font(.subheadline)
                                    .foregroundColor(.gray)
                            }
                            
                            HStack(spacing: 12) {
                                AnimatedPriceView(
                                    price: cryptocurrency.quote.USD.price,
                                    cryptoId: cryptocurrency.symbol
                                )
                                .font(.title)
                                .fontWeight(.semibold)
                                
                                HStack(spacing: 4) {
                                    Image(systemName: cryptocurrency.quote.USD.percent_change_24h >= 0 ? "triangle.fill" : "triangle.fill")
                                        .rotationEffect(.degrees(cryptocurrency.quote.USD.percent_change_24h >= 0 ? 0 : 180))
                                        .foregroundColor(cryptocurrency.quote.USD.percent_change_24h >= 0 ? .green : .red)
                                        .font(.caption)
                                    
                                    Text("\(abs(cryptocurrency.quote.USD.percent_change_24h), specifier: "%.2f")%")
                                        .font(.subheadline)
                                        .fontWeight(.medium)
                                        .foregroundColor(cryptocurrency.quote.USD.percent_change_24h >= 0 ? .green : .red)
                                }
                            }
                        }
                        
                        Spacer()
                        
                        Button(action: {
                            dismiss()
                        }) {
                            Image(systemName: "xmark.circle.fill")
                                .font(.title2)
                                .foregroundColor(.gray)
                        }
                    }
                    .padding(.horizontal, 20)
                    .padding(.top, 10)
                    
                    // Time frame selector
                    TimeFrameSelectorView(selectedTimeframe: $selectedTimeframe)
                        .onChange(of: selectedTimeframe) { _, newValue in
                            loadHistoricalDataFromFFI()
                        }
                        .padding(.horizontal, 20)
                    
                    // Fullscreen chart
                    if isLoading {
                        ProgressView()
                            .progressViewStyle(CircularProgressViewStyle(tint: .white))
                            .scaleEffect(1.5)
                    } else if let errorMessage = errorMessage {
                        VStack(spacing: 16) {
                            Image(systemName: "exclamationmark.triangle")
                                .font(.system(size: 48))
                                .foregroundColor(.red)
                            
                            Text("Chart Error")
                                .font(.title2)
                                .fontWeight(.medium)
                                .foregroundColor(.white)
                            
                            Text(errorMessage)
                                .font(.body)
                                .foregroundColor(.gray)
                                .multilineTextAlignment(.center)
                            
                            Button("Retry") {
                                loadHistoricalDataFromFFI()
                            }
                            .padding(.horizontal, 24)
                            .padding(.vertical, 12)
                            .background(Color.blue)
                            .foregroundColor(.white)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    } else {
                        // Calculate optimal chart height based on available space
                        let isLandscape = geometry.size.width > geometry.size.height
                        let chartHeight = isLandscape ? geometry.size.height * 0.65 : max(geometry.size.height * 0.5, 300)
                        
                        TradingViewChartView(data: historicalData,
                                           isPositive: cryptocurrency.quote.USD.percent_change_24h >= 0,
                                           timeframe: selectedTimeframe.cmcTimeframe)
                            .frame(maxWidth: .infinity)
                            .frame(height: chartHeight)
                            .padding(.horizontal, isLandscape ? 12 : 16)
                            .id("fullscreen-chart-\(selectedTimeframe.rawValue)") // Force refresh on timeframe change
                    }
                    
                    Spacer()
                }
            }
        }
        .preferredColorScheme(.dark)
        .statusBarHidden(true) // Hide status bar for true fullscreen
        .onAppear {
            // Request landscape orientation
            if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene {
                windowScene.requestGeometryUpdate(.iOS(interfaceOrientations: .landscape))
            }
        }
        .onDisappear {
            // Allow all orientations when dismissed
            DispatchQueue.main.async {
                if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene {
                    windowScene.requestGeometryUpdate(.iOS(interfaceOrientations: .allButUpsideDown))
                }
                
                // Force portrait after a brief delay
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    UIDevice.current.setValue(UIInterfaceOrientation.portrait.rawValue, forKey: "orientation")
                }
            }
        }
    }
    
    private func loadHistoricalDataFromFFI() {
        isLoading = true
        errorMessage = nil
        
        let symbol = cryptocurrency.symbol
        let timeframe = selectedTimeframe.cmcTimeframe
        
        DispatchQueue.global(qos: .userInitiated).async {
            let symbolCStr = symbol.cString(using: .utf8)
            let timeframeCStr = timeframe.cString(using: .utf8)
            
            guard let symbolPtr = symbolCStr, let timeframePtr = timeframeCStr else {
                DispatchQueue.main.async {
                    self.errorMessage = "Failed to convert parameters"
                    self.isLoading = false
                }
                return
            }
            
            let resultCStr = get_historical_data(symbolPtr, timeframePtr)
            
            guard let resultCStr = resultCStr else {
                DispatchQueue.main.async {
                    self.errorMessage = "Failed to get historical data from FFI"
                    self.isLoading = false
                }
                return
            }
            
            let resultString = String(cString: resultCStr)
            free_string(resultCStr)
            
            do {
                let result = try JSONDecoder().decode(HistoricalDataResult.self, from: resultString.data(using: .utf8) ?? Data())
                
                DispatchQueue.main.async {
                    if result.success {
                        self.historicalData = result.data.map { point in
                            ChartDataPoint(timestamp: point.timestamp, price: point.price, volume: point.volume)
                        }
                        self.errorMessage = nil
                    } else {
                        self.errorMessage = result.error ?? "Unknown error occurred"
                    }
                    self.isLoading = false
                }
            } catch {
                DispatchQueue.main.async {
                    self.errorMessage = "Failed to parse response: \(error.localizedDescription)"
                    self.isLoading = false
                }
            }
        }
    }
}