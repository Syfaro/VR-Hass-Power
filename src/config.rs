//! Configuration creation, loading, and saving.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::hass::{check_credentials, get_entity_state};

/// Name of file to use for configuration.
static CONFIG_NAME: &str = "config.toml";

/// Application configuration.
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub power: PowerConfig,
    pub check: CheckConfig,
    pub homeassistant: HomeAssistantConfig,
}

/// Home Assistant configuration.
#[derive(Clone, Serialize, Deserialize)]
pub struct HomeAssistantConfig {
    /// URL of Home Assistant instance.
    pub url: String,
    /// API key for Home Assistant instance.
    pub api_key: String,
    /// The name of the service used to control the entity.
    pub service: String,
    /// The entity ID to control.
    pub entity: String,
}

/// Processing checking configuration.
#[derive(Clone, Serialize, Deserialize)]
pub struct CheckConfig {
    /// The name of the process to monitor.
    pub process_name: String,
    /// The interval to check for the process.
    pub interval: u64,
}

/// Power control configuration.
#[derive(Clone, Serialize, Deserialize)]
pub struct PowerConfig {
    /// Delay after process has exited before turning off entity.
    pub delay: u64,
}

/// Load configuration from a directory.
pub fn load_config(config_dir: &std::path::Path) -> Result<Config, Box<dyn std::error::Error>> {
    let path = config_dir.join(CONFIG_NAME);
    let mut f = std::fs::File::open(path)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;

    let config: Config = toml::from_str(&buf)?;

    Ok(config)
}

/// Save configuration to a directory.
pub fn save_config(
    config_dir: &std::path::Path,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = toml::to_string_pretty(&config)?;

    let path = config_dir.join(CONFIG_NAME);
    let mut f = std::fs::File::create(path)?;
    f.write_all(config.as_bytes())?;

    Ok(())
}

/// Prompt user for required configuration values, then save configuration to
/// the specified directory.
pub fn prompt_config(config_dir: &std::path::Path) -> Result<Config, Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    let config = loop {
        let mut url = String::new();
        eprint!("Home Assistant URL: ");
        stdout.flush().unwrap();
        stdin.read_line(&mut url)?;

        let mut api_key = String::new();
        eprint!("Home Assistant API Key: ");
        stdout.flush().unwrap();
        stdin.read_line(&mut api_key)?;

        let mut entity = String::new();
        eprint!("Home Assistant Control Entity: ");
        stdout.flush().unwrap();
        stdin.read_line(&mut entity)?;

        let config = Config {
            power: PowerConfig { delay: 60 },
            check: CheckConfig {
                process_name: "vrserver.exe".to_string(),
                interval: 3,
            },
            homeassistant: HomeAssistantConfig {
                url: url.trim().to_string(),
                api_key: api_key.trim().to_string(),
                service: "switch".to_string(),
                entity: entity.trim().to_string(),
            },
        };

        if !check_credentials(&config.homeassistant) {
            eprintln!("Home Assistant credentials were invalid, please try again");
            continue;
        }

        if get_entity_state(&config.homeassistant).is_err() {
            eprintln!("Home Assistant entity returned error, please try again");
            continue;
        }

        break config;
    };

    save_config(config_dir, &config)?;

    Ok(config)
}
