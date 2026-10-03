use crate::addon_status::AddonStatus;
use crate::config::Config;
use crate::github::GitHubClient;
use std::error::Error;
use std::fs;
use std::path::Path;
use tar::Archive;
use flate2::read::GzDecoder;

#[derive(Debug, Clone)]
pub struct Addon {
    pub name: String,
    pub url: String,
    pub status: AddonStatus,
}

impl Addon {
    pub fn new(name: String, url: String, status: AddonStatus) -> Self {
        Self {
            name,
            url,
            status,
        }
    }

    /// Helper: Check if addon is installed
    pub fn is_installed(&self) -> bool {
        self.status.is_installed()
    }

    /// Helper: Get version if available
    pub fn version(&self) -> String {
        self.status.version().unwrap_or_else(|| "Unknown".to_string())
    }
}

pub struct AddonManager {
    config: Config,
    client: GitHubClient,
}

impl AddonManager {
    pub fn new(config: Config) -> Self {
        let client = GitHubClient::new(config.github_token.clone());
        Self { config, client }
    }

    pub fn new_with_token(config: Config, token: String) -> Self {
        let client = GitHubClient::new(Some(token));
        Self { config, client }
    }

    pub async fn list_addons(&self) -> Result<Vec<Addon>, Box<dyn Error>> {
        let mut addons = Vec::new();

        for repo in &self.config.repositories {
            let installed = self.is_installed(&repo.name);

            let status = if installed {
                let version = self.get_version_info(&repo.name)
                    .unwrap_or_else(|_| "installed".to_string());
                AddonStatus::Installed { version }
            } else {
                AddonStatus::NotInstalled
            };

            addons.push(Addon::new(
                repo.name.clone(),
                repo.url.clone(),
                status,
            ));
        }

        Ok(addons)
    }

    pub async fn get_addon_info(&self, name: &str) -> Result<Addon, Box<dyn Error>> {
        // Find the repository
        let repo = self
            .config
            .repositories
            .iter()
            .find(|repo| repo.name == name)
            .ok_or_else(|| format!("Addon {} not found in configuration", name))?;

        let installed = self.is_installed(name);

        let status = if installed {
            let version = self.get_version_info(name)
                .unwrap_or_else(|_| "installed".to_string());
            AddonStatus::Installed { version }
        } else {
            AddonStatus::NotInstalled
        };

        Ok(Addon::new(
            repo.name.clone(),
            repo.url.clone(),
            status,
        ))
    }

    fn is_installed(&self, name: &str) -> bool {
        let addon_path = format!("{}/{}", self.config.addon_dir, name);
        Path::new(&addon_path).exists()
    }

    fn get_version_info(&self, name: &str) -> Result<String, Box<dyn Error>> {
        let version_file = format!("{}/{}/.version", self.config.addon_dir, name);
        let version = fs::read_to_string(version_file)?;
        Ok(version.trim().to_string())
    }

    fn save_version_info(&self, name: &str, version: &str) -> Result<(), Box<dyn Error>> {
        let version_file = format!("{}/{}/.version", self.config.addon_dir, name);
        fs::write(version_file, version)?;
        Ok(())
    }

    pub async fn install_addon(&self, name: &str) -> Result<(), Box<dyn Error>> {
        // Find the repository
        let repo_info = self
            .config
            .repositories
            .iter()
            .find(|repo| repo.name == name)
            .ok_or_else(|| format!("Addon {} not found in configuration", name))?;

        // Parse owner and repo from URL
        let (owner, repo_name) = parse_repo_url(&repo_info.url)?;

        // Get the latest commit hash
        let version = self.client.get_latest_commit(&owner, &repo_name).await?;

        // Check if addon is already installed
        if self.is_installed(name) {
            return Err(format!("Addon {} is already installed", name).into());
        }

        // Download the repository
        let data = self
            .client
            .download_repository(&owner, &repo_name, &version)
            .await?;

        // Extract and install the addon
        self.extract_and_install(&data, name, &version, repo_info.path.as_deref())?;

        Ok(())
    }

    pub async fn update_addon(&self, name: &str) -> Result<bool, Box<dyn Error>> {
        // Find the repository
        let repo_info = self
            .config
            .repositories
            .iter()
            .find(|repo| repo.name == name)
            .ok_or_else(|| format!("Addon {} not found in configuration", name))?;

        // Parse owner and repo from URL
        let (owner, repo_name) = parse_repo_url(&repo_info.url)?;

        // Get the latest commit hash
        let latest_version = self.client.get_latest_commit(&owner, &repo_name).await?;

        // Check if addon is installed
        if !self.is_installed(name) {
            return Err(format!("Addon {} is not installed", name).into());
        }

        // Get the current installed version
        let current_version = self.get_version_info(name).unwrap_or_default();

        // Check if update is needed
        if current_version == latest_version {
            return Ok(false); // No update needed
        }

        // Download the repository
        let data = self
            .client
            .download_repository(&owner, &repo_name, &latest_version)
            .await?;

        // Remove the old addon directory
        let addon_path = format!("{}/{}", self.config.addon_dir, name);
        if Path::new(&addon_path).exists() {
            fs::remove_dir_all(&addon_path)?;
        }

        // Extract and install the addon
        self.extract_and_install(&data, name, &latest_version, repo_info.path.as_deref())?;

        Ok(true)
    }

    pub async fn check_for_updates(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // Collect all check tasks for installed addons
        let mut check_tasks = Vec::new();

        for repo in &self.config.repositories {
            // Check if addon is installed
            if self.is_installed(&repo.name) {
                // Parse owner and repo from URL
                let (owner, repo_name) = match parse_repo_url(&repo.url) {
                    Ok(result) => result,
                    Err(_) => continue, // Skip if URL parsing fails
                };

                let client = self.client.clone();
                let addon_name = repo.name.clone();
                let current_version = self.get_version_info(&repo.name).unwrap_or_default();
                let owner = owner.to_string();
                let repo_name = repo_name.to_string();

                // Create async task for this addon
                let task = async move {
                    match client.get_latest_commit(&owner, &repo_name).await {
                        Ok(latest_version) => {
                            if current_version != latest_version {
                                Some(addon_name)
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    }
                };

                check_tasks.push(task);
            }
        }

        // Execute all checks in parallel
        let results = futures::future::join_all(check_tasks).await;

        // Collect updates
        let updates: Vec<String> = results.into_iter().filter_map(|r| r).collect();

        Ok(updates)
    }

    pub fn remove_addon(&self, name: &str) -> Result<(), Box<dyn Error>> {
        // Check if addon is installed
        if !self.is_installed(name) {
            return Err(format!("Addon {} is not installed", name).into());
        }

        // Remove the addon directory (including .version file inside)
        let addon_path = format!("{}/{}", self.config.addon_dir, name);
        fs::remove_dir_all(&addon_path)?;

        Ok(())
    }

    fn extract_and_install(
        &self,
        data: &[u8],
        name: &str,
        version: &str,
        addon_path: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        // Create the addon directory if it doesn't exist
        fs::create_dir_all(&self.config.addon_dir)?;
        
        // Create the specific addon directory
        let addon_dir = format!("{}/{}", self.config.addon_dir, name);
        fs::create_dir_all(&addon_dir)?;

        // Check if addon directory is writable
        let test_file = format!("{}/.test", self.config.addon_dir);
        fs::write(&test_file, "")?;
        fs::remove_file(&test_file)?;

        // First pass: get the repo prefix from the first entry
        let repo_prefix = {
            let decoder = GzDecoder::new(data);
            let mut archive = Archive::new(decoder);
            let entries = archive.entries()?;

            // Find the first real directory (skip pax_global_header and similar metadata)
            let mut found_prefix = None;
            for entry in entries {
                let entry = entry?;
                let path = entry.path()?;
                let path_str = path.to_string_lossy();

                // Skip pax headers and other metadata
                if path_str.starts_with("pax_global_header") || path_str.starts_with("._") {
                    continue;
                }

                // Get the first component (root directory)
                if let Some(first_component) = path.components().next() {
                    found_prefix = Some(format!("{}/", first_component.as_os_str().to_string_lossy()));
                    break;
                }
            }

            found_prefix.ok_or("Could not find repository prefix in archive")?
        };

        // Second pass: extract the files
        let decoder = GzDecoder::new(data);
        let mut archive = Archive::new(decoder);

        println!("DEBUG: Extracting addon '{}', repo_prefix: '{}', addon_path: {:?}", name, repo_prefix, addon_path);
        let mut extracted_count = 0;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            // Convert path to string for easier handling
            let path_str = path.to_string_lossy();

            // Check if this file is part of the addon we want to extract
            let should_extract = if let Some(addon_subpath) = addon_path {
                let full_source_path = if addon_subpath == "" {
                    format!("{}", repo_prefix)
                } else {
                    format!("{}{}/", repo_prefix, addon_subpath)
                };
                println!("DEBUG: Checking '{}' against '{}'", path_str, full_source_path);
                path_str.starts_with(&full_source_path)
            } else {
                // Default behavior - extract all files from the repo (skip only the root directory)
                let parts: Vec<&str> = path_str.split('/').collect();
                parts.len() > 1 // Skip only the root directory entry itself
            };

            // Skip files that are not part of the addon
            if !should_extract {
                continue;
            }

            extracted_count += 1;

            // Construct the final path
            let final_path = if let Some(addon_subpath) = addon_path {
                let full_source_path = if addon_subpath == "" {
                    format!("{}", repo_prefix)
                } else {
                    format!("{}{}/", repo_prefix, addon_subpath)
                };
                let relative_path = path_str.strip_prefix(&full_source_path).unwrap_or("");
                format!("{}/{}/{}", self.config.addon_dir, name, relative_path)
            } else {
                // Extract from root of archive
                let parts: Vec<&str> = path_str.split('/').collect();
                if parts.len() > 1 {
                    // Check if there's already a directory with addon name in archive
                    // If parts[1] == name, then archive structure is: repo-hash/addon-name/files
                    // Otherwise: repo-hash/files
                    let start_index = if parts.len() > 2 && parts[1] == name {
                        2 // Skip repo-hash AND addon-name
                    } else {
                        1 // Skip only repo-hash
                    };

                    let relative_path = parts[start_index..].join("/");
                    format!("{}/{}/{}", self.config.addon_dir, name, relative_path)
                } else {
                    format!("{}/{}", self.config.addon_dir, name)
                }
            };

            println!("DEBUG: Extracting '{}' -> '{}'", path_str, final_path);

            // Handle directory creation and file extraction
            if entry.header().entry_type().is_dir() {
                fs::create_dir_all(&final_path)?;
            } else {
                // Create parent directories if they don't exist
                if let Some(parent) = Path::new(&final_path).parent() {
                    fs::create_dir_all(parent)?;
                }

                // Extract file directly
                entry.unpack(&final_path)?;
            }
        }

        println!("DEBUG: Extracted {} files for addon '{}'", extracted_count, name);

        // Save version information
        self.save_version_info(name, version)?;

        Ok(())
    }
}

fn parse_repo_url(url: &str) -> Result<(String, String), Box<dyn Error>> {
    // Remove trailing .git if present
    let url = url.trim_end_matches(".git");

    // Handle HTTPS URLs
    if url.starts_with("https://github.com/") {
        let parts: Vec<&str> = url.trim_start_matches("https://github.com/").split('/').collect();
        if parts.len() < 2 {
            return Err("Invalid GitHub URL format".into());
        }
        return Ok((parts[0].to_string(), parts[1].to_string()));
    }

    // Handle SSH URLs
    if url.starts_with("git@github.com:") {
        let parts: Vec<&str> = url.trim_start_matches("git@github.com:").split('/').collect();
        if parts.len() < 2 {
            return Err("Invalid GitHub URL format".into());
        }
        let repo_name = parts[1].trim_end_matches(".git");
        return Ok((parts[0].to_string(), repo_name.to_string()));
    }

    Err("Unsupported URL format".into())
}
