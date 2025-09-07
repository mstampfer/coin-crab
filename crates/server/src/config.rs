use log::{info, warn};
use std::path::Path;

pub struct ServerConfig {
    pub api_key: String,
    pub log_level: String,
    pub mqtt_broker_host: String,
    pub mqtt_broker_port: u16,
    pub update_interval_seconds: u64,
}

impl ServerConfig {
    pub fn load() -> Result<Self, String> {
        // Load .env file first with debug information
        let current_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        
        println!("Server starting from directory: {}", current_dir);
        
        let env_file_path = Path::new("crates/server/.env.server");
        println!("Looking for .env.server at: {}", env_file_path.display());
        println!(".env.server file exists: {}", env_file_path.exists());
        
        match dotenv::from_filename("crates/server/.env.server") {
            Ok(path) => println!("Successfully loaded .env.server from: {}", path.display()),
            Err(e) => println!("Failed to load .env.server: {}", e),
        }

        let api_key = std::env::var("CMC_API_KEY")
            .unwrap_or_else(|_| {
                warn!("CMC_API_KEY environment variable not set, using placeholder");
                "YOUR_API_KEY_HERE".to_string()
            });

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
        
        let mqtt_broker_host = std::env::var("MQTT_BROKER_HOST").unwrap_or_else(|_| {
            warn!("MQTT_BROKER_HOST not set in .env file, using localhost (0.0.0.0)");
            "0.0.0.0".to_string()
        });
        
        let mqtt_broker_port = std::env::var("MQTT_BROKER_PORT")
            .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
            .unwrap_or_else(|_| {
                warn!("MQTT_BROKER_PORT not set in .env file, using default (1883)");
                1883
            });

        let update_interval_seconds = std::env::var("UPDATE_INTERVAL_SECONDS")
            .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
            .unwrap_or_else(|_| {
                warn!("UPDATE_INTERVAL_SECONDS not set in .env file, using default (900 seconds / 15 minutes)");
                900
            });

        Ok(ServerConfig {
            api_key,
            log_level,
            mqtt_broker_host,
            mqtt_broker_port,
            update_interval_seconds,
        })
    }

    pub fn setup_logging(&self) {
        let mut builder = env_logger::Builder::from_default_env();
        
        // Set base log level from environment
        let level_filter = match self.log_level.to_uppercase().as_str() {
            "OFF" => log::LevelFilter::Off,
            "ERROR" => log::LevelFilter::Error,
            "WARN" => log::LevelFilter::Warn,
            "INFO" => log::LevelFilter::Info,
            "DEBUG" => log::LevelFilter::Debug,
            "TRACE" => log::LevelFilter::Trace,
            _ => log::LevelFilter::Info,
        };
        
        builder.filter_level(level_filter);
        
        // Always suppress rumqttd logs regardless of main log level
        builder.filter_module("rumqttd", log::LevelFilter::Off);
        builder.filter_module("rumqttd::router", log::LevelFilter::Off);
        builder.filter_module("rumqttd::router::routing", log::LevelFilter::Off);
        
        // Suppress MQTT publisher error messages
        builder.filter_module("coin_crab_server::mqtt::publisher", log::LevelFilter::Warn);
        
        builder.init();
        
        info!("Logging initialized with level: {}", self.log_level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_creation() {
        let config = ServerConfig {
            api_key: "test_key".to_string(),
            log_level: "DEBUG".to_string(),
            mqtt_broker_host: "localhost".to_string(),
            mqtt_broker_port: 1883,
            update_interval_seconds: 300,
        };

        assert_eq!(config.api_key, "test_key");
        assert_eq!(config.log_level, "DEBUG");
        assert_eq!(config.mqtt_broker_host, "localhost");
        assert_eq!(config.mqtt_broker_port, 1883);
        assert_eq!(config.update_interval_seconds, 300);
    }

    #[test]
    fn test_log_level_mapping() {
        // Test that different log levels map to the correct filter level
        let test_cases = vec![
            ("OFF", log::LevelFilter::Off),
            ("ERROR", log::LevelFilter::Error),
            ("WARN", log::LevelFilter::Warn),
            ("INFO", log::LevelFilter::Info),
            ("DEBUG", log::LevelFilter::Debug),
            ("TRACE", log::LevelFilter::Trace),
            ("invalid", log::LevelFilter::Info), // Default case
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
            
            assert_eq!(level_filter, expected);
        }
    }
}