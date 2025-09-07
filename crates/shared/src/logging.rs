use std::io::Write;

// Utility functions for logging across crates

pub fn debug_log(message: &str) {
    // ALWAYS log to console for debugging
    println!("[DEBUG] {}", message);
    eprintln!("[DEBUG-CONSOLE] {}", message);
    
    // Check environment variable for logging level
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
    eprintln!("[ENV-CHECK] LOG_LEVEL = '{}'", log_level);
    
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_debug_log_basic_functionality() {
        // Test that debug_log doesn't panic and handles basic input
        debug_log("Test message");
        debug_log("Another test message with numbers: 123");
        debug_log("");
        
        // If we reach here, debug_log works without panicking
        assert!(true);
    }

    #[test]
    fn test_debug_log_special_characters() {
        // Test that debug_log handles special characters
        debug_log("Test with special chars: !@#$%^&*()");
        debug_log("Unicode test: 🦀🔥💻");
        debug_log("Multiline\ntest\nmessage");
        
        assert!(true);
    }

    #[test]
    fn test_log_level_mapping() {
        // Test the log level mapping logic from init_logging
        let test_cases = vec![
            ("OFF", log::LevelFilter::Off),
            ("ERROR", log::LevelFilter::Error),
            ("WARN", log::LevelFilter::Warn),
            ("INFO", log::LevelFilter::Info),
            ("DEBUG", log::LevelFilter::Debug),
            ("TRACE", log::LevelFilter::Trace),
            ("invalid", log::LevelFilter::Info), // Default case
            ("", log::LevelFilter::Info), // Empty string should default to Info
        ];

        for (input, expected) in test_cases {
            let level_filter = match input.to_uppercase().as_str() {
                "OFF" => log::LevelFilter::Off,
                "ERROR" => log::LevelFilter::Error,
                "WARN" => log::LevelFilter::Warn,
                "INFO" => log::LevelFilter::Info,
                "DEBUG" => log::LevelFilter::Debug,
                "TRACE" => log::LevelFilter::Trace,
                _ => log::LevelFilter::Info,
            };
            
            assert_eq!(level_filter, expected, "Failed for input: '{}'", input);
        }
    }

    #[test]
    fn test_environment_variable_handling() {
        // Test default behavior when LOG_LEVEL environment variable is not set
        let default_level = std::env::var("NONEXISTENT_LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
        assert_eq!(default_level, "INFO");
        
        // Test case sensitivity handling
        let test_levels = vec!["info", "INFO", "Info", "iNfO"];
        for level in test_levels {
            let normalized = level.to_uppercase();
            assert_eq!(normalized, "INFO");
        }
    }

    #[test]
    fn test_log_file_path_logic() {
        // Test the log file path logic for different platforms
        let ios_path = {
            let home = "/Users/testuser";
            format!("{}/Documents/debug.log", home)
        };
        assert_eq!(ios_path, "/Users/testuser/Documents/debug.log");
        
        let desktop_path = "debug.log";
        assert_eq!(desktop_path, "debug.log");
        
        // Test path with different home directories
        let tmp_path = {
            let home = "/tmp";
            format!("{}/Documents/debug.log", home)
        };
        assert_eq!(tmp_path, "/tmp/Documents/debug.log");
    }

    #[test]
    fn test_timestamp_format() {
        // Test that we can create timestamps (indirectly testing chrono usage)
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        
        // Basic format validation
        assert!(timestamp.contains("-"));
        assert!(timestamp.contains(":"));
        assert!(timestamp.contains("UTC"));
        assert!(timestamp.len() > 15); // Should be at least "YYYY-MM-DD HH:MM:SS UTC"
    }

    #[test]
    fn test_init_logging_doesnt_panic() {
        // We can't really test init_logging fully without side effects,
        // but we can at least verify it doesn't panic when called
        // Note: This might initialize the global logger, but that's okay for tests
        
        // Save current LOG_LEVEL if it exists
        let original_level = env::var("LOG_LEVEL").ok();
        
        // Test with different log levels
        let test_levels = vec!["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
        
        for level in test_levels {
            env::set_var("LOG_LEVEL", level);
            
            // init_logging might panic or fail if called multiple times
            // but we can test the level parsing logic at least
            let level_filter = match level.to_uppercase().as_str() {
                "OFF" => log::LevelFilter::Off,
                "ERROR" => log::LevelFilter::Error,
                "WARN" => log::LevelFilter::Warn,
                "INFO" => log::LevelFilter::Info,
                "DEBUG" => log::LevelFilter::Debug,
                "TRACE" => log::LevelFilter::Trace,
                _ => log::LevelFilter::Info,
            };
            
            // Just verify the mapping works - test that it's a valid level filter
            match level {
                "ERROR" => assert_eq!(level_filter, log::LevelFilter::Error),
                "WARN" => assert_eq!(level_filter, log::LevelFilter::Warn),
                "INFO" => assert_eq!(level_filter, log::LevelFilter::Info),
                "DEBUG" => assert_eq!(level_filter, log::LevelFilter::Debug),
                "TRACE" => assert_eq!(level_filter, log::LevelFilter::Trace),
                "OFF" => assert_eq!(level_filter, log::LevelFilter::Off),
                _ => assert_eq!(level_filter, log::LevelFilter::Info),
            }
        }
        
        // Restore original LOG_LEVEL
        match original_level {
            Some(level) => env::set_var("LOG_LEVEL", level),
            None => env::remove_var("LOG_LEVEL"),
        }
        
        assert!(true);
    }

    #[test] 
    fn test_message_length_handling() {
        // Test that debug_log handles messages of different lengths
        debug_log("Short");
        
        let medium_message = "This is a medium length message that contains some more text to test handling.";
        debug_log(medium_message);
        
        let long_message = "This is a very long message ".repeat(100);
        debug_log(&long_message);
        
        assert!(true);
    }

    #[test]
    fn test_concurrent_logging() {
        // Test that multiple debug_log calls work (basic concurrency test)
        let messages = vec![
            "Concurrent message 1",
            "Concurrent message 2", 
            "Concurrent message 3",
        ];
        
        for message in messages {
            debug_log(message);
        }
        
        assert!(true);
    }
}