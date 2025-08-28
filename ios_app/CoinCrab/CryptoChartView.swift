import SwiftUI
import Foundation

struct CryptoChartView: View {
    let cryptocurrency: CryptoCurrency
    @State private var selectedTimeframe: TimeFrame = .day
    @State private var historicalData: [ChartDataPoint] = []
    @State private var isLoading = false
    @State private var errorMessage: String?
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
                            loadHistoricalData()
                        }
                    
                    // Chart container
                    if isLoading {
                        ChartLoadingView()
                    } else if let errorMessage = errorMessage {
                        ChartErrorView(message: errorMessage) {
                            loadHistoricalData()
                        }
                    } else {
                        LightweightChartView(data: historicalData, 
                                           isPositive: cryptocurrency.quote.USD.percent_change_24h >= 0)
                            .frame(maxWidth: .infinity)
                            .frame(height: 300)
                            .padding(.horizontal, 16)
                    }
                    
                    // Chart stats
                    ChartStatsView(cryptocurrency: cryptocurrency, selectedTimeframe: selectedTimeframe)
                    
                    Spacer()
                }
        }
        .preferredColorScheme(.dark)
        .onAppear {
            loadHistoricalData()
        }
    }
    
    private func loadHistoricalData() {
        isLoading = true
        errorMessage = nil
        
        Task {
            let cryptoSymbol = self.cryptocurrency.symbol
            let timeframe = self.selectedTimeframe
            
            // Make HTTP request to crypto_server instead of direct CMC API calls
            let symbol = cryptoSymbol.lowercased()
            let timeframeStr = timeframe.cmcTimeframe
            let serverURL = "http://127.0.0.1:8080/api/historical/\(symbol)?timeframe=\(timeframeStr)"
            
            NSLog("🔍 COINCRAB: Calling crypto_server for \(cryptoSymbol) with URL: \(serverURL)")
            print("🔍 Calling crypto_server for \(cryptoSymbol) with URL: \(serverURL)")
            
            guard let url = URL(string: serverURL) else {
                DispatchQueue.main.async {
                    NSLog("🚨 COINCRAB: Invalid server URL, using mock data")
                    print("Invalid server URL, using mock data")
                    self.generateMockData()
                    self.isLoading = false
                }
                return
            }
            
            do {
                NSLog("🧪 COINCRAB: About to call server endpoint...")
                let (data, response) = try await URLSession.shared.data(from: url)
                
                guard let httpResponse = response as? HTTPURLResponse else {
                    DispatchQueue.main.async {
                        NSLog("🚨 COINCRAB: Invalid response type, using mock data")
                        print("Invalid response type, using mock data")
                        self.generateMockData()
                        self.isLoading = false
                    }
                    return
                }
                
                NSLog("🧪 COINCRAB: Server responded with status: \(httpResponse.statusCode)")
                
                guard httpResponse.statusCode == 200 else {
                    DispatchQueue.main.async {
                        NSLog("🚨 COINCRAB: Server error \(httpResponse.statusCode), using mock data")
                        print("Server error \(httpResponse.statusCode), using mock data")
                        self.generateMockData()
                        self.isLoading = false
                    }
                    return
                }
                
                let resultString = String(data: data, encoding: .utf8) ?? ""
                NSLog("🔍 COINCRAB: Got server result: \(String(resultString.prefix(100)))...")
                
                // Check for server-side errors in the JSON response
                if resultString.contains("\"success\":false") {
                    DispatchQueue.main.async {
                        NSLog("🚨 COINCRAB: Server returned error, using mock data")
                        print("Server returned error: \(resultString)")
                        self.generateMockData()
                        self.isLoading = false
                    }
                    return
                }
                
                guard let responseData = resultString.data(using: .utf8) else {
                    DispatchQueue.main.async {
                        NSLog("🚨 COINCRAB: Invalid response format, using mock data")
                        print("Invalid response format, using mock data")
                        self.generateMockData()
                        self.isLoading = false
                    }
                    return
                }
                
                // Parse the server response
                NSLog("🔍 COINCRAB: About to parse JSON, length: \(responseData.count) bytes")
                
                do {
                    let result = try JSONDecoder().decode(HistoricalDataResult.self, from: responseData)
                NSLog("🔍 COINCRAB: JSON parsing successful, success=\(result.success), data points=\(result.data.count)")
                
                DispatchQueue.main.async {
                    if result.success {
                        NSLog("🎉 COINCRAB: Processing \(result.data.count) data points for chart")
                        self.historicalData = result.data.map { dataPoint in
                            ChartDataPoint(
                                timestamp: dataPoint.timestamp,
                                price: dataPoint.price
                            )
                        }.sorted { $0.timestamp < $1.timestamp }
                        NSLog("🎉 COINCRAB: Chart updated with real data, first price: $\(result.data.first?.price ?? 0)")
                        self.errorMessage = nil
                    } else {
                        NSLog("🚨 COINCRAB: API returned error, using mock data: \(result.error ?? "Unknown error")")
                        print("API returned error, using mock data: \(result.error ?? "Unknown error")")
                        self.generateMockData()
                    }
                    self.isLoading = false
                }
                } catch {
                    // Fallback to mock data if JSON parsing fails
                    NSLog("🚨 COINCRAB: JSON parsing failed: \(error)")
                    NSLog("🚨 COINCRAB: Raw response: \(resultString.prefix(500))")
                    DispatchQueue.main.async {
                        print("API parsing failed, using mock data: \(error)")
                        self.generateMockData()
                        self.isLoading = false
                    }
                }
                
            } catch {
                // Fallback to mock data if HTTP request fails
                NSLog("🚨 COINCRAB: HTTP request failed: \(error)")
                DispatchQueue.main.async {
                    print("HTTP request failed, using mock data: \(error)")
                    self.generateMockData()
                    self.isLoading = false
                }
            }
        }
    }
    
    
    private func generateMockData() {
        // Generate mock data that resembles the uploaded chart pattern
        let basePrice = cryptocurrency.quote.USD.price
        
        var data: [ChartDataPoint] = []
        let pointCount = getPointCount()
        
        for i in 0..<pointCount {
            let progress = Double(i) / Double(pointCount - 1)
            let timeInterval = getTimeInterval(for: i, total: pointCount)
            
            // Create a pattern similar to the uploaded chart (dramatic rise)
            var priceMultiplier: Double
            if progress < 0.7 {
                // Gradual decline in first 70%
                priceMultiplier = 1.0 - (progress * 0.4) // Down to 0.6
            } else {
                // Sharp rise in last 30%
                let riseProgress = (progress - 0.7) / 0.3
                priceMultiplier = 0.6 + (riseProgress * 0.9) // Up to 1.5
            }
            
            let price = basePrice * priceMultiplier
            let timestamp = Date().timeIntervalSince1970 - timeInterval
            
            data.append(ChartDataPoint(timestamp: timestamp, price: price))
        }
        
        self.historicalData = data
    }
    
    private func getPointCount() -> Int {
        switch selectedTimeframe {
        case .hour: return 12 // 5-minute intervals
        case .day: return 24 // 1-hour intervals
        case .week: return 28 // 6-hour intervals
        case .month: return 30 // 1-day intervals
        case .quarter: return 90 // 1-day intervals
        case .year: return 365 // 1-day intervals
        case .all: return 100 // Variable intervals
        }
    }
    
    private func getTimeInterval(for index: Int, total: Int) -> TimeInterval {
        let totalInterval: TimeInterval
        switch selectedTimeframe {
        case .hour: totalInterval = 3600 // 1 hour
        case .day: totalInterval = 86400 // 24 hours
        case .week: totalInterval = 604800 // 7 days
        case .month: totalInterval = 2592000 // 30 days
        case .quarter: totalInterval = 7776000 // 90 days
        case .year: totalInterval = 31536000 // 365 days
        case .all: totalInterval = 157680000 // ~5 years
        }
        
        return totalInterval * Double(total - index) / Double(total)
    }
}

struct ChartDataPoint {
    let timestamp: TimeInterval
    let price: Double
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
                        Text("$\(cryptocurrency.quote.USD.price, specifier: "%.2f")")
                            .font(.system(size: 28, weight: .bold))
                            .foregroundColor(.white)
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
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(CryptoChartView.TimeFrame.allCases, id: \.self) { timeframe in
                    Button(action: {
                        selectedTimeframe = timeframe
                    }) {
                        Text(timeframe.rawValue)
                            .font(.system(size: 14, weight: .medium))
                            .foregroundColor(selectedTimeframe == timeframe ? .black : .gray)
                            .padding(.horizontal, 16)
                            .padding(.vertical, 8)
                            .background(
                                RoundedRectangle(cornerRadius: 20)
                                    .fill(selectedTimeframe == timeframe ? Color.white : Color.gray.opacity(0.1))
                            )
                    }
                }
            }
            .padding(.horizontal, 16)
        }
        .padding(.vertical, 8)
    }
}

struct LightweightChartView: View {
    let data: [ChartDataPoint]
    let isPositive: Bool
    
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 12)
                .fill(Color.gray.opacity(0.05))
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(Color.gray.opacity(0.1), lineWidth: 1)
                )
            
            if data.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "chart.line.uptrend.xyaxis")
                        .font(.system(size: 40))
                        .foregroundColor(.gray)
                    Text("No chart data available")
                        .font(.body)
                        .foregroundColor(.gray)
                }
            } else {
                // For now, we'll use a custom SwiftUI chart
                // This will be replaced with TradingView Lightweight Charts
                CustomLineChart(data: data, isPositive: isPositive)
                    .padding(20)
            }
        }
    }
}

struct CustomLineChart: View {
    let data: [ChartDataPoint]
    let isPositive: Bool
    
    var body: some View {
        GeometryReader { geometry in
            let width = geometry.size.width
            let height = geometry.size.height
            
            if let minPrice = data.map(\.price).min(),
               let maxPrice = data.map(\.price).max(),
               maxPrice > minPrice {
                
                Path { path in
                    for (index, point) in data.enumerated() {
                        let x = width * CGFloat(index) / CGFloat(data.count - 1)
                        let normalizedPrice = (point.price - minPrice) / (maxPrice - minPrice)
                        let y = height * (1 - CGFloat(normalizedPrice))
                        
                        if index == 0 {
                            path.move(to: CGPoint(x: x, y: y))
                        } else {
                            path.addLine(to: CGPoint(x: x, y: y))
                        }
                    }
                }
                .stroke(
                    LinearGradient(
                        colors: isPositive ? [.green, .green.opacity(0.8)] : [.red, .red.opacity(0.8)],
                        startPoint: .leading,
                        endPoint: .trailing
                    ),
                    style: StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round)
                )
                
                // Add fill gradient
                Path { path in
                    for (index, point) in data.enumerated() {
                        let x = width * CGFloat(index) / CGFloat(data.count - 1)
                        let normalizedPrice = (point.price - minPrice) / (maxPrice - minPrice)
                        let y = height * (1 - CGFloat(normalizedPrice))
                        
                        if index == 0 {
                            path.move(to: CGPoint(x: x, y: y))
                        } else {
                            path.addLine(to: CGPoint(x: x, y: y))
                        }
                    }
                    
                    // Close the path to create a fill area
                    path.addLine(to: CGPoint(x: width, y: height))
                    path.addLine(to: CGPoint(x: 0, y: height))
                    path.closeSubpath()
                }
                .fill(
                    LinearGradient(
                        colors: isPositive ? 
                            [Color.green.opacity(0.3), Color.green.opacity(0.05)] :
                            [Color.red.opacity(0.3), Color.red.opacity(0.05)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
            }
        }
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