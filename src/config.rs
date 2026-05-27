use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub username: String,
    pub listen_port: u16,
    pub peers: Vec<PeerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    pub name: String,
    pub address: String,
    pub expected_fingerprint: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: "suraj".to_string(),
            listen_port: 7799,
            peers: vec![
                PeerConfig {
                    name: "friend".to_string(),
                    address: "127.0.0.1:7799".to_string(),
                    expected_fingerprint: None,
                },
            ],
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("cld");

    fs::create_dir_all(&config_dir)?;

    Ok(config_dir.join("config.toml"))
}

pub fn load_or_create_config() -> Result<Config> {
    let path = config_path()?;

    if !path.exists() {
        let default_config = Config::default();
        let toml = toml::to_string_pretty(&default_config)?;
        fs::write(&path, toml)?;

        println!("Created default config at: {}", path.display());

        return Ok(default_config);
    }

    let content = fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&content)?;

    Ok(config)
}