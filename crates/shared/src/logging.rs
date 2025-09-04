use std::io::Write;

// Utility functions for logging across crates

pub fn debug_log(message: &str) {
    // ALWAYS log to console for debugging
    println!("[DEBUG] {}", message);
    eprintln!("[DEBUG-CONSOLE] {}", message);
    
    // Check environment variable for debug logging
    let enable_debug = std::env::var("ENABLE_DEBUG_LOGGING").unwrap_or_else(|_| "false".to_string());
    eprintln!("[ENV-CHECK] ENABLE_DEBUG_LOGGING = '{}'", enable_debug);
    
    // Always try to write to file for debugging
    let log_path = if cfg!(target_os = "ios") {
        // Try to get iOS Documents directory
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let path = format!("{}/Documents/debug.log", home);
        eprintln!("[FILE-PATH] Trying to write to: {}", path);
        path
    } else {
        "debug.log".to_string()
    };
    
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        writeln!(file, "[{}] {}", timestamp, message).ok();
        eprintln!("[FILE-SUCCESS] Wrote to debug log file");
    } else {
        eprintln!("[FILE-FAILED] Could not write to debug log at: {}", log_path);
    }
}

pub fn init_logging() {
    // Initialize logging based on environment variables
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
    
    let level_filter = match log_level.to_uppercase().as_str() {
        "OFF" => log::LevelFilter::Off,
        "ERROR" => log::LevelFilter::Error,
        "WARN" => log::LevelFilter::Warn,
        "INFO" => log::LevelFilter::Info,
        "DEBUG" => log::LevelFilter::Debug,
        "TRACE" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };
    
    env_logger::Builder::from_default_env()
        .filter_level(level_filter)
        .init();
        
    debug_log("Logging system initialized");
}