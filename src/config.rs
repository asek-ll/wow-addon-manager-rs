use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub addon_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_token: Option<String>,
    pub repositories: Vec<RepoInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let mut config: Config = serde_yaml::from_str(&contents)?;

        // If github_token is not set in config, try to read from environment variable
        if config.github_token.is_none() || config.github_token.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                if !token.is_empty() {
                    config.github_token = Some(token);
                }
            }
        }

        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let yaml = serde_yaml::to_string(self)?;
        fs::write(path, yaml)?;
        Ok(())
    }

    pub fn default_config_path() -> String {
        // Try to find config.yaml in current directory first
        let local_config = "config.yaml";
        if Path::new(local_config).exists() {
            return local_config.to_string();
        }

        // If not found, create in user's config directory
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("wow-addon-manager").join("config.yaml");
            if let Some(parent) = config_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            return config_path.to_string_lossy().to_string();
        }

        // Fallback to current directory
        local_config.to_string()
    }

    pub fn default_addon_dir() -> String {
        // Try to find a common WoW installation path
        if cfg!(target_os = "windows") {
            // Windows paths
            let paths = [
                r"C:\Program Files (x86)\World of Warcraft\Interface\AddOns",
                r"C:\Program Files\World of Warcraft\Interface\AddOns",
                r"D:\World of Warcraft\Interface\AddOns",
            ];
            for path in &paths {
                if Path::new(path).exists() {
                    return path.to_string();
                }
            }
        } else if cfg!(target_os = "macos") {
            // macOS paths
            let paths = [
                "/Applications/World of Warcraft/Interface/AddOns",
                "/Users/Shared/World of Warcraft/Interface/AddOns",
            ];
            for path in &paths {
                if Path::new(path).exists() {
                    return path.to_string();
                }
            }
        } else {
            // Linux paths
            let paths = [
                "/home/user/.wine/drive_c/Program Files (x86)/World of Warcraft/Interface/AddOns",
                "/home/user/Games/world-of-warcraft/drive_c/Program Files (x86)/World of Warcraft/Interface/AddOns",
                "/mnt/drive/data/Games/wow/335/synas/wow/Interface/AddOns",
            ];
            for path in &paths {
                if Path::new(path).exists() {
                    return path.to_string();
                }
            }
        }

        // Default to current directory + AddOns
        "AddOns".to_string()
    }
}
