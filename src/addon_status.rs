use serde::{Deserialize, Serialize};

/// Represents the current state of an addon
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AddonStatus {
    /// Addon is not installed
    NotInstalled,

    /// Addon is currently being installed
    Installing { progress: Option<f32> },

    /// Addon is installed and up to date
    Installed { version: String },

    /// Addon has an update available
    UpdateAvailable { current: String, latest: String },

    /// Addon is currently being updated
    Updating {
        current_version: String,
        progress: Option<f32>,
    },

    /// Addon is being deleted
    Deleting,

    /// An error occurred with this addon
    Error { message: String },
}

impl AddonStatus {
    /// Check if the addon is installed (in any installed state)
    pub fn is_installed(&self) -> bool {
        matches!(
            self,
            AddonStatus::Installed { .. }
                | AddonStatus::UpdateAvailable { .. }
                | AddonStatus::Updating { .. }
                | AddonStatus::Deleting
        )
    }

    /// Check if an operation is in progress
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            AddonStatus::Installing { .. } | AddonStatus::Updating { .. } | AddonStatus::Deleting
        )
    }

    /// Check if update is available
    pub fn has_update(&self) -> bool {
        matches!(self, AddonStatus::UpdateAvailable { .. })
    }

    /// Get version string if available
    pub fn version(&self) -> Option<String> {
        match self {
            AddonStatus::Installed { version } => Some(version.clone()),
            AddonStatus::UpdateAvailable { current, .. } => Some(current.clone()),
            AddonStatus::Updating {
                current_version, ..
            } => Some(current_version.clone()),
            _ => None,
        }
    }

    /// Get display text for the status
    pub fn display_text(&self) -> String {
        match self {
            AddonStatus::NotInstalled => "Not Installed".to_string(),
            AddonStatus::Installing { progress } => {
                if let Some(p) = progress {
                    format!("Installing... {:.0}%", p * 100.0)
                } else {
                    "Installing...".to_string()
                }
            }
            AddonStatus::Installed { .. } => "Installed".to_string(),
            AddonStatus::UpdateAvailable { .. } => "Update Available".to_string(),
            AddonStatus::Updating { progress, .. } => {
                if let Some(p) = progress {
                    format!("Updating... {:.0}%", p * 100.0)
                } else {
                    "Updating...".to_string()
                }
            }
            AddonStatus::Deleting => "Deleting...".to_string(),
            AddonStatus::Error { message } => format!("Error: {}", message),
        }
    }

    /// Get color for the status display
    pub fn display_color(&self) -> egui::Color32 {
        match self {
            AddonStatus::NotInstalled => egui::Color32::GRAY,
            AddonStatus::Installing { .. } => egui::Color32::BLUE,
            AddonStatus::Installed { .. } => egui::Color32::GREEN,
            AddonStatus::UpdateAvailable { .. } => egui::Color32::from_rgb(128, 128, 0),
            AddonStatus::Updating { .. } => egui::Color32::BLUE,
            AddonStatus::Deleting => egui::Color32::RED,
            AddonStatus::Error { .. } => egui::Color32::RED,
        }
    }
}

impl Default for AddonStatus {
    fn default() -> Self {
        AddonStatus::NotInstalled
    }
}
