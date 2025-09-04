use dotenv;
use shared::debug_log;

/// Default MQTT broker host (AWS EC2)
const DEFAULT_BROKER_HOST: &str = "100.26.107.175";
const DEFAULT_BROKER_PORT: u16 = 1883;

pub struct Config {
    pub broker_host: String,
    pub broker_port: u16,
    pub enable_debug_logging: bool,
    pub log_level: String,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        // Set debug logging environment variables first
        std::env::set_var("ENABLE_DEBUG_LOGGING", "true");
        std::env::set_var("LOG_LEVEL", "DEBUG");
        
        // Load .env file from iOS bundle resources
        debug_log("Config: Attempting to load .env.client from iOS bundle...");
        
        let env_loaded = Self::load_env_file()?;
        
        if !env_loaded {
            debug_log("Config: No .env.client file found, using environment variables or defaults");
        }
        
        let broker_host = std::env::var("MQTT_BROKER_HOST").unwrap_or_else(|_| {
            debug_log("Config: MQTT_BROKER_HOST not set, using AWS EC2 default");
            DEFAULT_BROKER_HOST.to_string()
        });
        
        let broker_port = std::env::var("MQTT_BROKER_PORT")
            .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
            .unwrap_or(DEFAULT_BROKER_PORT);
        
        let enable_debug_logging = std::env::var("ENABLE_DEBUG_LOGGING")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);
        
        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "DEBUG".to_string());
        
        debug_log(&format!("Config: Loaded broker_host={}, port={}, debug={}, log_level={}", 
            broker_host, broker_port, enable_debug_logging, log_level));
        
        Ok(Config {
            broker_host,
            broker_port,
            enable_debug_logging,
            log_level,
        })
    }
    
    fn load_env_file() -> Result<bool, String> {
        // Try to find the .env.client file in the app bundle
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(bundle_dir) = exe_path.parent() {
                let env_file_path = bundle_dir.join(".env.client");
                debug_log(&format!("Config: Trying bundle path: {}", env_file_path.display()));
                
                if env_file_path.exists() {
                    debug_log("Config: .env.client file found in bundle");
                    if let Ok(_) = dotenv::from_path(&env_file_path) {
                        debug_log("Config: .env.client loaded from app bundle successfully");
                        return Ok(true);
                    } else {
                        debug_log("Config: Failed to load .env.client from bundle");
                        return Ok(false);
                    }
                } else {
                    debug_log("Config: .env.client file not found in bundle directory");
                }
            } else {
                debug_log("Config: Could not get bundle directory from executable path");
            }
        } else {
            debug_log("Config: Could not get current executable path");
            // Fallback: try current directory
            if let Ok(_) = dotenv::from_filename(".env.client") {
                debug_log("Config: .env.client loaded from current directory");
                return Ok(true);
            } else {
                debug_log("Config: .env.client not found in current directory either");
            }
        }
        
        Ok(false)
    }
    
    pub fn broker_address(&self) -> String {
        format!("{}:{}", self.broker_host, self.broker_port)
    }
}