import SwiftUI
import WebKit

struct TradingViewChartView: UIViewRepresentable {
    let data: [ChartDataPoint]
    let isPositive: Bool
    
    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        
        // Enable JavaScript
        configuration.preferences.javaScriptEnabled = true
        
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
        webView.loadHTMLString(htmlContent, baseURL: nil)
    }
    
    private func generateHTMLContent() -> String {
        let chartData = formatDataForTradingView()
        let chartColor = isPositive ? "#00C851" : "#FF4444"
        let fillColor = isPositive ? "rgba(0, 200, 81, 0.1)" : "rgba(255, 68, 68, 0.1)"
        
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
            
            <script src="https://unpkg.com/lightweight-charts/dist/lightweight-charts.standalone.production.js"></script>
            <script>
                try {
                    // Wait for the DOM and script to be loaded
                    setTimeout(() => {
                        const container = document.getElementById('chartContainer');
                        if (!container || !window.LightweightCharts) {
                            console.error('Container or LightweightCharts not available');
                            return;
                        }
                        
                        const chart = LightweightCharts.createChart(container, {
                            width: container.clientWidth,
                            height: container.clientHeight,
                            layout: {
                                background: { type: 'solid', color: 'transparent' },
                                textColor: '#ffffff',
                            },
                            grid: {
                                vertLines: { 
                                    color: 'rgba(255, 255, 255, 0.1)',
                                    style: LightweightCharts.LineStyle.Dashed,
                                },
                                horzLines: { 
                                    color: 'rgba(255, 255, 255, 0.1)',
                                    style: LightweightCharts.LineStyle.Dashed,
                                },
                            },
                            crosshair: {
                                mode: LightweightCharts.CrosshairMode.Normal,
                            },
                            rightPriceScale: {
                                borderColor: 'rgba(255, 255, 255, 0.2)',
                            },
                            timeScale: {
                                borderColor: 'rgba(255, 255, 255, 0.2)',
                                timeVisible: true,
                                secondsVisible: false,
                            },
                        });
                        
                        const areaSeries = chart.addAreaSeries({
                            lineColor: '\(chartColor)',
                            topColor: '\(fillColor)',
                            bottomColor: 'rgba(0, 0, 0, 0)',
                            lineWidth: 2,
                            priceFormat: {
                                type: 'price',
                                precision: 2,
                                minMove: 0.01,
                            },
                        });
                        
                        const chartData = \(chartData);
                        console.log('Setting chart data:', chartData);
                        
                        if (chartData && chartData.length > 0) {
                            areaSeries.setData(chartData);
                            chart.timeScale().fitContent();
                        }
                        
                        // Handle resize
                        const resizeObserver = new ResizeObserver(entries => {
                            for (let entry of entries) {
                                const { width, height } = entry.contentRect;
                                chart.applyOptions({ width: width, height: height });
                            }
                        });
                        resizeObserver.observe(container);
                        
                        console.log('TradingView chart initialized successfully');
                        
                    }, 100);
                    
                } catch (error) {
                    console.error('Error creating chart:', error);
                }
            </script>
        </body>
        </html>
        """
    }
    
    private func formatDataForTradingView() -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        formatter.timeZone = TimeZone(identifier: "UTC")
        
        let tradingViewData = data.map { point in
            let date = Date(timeIntervalSince1970: point.timestamp)
            let timeString = formatter.string(from: date)
            return "{ time: '\(timeString)', value: \(point.price) }"
        }.joined(separator: ", ")
        
        return "[\(tradingViewData)]"
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