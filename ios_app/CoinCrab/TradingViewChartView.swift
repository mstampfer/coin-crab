import SwiftUI
import WebKit

struct TradingViewChartView: UIViewRepresentable {
    let data: [ChartDataPoint]
    let isPositive: Bool
    
    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        
        // Enable JavaScript
        configuration.preferences.javaScriptEnabled = true
        
        // Allow insecure content (mixed content)
        configuration.preferences.setValue(true, forKey: "allowFileAccessFromFileURLs")
        configuration.setValue(true, forKey: "allowUniversalAccessFromFileURLs")
        
        // Create WebView
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.scrollView.isScrollEnabled = false
        webView.scrollView.bounces = false
        webView.backgroundColor = UIColor.clear
        webView.isOpaque = false
        
        return webView
    }
    
    func updateUIView(_ webView: WKWebView, context: Context) {
        let htmlContent = generateHTMLContent()
        let baseURL = URL(string: "https://unpkg.com/")
        webView.loadHTMLString(htmlContent, baseURL: baseURL)
    }
    
    private func generateHTMLContent() -> String {
        let chartData = formatDataForTradingView()
        let chartColor = isPositive ? "#00C851" : "#FF4444"
        let fillColor = isPositive ? "rgba(0, 200, 81, 0.1)" : "rgba(255, 68, 68, 0.1)"
        
        print("📊 TradingView HTML: Generated chartData for JS: \(chartData)")
        
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
                }
            </style>
        </head>
        <body>
            <div id="chartContainer"></div>
            
            <script src="https://unpkg.com/lightweight-charts@4.1.0/dist/lightweight-charts.standalone.production.js" onload="logToFile('TradingView v4.1.0 library loaded')" onerror="logToFile('Failed to load v4.1.0, trying latest...'); loadFromUnpkg();"></script>
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
                    
                    // Only show critical errors, not debug logs
                    if (message.includes('ERROR') || message.includes('FAIL')) {
                        let logDiv = document.getElementById('debug-log');
                        if (!logDiv) {
                            logDiv = document.createElement('div');
                            logDiv.id = 'debug-log';
                            logDiv.style.cssText = 'position: fixed; top: 0; left: 0; width: 100%; background: rgba(255,0,0,0.9); color: white; font-family: monospace; font-size: 12px; z-index: 9999; padding: 5px;';
                            document.body.appendChild(logDiv);
                        }
                        
                        const timestamp = new Date().toISOString().substr(11, 8);
                        logDiv.innerHTML = timestamp + ': ' + message;
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
                            
                            // Try absolute minimal chart configuration
                            logToFile('Trying minimal chart config...');
                            chart = LightweightCharts.createChart(container, {
                                width: chartWidth,
                                height: chartHeight,
                            });
                            logToFile('TradingView: Chart created successfully');
                            
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
                            logToFile('TradingView: All function methods: ' + allMethods.join(', '));
                            
                            // Try different API patterns that might exist
                            let seriesCreated = false;
                            
                            // Pattern 1: Try v4.x API first
                            if (chart.addLineSeries && typeof chart.addLineSeries === 'function') {
                                logToFile('Using v4.x addLineSeries API');
                                try {
                                    areaSeries = chart.addLineSeries({
                                        color: '\(chartColor)',
                                        lineWidth: 2,
                                    });
                                    logToFile('SUCCESS: v4.x line series created!');
                                    seriesCreated = true;
                                } catch (v4Error) {
                                    logToFile('v4.x line series failed: ' + v4Error.message);
                                }
                            }
                            // Try area series if line series worked
                            else if (chart.addAreaSeries && typeof chart.addAreaSeries === 'function') {
                                logToFile('Using v4.x addAreaSeries API');
                                try {
                                    areaSeries = chart.addAreaSeries({
                                        lineColor: '\(chartColor)',
                                        topColor: '\(fillColor)',
                                        bottomColor: 'rgba(0, 0, 0, 0)',
                                        lineWidth: 2,
                                    });
                                    logToFile('SUCCESS: v4.x area series created!');
                                    seriesCreated = true;
                                } catch (v4AreaError) {
                                    logToFile('v4.x area series failed: ' + v4AreaError.message);
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
                        
                        // Use real chart data from Swift
                        if (areaSeries) {
                            try {
                                const chartData = \(chartData);
                                if (chartData && chartData.length > 0) {
                                    logToFile('Setting real chart data (' + chartData.length + ' points)');
                                    areaSeries.setData(chartData);
                                    chart.timeScale().fitContent();
                                } else {
                                    // Fallback to test data if no real data
                                    const testData = [
                                        { time: 1725148800, value: 58000 },
                                        { time: 1725152400, value: 58200 },
                                        { time: 1725156000, value: 58100 },
                                        { time: 1725159600, value: 58400 },
                                        { time: 1725163200, value: 58300 }
                                    ];
                                    areaSeries.setData(testData);
                                    chart.timeScale().fitContent();
                                }
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
    
    private func formatDataForTradingView() -> String {
        print("📊 TradingView: Formatting \(data.count) data points")
        if !data.isEmpty {
            print("📊 TradingView: First point - timestamp: \(data[0].timestamp), price: \(data[0].price)")
            print("📊 TradingView: Last point - timestamp: \(data.last!.timestamp), price: \(data.last!.price)")
        } else {
            print("📊 TradingView: WARNING - No data points available!")
            // Return empty array if no data
            return "[]"
        }
        
        // Sort data by timestamp to ensure proper ordering
        let sortedData = data.sorted { $0.timestamp < $1.timestamp }
        
        // Use Unix timestamp format for TradingView (seconds since epoch)
        let tradingViewData = sortedData.map { point in
            return "{ time: \(Int(point.timestamp)), value: \(point.price) }"
        }.joined(separator: ", ")
        
        let result = "[\(tradingViewData)]"
        print("📊 TradingView: Generated data string length: \(result.count)")
        print("📊 TradingView: Full data string: \(result)")
        if !tradingViewData.isEmpty {
            print("📊 TradingView: Sample data point: \(tradingViewData.prefix(100))")
        }
        return result
    }
}

// MARK: - Preview
struct TradingViewChartView_Previews: PreviewProvider {
    static var previews: some View {
        let sampleData = [
            ChartDataPoint(timestamp: 1640995200, price: 47000.0),
            ChartDataPoint(timestamp: 1641081600, price: 47500.0),
            ChartDataPoint(timestamp: 1641168000, price: 46800.0),
            ChartDataPoint(timestamp: 1641254400, price: 48200.0),
            ChartDataPoint(timestamp: 1641340800, price: 49100.0),
        ]
        
        TradingViewChartView(data: sampleData, isPositive: true)
            .frame(height: 300)
            .background(Color.black)
            .previewLayout(.sizeThatFits)
    }
}