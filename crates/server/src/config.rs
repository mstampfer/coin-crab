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
        
        builder.init();
        
        info!("Logging initialized with level: {}", self.log_level);
    }
}