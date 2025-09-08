import SwiftUI
import WebKit
import Foundation

extension String {
    func appendToFile(url: URL) throws {
        if FileManager.default.fileExists(atPath: url.path) {
            let fileHandle = try FileHandle(forWritingTo: url)
            fileHandle.seekToEndOfFile()
            fileHandle.write(self.data(using: .utf8)!)
            fileHandle.closeFile()
        } else {
            try self.write(to: url, atomically: true, encoding: .utf8)
        }
    }
}

struct TradingViewChartView: UIViewRepresentable {
    let data: [ChartDataPoint]
    let isPositive: Bool
    let timeframe: String
    
    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        
        // Enable JavaScript
        configuration.preferences.javaScriptEnabled = true
        
        // Allow insecure content (mixed content)
        configuration.preferences.setValue(true, forKey: "allowFileAccessFromFileURLs")
        configuration.setValue(true, forKey: "allowUniversalAccessFromFileURLs")
        
        // Add logging message handler
        let coordinator = Coordinator()
        configuration.userContentController.add(coordinator, name: "logging")
        
        // Create WebView
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.scrollView.isScrollEnabled = false
        webView.scrollView.bounces = false
        webView.backgroundColor = UIColor.clear
        webView.isOpaque = false
        
        return webView
    }
    
    func makeCoordinator() -> Coordinator {
        Coordinator()
    }
    
    class Coordinator: NSObject, WKScriptMessageHandler {
        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            if message.name == "logging" {
                if let body = message.body as? [String: String],
                   let timestamp = body["timestamp"],
                   let logMessage = body["message"] {
                    let logEntry = "\(timestamp): \(logMessage)"
                    print("📊 TradingView JS Log: \(logEntry)")
                    
                    // Write to Documents directory
                    if let documentsPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first {
                        let logFile = documentsPath.appendingPathComponent("tradingview_debug.log")
                        try? (logEntry + "\n").appendToFile(url: logFile)
                    }
                }
            }
        }
    }
    
    func updateUIView(_ webView: WKWebView, context: Context) {
        let htmlContent = generateHTMLContent()
        let baseURL = URL(string: "https://unpkg.com/")
        webView.loadHTMLString(htmlContent, baseURL: baseURL)
    }
    
    private func generateHTMLContent() -> String {
        let (priceData, volumeData) = formatDataForTradingView()
        let chartColor = isPositive ? "#00C851" : "#FF4444"
        let fillColor = isPositive ? "rgba(0, 200, 81, 0.1)" : "rgba(255, 68, 68, 0.1)"
        
        print("📊 TradingView HTML: Generated priceData for JS: \(priceData.prefix(100))...")
        print("📊 TradingView HTML: Generated volumeData for JS: \(volumeData.prefix(100))...")
        
        return """
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <style>
                body {
                    margin: 0;
                    padding: 0;
                    background-color: transparent;
                    overflow: hidden;
                }
                #chartContainer {
                    width: 100%;
                    height: 100vh;
                    background-color: transparent;
                    box-sizing: border-box;
                }
                /* Smaller font for time axis labels to allow more tick marks */
                .tv-lightweight-charts table tr td {
                    font-size: 10px !important;
                }
                /* Make fullscreen icon smaller */
                .tv-lightweight-charts canvas + div > div:last-child {
                    transform: scale(0.6) !important;
                    transform-origin: bottom right !important;
                }
                /* Alternative selector for fullscreen button */
                .tv-lightweight-charts div[style*="position: absolute"][style*="right: 8px"][style*="bottom: 8px"] {
                    transform: scale(0.6) !important;
                    transform-origin: bottom right !important;
                }
            </style>
        </head>
        <body>
            <div id="chartContainer"></div>
            
            <script src="https://unpkg.com/lightweight-charts@latest/dist/lightweight-charts.standalone.production.js" onload="logToFile('TradingView v5.0 library loaded')" onerror="logToFile('Failed to load v5.0, trying fallback...'); loadFromUnpkg();"></script>
            <script>
                function loadFromUnpkg() {
                    const script = document.createElement('script');
                    script.src = 'https://unpkg.com/lightweight-charts/dist/lightweight-charts.standalone.production.js';
                    script.onload = () => logToFile('TradingView library loaded from unpkg');
                    script.onerror = () => logToFile('ERROR: All CDN sources failed to load');
                    document.head.appendChild(script);
                }
            </script>
            <script>
                // Set up logging function that writes to both console and creates visible feedback
                function logToFile(message) {
                    console.log(message);
                    
                    // Only log to filesystem, no overlay
                    const timestamp = new Date().toISOString().substr(11, 8);
                    
                    // Write to iOS app documents directory via Swift
                    try {
                        window.webkit.messageHandlers.logging.postMessage({
                            timestamp: timestamp,
                            message: message
                        });
                    } catch (e) {
                        // Silently fail if message handler not available
                    }
                }
                
                try {
                    // Show immediate visual feedback that JavaScript is running
                    document.body.style.backgroundColor = '#333333';
                    logToFile('TradingView: JavaScript is executing!');
                    
                    // Wait for the library to load
                    function waitForLibraryAndInit() {
                        logToFile('TradingView: Checking if library is loaded...');
                        
                        if (typeof LightweightCharts === 'undefined') {
                            logToFile('TradingView: Library not loaded yet, waiting...');
                            setTimeout(waitForLibraryAndInit, 500);
                            return;
                        }
                        
                        logToFile('TradingView: Library loaded! LightweightCharts is available');
                        initializeChart();
                    }
                    
                    // Initialize chart once library is loaded
                    function initializeChart() {
                        logToFile('TradingView: Timeout callback executing...');
                        const container = document.getElementById('chartContainer');
                        
                        if (!container) {
                            logToFile('ERROR: Container not found!');
                            document.body.innerHTML = '<h1 style="color: red;">Container not found</h1>';
                            return;
                        }
                        
                        if (!window.LightweightCharts) {
                            logToFile('ERROR: LightweightCharts not loaded!');
                            document.body.innerHTML = '<h1 style="color: red;">LightweightCharts not loaded</h1>';
                            return;
                        }
                        
                        logToFile('SUCCESS: Both container and LightweightCharts available');
                        
                        let chart, areaSeries;
                        try {
                            logToFile('TradingView: Creating chart...');
                            logToFile('Container dimensions: ' + container.clientWidth + 'x' + container.clientHeight);
                            
                            const chartWidth = container.clientWidth || 400;
                            const chartHeight = container.clientHeight || 300;
                            
                            logToFile('Using chart dimensions: ' + chartWidth + 'x' + chartHeight);
                            
                            // Create chart with proper dark theme styling
                            chart = LightweightCharts.createChart(container, {
                                width: chartWidth,
                                height: chartHeight,
                                layout: {
                                    background: { type: 'solid', color: '#000000' },
                                    textColor: '#ffffff',
                                    attributionLogo: false,
                                },
                                grid: {
                                    vertLines: { color: 'rgba(255, 255, 255, 0.1)' },
                                    horzLines: { color: 'rgba(255, 255, 255, 0.1)' },
                                },
                                timeScale: {
                                    timeVisible: true,
                                    secondsVisible: false,
                                    borderColor: 'rgba(255, 255, 255, 0.2)',
                                    rightOffset: 5,
                                    barSpacing: 8,
                                    fixLeftEdge: true,
                                    fixRightEdge: true,
                                    lockVisibleTimeRangeOnResize: true,
                                    rightBarStaysOnScroll: true,
                                    shiftVisibleRangeOnNewBar: false,
                                    tickMarkFormatter: (time, tickMarkType, locale) => {
                                        const date = new Date(time * 1000);
                                        const timeframe = '\(timeframe)';
                                        
                                        // Custom formatting based on timeframe
                                        if (timeframe === '1h') {
                                            // 1H: Show time as hh:mm (changed from hh:mm:ss)
                                            return date.toLocaleTimeString('en-US', { 
                                                hour: '2-digit', 
                                                minute: '2-digit',
                                                hour12: false 
                                            });
                                        } else if (timeframe === '24h') {
                                            // 24H: Show time as hh:mm
                                            return date.toLocaleTimeString('en-US', { 
                                                hour: '2-digit', 
                                                minute: '2-digit',
                                                hour12: false 
                                            });
                                        } else {
                                            // All others: Show date as dd/mm
                                            const day = String(date.getDate()).padStart(2, '0');
                                            const month = String(date.getMonth() + 1).padStart(2, '0');
                                            return `${day}/${month}`;
                                        }
                                    }
                                },
                                rightPriceScale: {
                                    borderColor: 'rgba(255, 255, 255, 0.2)',
                                },
                                handleScroll: {
                                    mouseWheel: true,
                                    pressedMouseMove: true,
                                    horzTouchDrag: true,
                                    vertTouchDrag: false,
                                },
                                handleScale: {
                                    axisPressedMouseMove: true,
                                    mouseWheel: true,
                                    pinch: true,
                                },
                            });
                            logToFile('TradingView: Chart created successfully');
                            
                            // Price formatting will be applied after data is loaded
                            
                            // Debug: Show what methods are actually available
                            logToFile('TradingView: Chart object type: ' + typeof chart);
                            logToFile('TradingView: Chart prototype methods: ' + Object.getOwnPropertyNames(Object.getPrototypeOf(chart)));
                            
                            // Check for different possible method names
                            const allMethods = [];
                            for (let prop in chart) {
                                if (typeof chart[prop] === 'function') {
                                    allMethods.push(prop);
                                }
                            }
                            logToFile('TradingView v5.0: All function methods: ' + allMethods.join(', '));
                            
                            // Check TradingView v5.0 constants and methods
                            logToFile('TradingView v5.0: addSeries method: ' + (typeof chart.addSeries));
                            logToFile('TradingView v5.0: BaselineSeries constant: ' + (typeof LightweightCharts.BaselineSeries));
                            logToFile('TradingView v5.0: AreaSeries constant: ' + (typeof LightweightCharts.AreaSeries));
                            logToFile('TradingView v5.0: LineSeries constant: ' + (typeof LightweightCharts.LineSeries));
                            
                            // Check if old methods still exist
                            logToFile('TradingView v5.0: addBaselineSeries (old): ' + (typeof chart.addBaselineSeries));
                            logToFile('TradingView v5.0: addLineSeries (old): ' + (typeof chart.addLineSeries));
                            logToFile('TradingView v5.0: addAreaSeries (old): ' + (typeof chart.addAreaSeries));
                            
                            // Try different API patterns that might exist
                            let seriesCreated = false;
                            
                            // Try TradingView v5.0 API with BaselineSeries constant
                            if (chart.addSeries && typeof chart.addSeries === 'function') {
                                try {
                                    logToFile('Using TradingView v5.0 API with BaselineSeries constant');
                                    
                                    // Get start price for baseline - need to access it here first
                                    const priceData = \(priceData);
                                    const volumeData = \(volumeData);
                                    let baselinePrice = 108000; // Default baseline closer to BTC range
                                    
                                    logToFile('Raw priceData type: ' + typeof priceData);
                                    logToFile('Raw priceData: ' + JSON.stringify(priceData).substring(0, 200));
                                    
                                    if (priceData && priceData.length > 0) {
                                        const firstPoint = priceData[0];
                                        logToFile('First data point: ' + JSON.stringify(firstPoint));
                                        
                                        // Try different property names
                                        baselinePrice = firstPoint.value || firstPoint.price || firstPoint.y || 108000;
                                        logToFile('Extracted baseline price: $' + baselinePrice);
                                    } else {
                                        logToFile('No chart data available for baseline');
                                    }
                                    
                                    // Apply price formatter using localization as per API documentation
                                    try {
                                        const myPriceFormatter = function(price) {
                                            // Calculate precision based on the actual price being formatted
                                            const precision = price >= 1000 ? 0 : price >= 100 ? 1 : price >= 10 ? 2 : price >= 1 ? 3 : price >= 0.01 ? 4 : 6;
                                            const fixed = price.toFixed(precision);
                                            const [whole, decimal] = fixed.split('.');
                                            const withCommas = whole.replace(/\\B(?=(\\d{3})+(?!\\d))/g, ',');
                                            const result = decimal && decimal !== '00' && decimal !== '0' ? withCommas + '.' + decimal : withCommas;
                                            return result;
                                        };
                                        
                                        logToFile('Applying price formatter with dynamic precision based on price value');
                                        
                                        chart.applyOptions({
                                            localization: {
                                                priceFormatter: myPriceFormatter
                                            }
                                        });
                                        logToFile('Price formatter applied successfully');
                                    } catch (formatError) {
                                        logToFile('Price formatter error: ' + formatError.message);
                                    }
                                    
                                    // Check if BaselineSeries constant is available in LightweightCharts namespace
                                    if (typeof LightweightCharts.BaselineSeries !== 'undefined') {
                                        logToFile('BaselineSeries constant found, creating baseline series');
                                        areaSeries = chart.addSeries(LightweightCharts.BaselineSeries, {
                                            baseValue: { type: 'price', value: baselinePrice },  // Revert to object format
                                            topLineColor: '#00FF00',
                                            topFillColor1: 'rgba(0, 255, 0, 0.4)',
                                            topFillColor2: 'rgba(0, 255, 0, 0.1)',
                                            bottomLineColor: '#FF0000', 
                                            bottomFillColor1: 'rgba(255, 0, 0, 0.4)',
                                            bottomFillColor2: 'rgba(255, 0, 0, 0.1)',
                                            lineWidth: 2,
                                            lineStyle: 0,
                                            priceFormat: {
                                                type: 'price',
                                                precision: baselinePrice >= 1000 ? 0 : baselinePrice >= 100 ? 1 : baselinePrice >= 10 ? 2 : baselinePrice >= 1 ? 3 : baselinePrice >= 0.01 ? 4 : 6,
                                                minMove: baselinePrice >= 1000 ? 1 : baselinePrice >= 100 ? 0.1 : baselinePrice >= 10 ? 0.01 : baselinePrice >= 1 ? 0.001 : baselinePrice >= 0.01 ? 0.0001 : 0.000001,
                                                formatter: function(price) {
                                                    const precision = baselinePrice >= 1000 ? 0 : baselinePrice >= 100 ? 1 : baselinePrice >= 10 ? 2 : baselinePrice >= 1 ? 3 : baselinePrice >= 0.01 ? 4 : 6;
                                                    const fixed = price.toFixed(precision);
                                                    const [whole, decimal] = fixed.split('.');
                                                    const withCommas = whole.replace(/\\B(?=(\\d{3})+(?!\\d))/g, ',');
                                                    return decimal ? withCommas + '.' + decimal : withCommas;
                                                }
                                            },
                                        });
                                        logToFile('SUCCESS: TradingView v5.0 Baseline series created with start price: $' + baselinePrice.toFixed(2));
                                        logToFile('Baseline series type: ' + typeof areaSeries);
                                        logToFile('Baseline configuration - baseValue: ' + baselinePrice);
                                        seriesCreated = true;
                                    } else {
                                        logToFile('BaselineSeries constant not found, trying AreaSeries');
                                        if (typeof LightweightCharts.AreaSeries !== 'undefined') {
                                            areaSeries = chart.addSeries(LightweightCharts.AreaSeries, {
                                                lineColor: '#00FF00',
                                                topColor: 'rgba(0, 255, 0, 0.1)',
                                                bottomColor: 'rgba(0, 0, 0, 0)',
                                                lineWidth: 3,
                                                priceFormat: {
                                                    type: 'price',
                                                    precision: baselinePrice >= 1000 ? 0 : baselinePrice >= 100 ? 1 : baselinePrice >= 10 ? 2 : baselinePrice >= 1 ? 3 : baselinePrice >= 0.01 ? 4 : 6,
                                                    minMove: baselinePrice >= 1000 ? 1 : baselinePrice >= 100 ? 0.1 : baselinePrice >= 10 ? 0.01 : baselinePrice >= 1 ? 0.001 : baselinePrice >= 0.01 ? 0.0001 : 0.000001,
                                                    formatter: function(price) {
                                                        const precision = baselinePrice >= 1000 ? 0 : baselinePrice >= 100 ? 1 : baselinePrice >= 10 ? 2 : baselinePrice >= 1 ? 3 : baselinePrice >= 0.01 ? 4 : 6;
                                                        const fixed = price.toFixed(precision);
                                                        const [whole, decimal] = fixed.split('.');
                                                        const withCommas = whole.replace(/\\B(?=(\\d{3})+(?!\\d))/g, ',');
                                                        return decimal ? withCommas + '.' + decimal : withCommas;
                                                    }
                                                },
                                            });
                                            logToFile('SUCCESS: TradingView v5.0 Area series created!');
                                            seriesCreated = true;
                                        } else {
                                            logToFile('Neither BaselineSeries nor AreaSeries constants found');
                                        }
                                    }
                                } catch (seriesError) {
                                    logToFile('TradingView v5.0 series creation failed: ' + seriesError.message);
                                    // Fallback to LineSeries
                                    try {
                                        if (typeof LightweightCharts.LineSeries !== 'undefined') {
                                            areaSeries = chart.addSeries(LightweightCharts.LineSeries, {
                                                color: '#00FF00',
                                                lineWidth: 3,
                                                priceFormat: {
                                                    type: 'price',
                                                    precision: baselinePrice >= 1000 ? 0 : baselinePrice >= 100 ? 1 : baselinePrice >= 10 ? 2 : baselinePrice >= 1 ? 3 : baselinePrice >= 0.01 ? 4 : 6,
                                                    minMove: baselinePrice >= 1000 ? 1 : baselinePrice >= 100 ? 0.1 : baselinePrice >= 10 ? 0.01 : baselinePrice >= 1 ? 0.001 : baselinePrice >= 0.01 ? 0.0001 : 0.000001,
                                                    formatter: function(price) {
                                                        const precision = baselinePrice >= 1000 ? 0 : baselinePrice >= 100 ? 1 : baselinePrice >= 10 ? 2 : baselinePrice >= 1 ? 3 : baselinePrice >= 0.01 ? 4 : 6;
                                                        const fixed = price.toFixed(precision);
                                                        const [whole, decimal] = fixed.split('.');
                                                        const withCommas = whole.replace(/\\B(?=(\\d{3})+(?!\\d))/g, ',');
                                                        return decimal ? withCommas + '.' + decimal : withCommas;
                                                    }
                                                },
                                            });
                                            logToFile('SUCCESS: TradingView v5.0 Line series created as fallback!');
                                            seriesCreated = true;
                                        } else {
                                            logToFile('LineSeries constant also not found');
                                        }
                                    } catch (lineError) {
                                        logToFile('Line series fallback also failed: ' + lineError.message);
                                    }
                                }
                            }
                            // Fallback to area series if baseline not available
                            else if (chart.addAreaSeries && typeof chart.addAreaSeries === 'function') {
                                try {
                                    areaSeries = chart.addAreaSeries({
                                        lineColor: '#00FF00',
                                        topColor: 'rgba(0, 255, 0, 0.1)', 
                                        bottomColor: 'rgba(0, 0, 0, 0)',
                                        lineWidth: 3,
                                        priceFormat: {
                                            type: 'price',
                                            precision: baselinePrice >= 1000 ? 0 : baselinePrice >= 100 ? 1 : baselinePrice >= 10 ? 2 : baselinePrice >= 1 ? 3 : baselinePrice >= 0.01 ? 4 : 6,
                                            minMove: baselinePrice >= 1000 ? 1 : baselinePrice >= 100 ? 0.1 : baselinePrice >= 10 ? 0.01 : baselinePrice >= 1 ? 0.001 : baselinePrice >= 0.01 ? 0.0001 : 0.000001,
                                            formatter: function(price) {
                                                const precision = baselinePrice >= 1000 ? 0 : baselinePrice >= 100 ? 1 : baselinePrice >= 10 ? 2 : baselinePrice >= 1 ? 3 : baselinePrice >= 0.01 ? 4 : 6;
                                                const fixed = price.toFixed(precision);
                                                const [whole, decimal] = fixed.split('.');
                                                const withCommas = whole.replace(/\\B(?=(\\d{3})+(?!\\d))/g, ',');
                                                return decimal ? withCommas + '.' + decimal : withCommas;
                                            }
                                        },
                                    });
                                    logToFile('SUCCESS: Area series created!');
                                    seriesCreated = true;
                                } catch (areaError) {
                                    logToFile('Area series failed: ' + areaError.message);
                                }
                            }
                            // Pattern 2: Try correct TradingView API with proper series types
                            else if (chart.addSeries && typeof chart.addSeries === 'function') {
                                logToFile('Using TradingView addSeries API');
                                try {
                                    // Try different series type strings that TradingView might expect
                                    const seriesTypes = ['Area', 'Line', 'area', 'line'];
                                    for (let seriesType of seriesTypes) {
                                        try {
                                            logToFile('Trying series type: ' + seriesType);
                                            areaSeries = chart.addSeries(seriesType, {});
                                            logToFile('SUCCESS with series type: ' + seriesType);
                                            seriesCreated = true;
                                            break;
                                        } catch (typeError) {
                                            logToFile('Type ' + seriesType + ' failed: ' + typeError.message);
                                        }
                                    }
                                } catch (addSeriesError) {
                                    logToFile('addSeries method failed: ' + addSeriesError.message);
                                }
                            }
                            // Pattern 3: Try createSeries
                            else if (chart.createSeries && typeof chart.createSeries === 'function') {
                                logToFile('Using createSeries API');
                                areaSeries = chart.createSeries('line');
                                seriesCreated = true;
                            }
                            
                            if (seriesCreated) {
                                logToFile('SUCCESS: Series created with working API!');
                            } else {
                                logToFile('ERROR: No working series creation method found');
                                areaSeries = null;
                            }
                            
                        } catch (chartError) {
                            logToFile('ERROR in chart creation block: ' + chartError.message);
                            logToFile('ERROR stack: ' + (chartError.stack || 'no stack'));
                            if (chartError.message.includes('Assertion failed')) {
                                logToFile('ASSERTION ERROR: Series creation failed, trying line series fallback');
                                try {
                                    areaSeries = chart.addLineSeries({
                                        color: '\(chartColor)',
                                        lineWidth: 2,
                                    });
                                    logToFile('SUCCESS: Line series created as fallback');
                                } catch (lineError) {
                                    logToFile('Line series also failed: ' + lineError.message);
                                    areaSeries = null;
                                }
                            } else {
                                return;
                            }
                        }
                        
                        // Use real chart data with reference lines
                        if (areaSeries) {
                            try {
                                const priceData = \(priceData);
                                const volumeData = \(volumeData);
                                let priceDataToUse = priceData;
                                let volumeDataToUse = volumeData;
                                
                                if (!priceData || priceData.length === 0) {
                                    // Fallback to test data if no real data
                                    priceDataToUse = [
                                        { time: 1725148800, value: 58000 },
                                        { time: 1725152400, value: 58200 },
                                        { time: 1725156000, value: 58100 },
                                        { time: 1725159600, value: 58400 },
                                        { time: 1725163200, value: 58300 }
                                    ];
                                    volumeDataToUse = [
                                        { time: 1725148800, value: 1200000000, color: 'rgba(0, 150, 136, 0.8)' },
                                        { time: 1725152400, value: 1300000000, color: 'rgba(0, 150, 136, 0.8)' },
                                        { time: 1725156000, value: 900000000, color: 'rgba(255, 82, 82, 0.8)' },
                                        { time: 1725159600, value: 1500000000, color: 'rgba(0, 150, 136, 0.8)' },
                                        { time: 1725163200, value: 1100000000, color: 'rgba(255, 82, 82, 0.8)' }
                                    ];
                                }
                                
                                // Set data and verify baseline is working
                                if (priceDataToUse.length > 0) {
                                    const startPrice = priceDataToUse[0].value;
                                    const endPrice = priceDataToUse[priceDataToUse.length - 1].value;
                                    logToFile('Data range - Start: $' + startPrice.toFixed(2) + ', End: $' + endPrice.toFixed(2));
                                    
                                    // Find min/max for debugging
                                    const prices = priceDataToUse.map(p => p.value);
                                    const minPrice = Math.min(...prices);
                                    const maxPrice = Math.max(...prices);
                                    logToFile('Price range - Min: $' + minPrice.toFixed(2) + ', Max: $' + maxPrice.toFixed(2));
                                    
                                    // Check if baseline should show both colors
                                    const pointsAbove = prices.filter(p => p > startPrice).length;
                                    const pointsBelow = prices.filter(p => p < startPrice).length;
                                    logToFile('Points above start: ' + pointsAbove + ', below start: ' + pointsBelow);
                                }
                                
                                areaSeries.setData(priceDataToUse);
                                
                                // Create volume histogram series
                                logToFile('Creating volume histogram series...');
                                let volumeSeries;
                                
                                try {
                                    // Add volume colors based on price movement
                                    const volumeWithColors = volumeDataToUse.map((vol, index) => {
                                        if (index === 0 || !priceDataToUse[index] || !priceDataToUse[index - 1]) {
                                            return { ...vol, color: 'rgba(0, 150, 136, 0.3)' };
                                        }
                                        const priceUp = priceDataToUse[index].value > priceDataToUse[index - 1].value;
                                        return {
                                            ...vol,
                                            color: priceUp ? 'rgba(0, 200, 81, 0.3)' : 'rgba(255, 68, 68, 0.3)'
                                        };
                                    });
                                    
                                    if (chart.addSeries && typeof chart.addSeries === 'function' && LightweightCharts.HistogramSeries) {
                                        volumeSeries = chart.addSeries(LightweightCharts.HistogramSeries, {
                                            priceFormat: {
                                                type: 'volume',
                                            },
                                            priceScaleId: 'volume',
                                            color: 'rgba(0, 150, 136, 0.3)',
                                        });
                                        logToFile('Volume series created using v5.0 API');
                                    } else if (chart.addHistogramSeries && typeof chart.addHistogramSeries === 'function') {
                                        volumeSeries = chart.addHistogramSeries({
                                            priceFormat: {
                                                type: 'volume',
                                            },
                                            priceScaleId: 'volume',
                                            color: 'rgba(0, 150, 136, 0.3)',
                                        });
                                        logToFile('Volume series created using addHistogramSeries');
                                    }
                                    
                                    if (volumeSeries) {
                                        volumeSeries.setData(volumeWithColors);
                                        logToFile('Volume data set successfully with ' + volumeWithColors.length + ' points');
                                        
                                        // Configure volume scale on the right
                                        chart.priceScale('volume').applyOptions({
                                            scaleMargins: {
                                                top: 0.8, // Volume uses bottom 20% of chart
                                                bottom: 0,
                                            },
                                        });
                                    } else {
                                        logToFile('WARNING: Could not create volume series');
                                    }
                                } catch (volError) {
                                    logToFile('ERROR creating volume series: ' + volError.message);
                                }
                                
                                // Add reference lines for start and end prices (without labels)
                                if (priceDataToUse.length > 1) {
                                    const startPrice = priceDataToUse[0].value;
                                    const endPrice = priceDataToUse[priceDataToUse.length - 1].value;
                                    
                                    // Add start price line (no title/label)
                                    const startPriceLine = areaSeries.createPriceLine({
                                        price: startPrice,
                                        color: 'rgba(255, 255, 255, 0.5)',
                                        lineWidth: 1,
                                        lineStyle: 2, // Dashed line
                                        axisLabelVisible: true,
                                        title: '', // Remove label
                                    });
                                    
                                    logToFile('Added start price reference line at: $' + startPrice.toFixed(2));
                                    
                                    // Add end price line (no title/label)
                                    const endPriceLine = areaSeries.createPriceLine({
                                        price: endPrice,
                                        color: 'rgba(255, 255, 255, 0.3)',
                                        lineWidth: 1,
                                        lineStyle: 2, // Dashed line
                                        axisLabelVisible: false, // Hide duplicate label
                                        title: '', // Remove label
                                    });
                                }
                                
                                // Configure evenly spaced time intervals based on timeframe
                                const timeframe = '\(timeframe)';
                                if (priceDataToUse.length > 0) {
                                    const firstTime = priceDataToUse[0].time;
                                    const lastTime = priceDataToUse[priceDataToUse.length - 1].time;
                                    
                                    // Set visible range to show all data spanning full width
                                    chart.timeScale().setVisibleRange({
                                        from: firstTime,
                                        to: lastTime
                                    });
                                    
                                    // Configure better tick mark spacing for all timeframes
                                    chart.applyOptions({
                                        timeScale: {
                                            minBarSpacing: 1,
                                            barSpacing: timeframe === '1h' ? 3 : 6,
                                            rightOffset: 12,
                                            ticksVisible: true,
                                            uniformDistribution: true,
                                        }
                                    });
                                    logToFile('Applied improved tick mark spacing for ' + timeframe);
                                    
                                    logToFile('Set visible time range from ' + new Date(firstTime * 1000) + ' to ' + new Date(lastTime * 1000));
                                } else {
                                    chart.timeScale().fitContent();
                                }
                                
                                // Price formatting now handled via localization.priceFormatter
                                
                                logToFile('Chart data and reference lines set successfully');
                            } catch (dataError) {
                                logToFile('ERROR setting chart data: ' + dataError.message);
                            }
                        }
                        
                        // Handle resize
                        const resizeObserver = new ResizeObserver(entries => {
                            for (let entry of entries) {
                                const { width, height } = entry.contentRect;
                                chart.applyOptions({ width: width, height: height });
                            }
                        });
                        resizeObserver.observe(container);
                        
                        logToFile('TradingView chart initialized successfully');
                    }
                    
                    // Start waiting for library to load
                    setTimeout(waitForLibraryAndInit, 100);
                    
                } catch (error) {
                    logToFile('ERROR in main try block: ' + error.message);
                }
            </script>
        </body>
        </html>
        """
    }
    
    private func formatDataForTradingView() -> (String, String) {
        print("📊 TradingView: Formatting \(data.count) data points")
        if !data.isEmpty {
            print("📊 TradingView: First point - timestamp: \(data[0].timestamp), price: \(data[0].price), volume: \(data[0].volume ?? 0)")
            print("📊 TradingView: Last point - timestamp: \(data.last!.timestamp), price: \(data.last!.price), volume: \(data.last!.volume ?? 0)")
        } else {
            print("📊 TradingView: WARNING - No data points available!")
            // Return empty arrays if no data
            return ("[]", "[]")
        }
        
        // Sort data by timestamp to ensure proper ordering
        let sortedData = data.sorted { $0.timestamp < $1.timestamp }
        
        // Use Unix timestamp format for TradingView (seconds since epoch)
        let priceData = sortedData.map { point in
            return "{ time: \(Int(point.timestamp)), value: \(point.price) }"
        }.joined(separator: ", ")
        
        let volumeData = sortedData.map { point in
            let volumeValue = point.volume ?? 0.0
            return "{ time: \(Int(point.timestamp)), value: \(volumeValue) }"
        }.joined(separator: ", ")
        
        let priceResult = "[\(priceData)]"
        let volumeResult = "[\(volumeData)]"
        print("📊 TradingView: Generated price data string length: \(priceResult.count)")
        print("📊 TradingView: Generated volume data string length: \(volumeResult.count)")
        if !priceData.isEmpty {
            print("📊 TradingView: Sample price point: \(priceData.prefix(100))")
            print("📊 TradingView: Sample volume point: \(volumeData.prefix(100))")
        }
        return (priceResult, volumeResult)
    }
}

// MARK: - Preview
struct TradingViewChartView_Previews: PreviewProvider {
    static var previews: some View {
        let sampleData = [
            ChartDataPoint(timestamp: 1640995200, price: 47000.0, volume: 25000000000),
            ChartDataPoint(timestamp: 1641081600, price: 47500.0, volume: 27000000000),
            ChartDataPoint(timestamp: 1641168000, price: 46800.0, volume: 22000000000),
            ChartDataPoint(timestamp: 1641254400, price: 48200.0, volume: 30000000000),
            ChartDataPoint(timestamp: 1641340800, price: 49100.0, volume: 28000000000),
        ]
        
        TradingViewChartView(data: sampleData, isPositive: true, timeframe: "24h")
            .frame(height: 300)
            .background(Color.black)
            .previewLayout(.sizeThatFits)
    }
}