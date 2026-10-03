use chrono::Local;
use eframe::egui;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::addon::{Addon, AddonManager};
use crate::config::Config;
use crate::ui::error_banner;
use crate::ui::settings::SettingsWindow;

#[derive(Default)]
pub struct WowAddonManagerApp {
    config: Option<Config>,
    addons: Vec<Addon>,
    updates_available: Vec<String>,
    selected_addon: Option<usize>,
    loading: bool,
    checking_updates: bool,
    logs: Vec<String>,
    error: Option<String>,
    confirming_delete: Option<String>,
    runtime: Option<Arc<Runtime>>,
    addon_manager: Option<Arc<Mutex<AddonManager>>>,
    addon_receiver: Option<mpsc::Receiver<Vec<Addon>>>,
    update_receiver: Option<mpsc::Receiver<Vec<String>>>,
    error_receiver: Option<mpsc::Receiver<String>>,
    error_sender: Option<mpsc::Sender<String>>,
    // Operation results
    operation_receiver: Option<mpsc::Receiver<(String, bool)>>, // (addon_name, success)
    operation_sender: Option<mpsc::Sender<(String, bool)>>,
    // Single addon update
    addon_update_receiver: Option<mpsc::Receiver<Addon>>,
    addon_update_sender: Option<mpsc::Sender<Addon>>,
    // Settings window
    settings_open: bool,
    settings_addon_dir: String,
    settings_github_token: String,
    // Search and filters
    search_query: String,
    filter_installed_only: bool,
    filter_updates_only: bool,
}

impl WowAddonManagerApp {
    fn load_config(&mut self) {
        let config_path = Config::default_config_path();
        match Config::load(&config_path) {
            Ok(config) => {
                self.config = Some(config.clone());
                self.add_log(format!("Loaded config from {}", config_path));
                self.addon_receiver = None;
                self.update_receiver = None;

                // Create error channel
                let (error_tx, error_rx) = mpsc::channel();
                self.error_sender = Some(error_tx);
                self.error_receiver = Some(error_rx);

                // Create operation results channel
                let (op_tx, op_rx) = mpsc::channel();
                self.operation_sender = Some(op_tx);
                self.operation_receiver = Some(op_rx);

                // Create addon update channel
                let (addon_tx, addon_rx) = mpsc::channel();
                self.addon_update_sender = Some(addon_tx);
                self.addon_update_receiver = Some(addon_rx);

                // Initialize addon manager
                let runtime = Runtime::new().expect("Failed to create Tokio runtime");
                let addon_manager = if let Some(token) = &config.github_token {
                    AddonManager::new_with_token(config.clone(), token.clone())
                } else {
                    AddonManager::new(config.clone())
                };

                self.runtime = Some(Arc::new(runtime));
                self.addon_manager = Some(Arc::new(Mutex::new(addon_manager)));

                // Load addons
                self.load_addons();
            }
            Err(e) => {
                self.add_log(format!("Failed to load config: {}", e));
                self.error = Some(format!("Failed to load config: {}", e));
                self.addon_receiver = None;
                self.update_receiver = None;

                // Create default config
                self.create_default_config();
            }
        }
    }

    fn create_default_config(&mut self) {
        let config = Config {
            addon_dir: Config::default_addon_dir(),
            github_token: None,
            repositories: vec![
                // Add some default repositories
            ],
        };

        self.config = Some(config.clone());
        self.addon_receiver = None;
        self.update_receiver = None;

        // Initialize addon manager
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        let addon_manager = AddonManager::new(config.clone());

        self.runtime = Some(Arc::new(runtime));
        self.addon_manager = Some(Arc::new(Mutex::new(addon_manager)));

        self.add_log("Created default configuration".to_string());
    }

    fn load_addons(&mut self) {
        if let Some(runtime) = &self.runtime {
            if let Some(addon_manager) = &self.addon_manager {
                self.loading = true;
                self.error = None;

                let (sender, receiver) = mpsc::channel();
                self.addon_receiver = Some(receiver);

                let addon_manager_clone = addon_manager.clone();
                let runtime_clone = runtime.clone();
                let error_sender = self.error_sender.clone();

                // Run async operation in background
                std::thread::spawn(move || {
                    let result = runtime_clone.block_on(async {
                        let manager = addon_manager_clone.lock().unwrap();
                        manager.list_addons().await
                    });

                    // Send result back to UI thread through channel
                    match result {
                        Ok(addons) => {
                            let _ = sender.send(addons);
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to load addons: {}", e);
                            eprintln!("{}", error_msg);
                            if let Some(err_sender) = error_sender {
                                let _ = err_sender.send(error_msg);
                            }
                        }
                    }
                });
            }
        }
    }

    fn add_log(&mut self, message: String) {
        let timestamp = Local::now().format("%H:%M:%S").to_string();
        self.logs.push(format!("[{}] {}", timestamp, message));
        // Keep only the last 50 logs
        if self.logs.len() > 50 {
            self.logs.drain(0..self.logs.len() - 50);
        }
    }

    fn show_settings_window(&mut self, ctx: &egui::Context) -> bool {
        let (should_save, should_close) = SettingsWindow::show(
            ctx,
            &mut self.settings_open,
            &mut self.settings_addon_dir,
            &mut self.settings_github_token,
        );

        if should_close {
            self.settings_open = false;
        }

        should_save
    }

    fn save_settings(&mut self) {
        if let Some(config) = &mut self.config {
            // Update config
            config.addon_dir = self.settings_addon_dir.clone();
            config.github_token = if self.settings_github_token.is_empty() {
                None
            } else {
                Some(self.settings_github_token.clone())
            };

            // Save to file
            let config_path = Config::default_config_path();
            match config.save(&config_path) {
                Ok(_) => {
                    self.add_log(format!("Settings saved to {}", config_path));
                    // Reload config and addon manager
                    self.load_config();
                }
                Err(e) => {
                    self.add_log(format!("Failed to save settings: {}", e));
                    self.error = Some(format!("Failed to save settings: {}", e));
                }
            }
        }
    }

    fn handle_addon_action(&mut self, action: &str, addon_name: &str) {
        match action {
            "install" => {
                self.add_log(format!("Installing addon: {}", addon_name));

                // Set status to Installing
                if let Some(addon) = self.addons.iter_mut().find(|a| a.name == addon_name) {
                    addon.status = crate::addon_status::AddonStatus::Installing { progress: None };
                }

                // Install the addon
                if let Some(runtime) = &self.runtime {
                    if let Some(addon_manager) = &self.addon_manager {
                        let addon_manager_clone = addon_manager.clone();
                        let runtime_clone = runtime.clone();
                        let addon_name_string = addon_name.to_string();
                        let error_sender = self.error_sender.clone();
                        let operation_sender = self.operation_sender.clone();

                        // Run installation in background
                        std::thread::spawn(move || {
                            let result = runtime_clone.block_on(async {
                                let manager = addon_manager_clone.lock().unwrap();
                                manager.install_addon(&addon_name_string).await
                            });

                            match result {
                                Ok(_) => {
                                    println!("Addon {} installed successfully", addon_name_string);
                                    if let Some(sender) = operation_sender {
                                        let _ = sender.send((addon_name_string.clone(), true));
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "Failed to install addon {}: {}",
                                        addon_name_string, e
                                    );
                                    eprintln!("{}", error_msg);
                                    if let Some(sender) = error_sender {
                                        let _ = sender.send(error_msg);
                                    }
                                    if let Some(sender) = operation_sender {
                                        let _ = sender.send((addon_name_string.clone(), false));
                                    }
                                }
                            }
                        });
                    }
                }
            }
            "update" => {
                self.add_log(format!("Updating addon: {}", addon_name));

                // Set status to Updating
                if let Some(addon) = self.addons.iter_mut().find(|a| a.name == addon_name) {
                    let current_version = addon.version();
                    addon.status = crate::addon_status::AddonStatus::Updating {
                        current_version,
                        progress: None
                    };
                }

                // Update the addon
                if let Some(runtime) = &self.runtime {
                    if let Some(addon_manager) = &self.addon_manager {
                        let addon_manager_clone = addon_manager.clone();
                        let runtime_clone = runtime.clone();
                        let addon_name_string = addon_name.to_string();
                        let error_sender = self.error_sender.clone();
                        let operation_sender = self.operation_sender.clone();

                        // Run update in background
                        std::thread::spawn(move || {
                            let result = runtime_clone.block_on(async {
                                let manager = addon_manager_clone.lock().unwrap();
                                manager.update_addon(&addon_name_string).await
                            });

                            match result {
                                Ok(updated) => {
                                    if updated {
                                        println!(
                                            "Addon {} updated successfully",
                                            addon_name_string
                                        );
                                    } else {
                                        println!(
                                            "Addon {} is already up to date",
                                            addon_name_string
                                        );
                                    }
                                    if let Some(sender) = operation_sender {
                                        let _ = sender.send((addon_name_string.clone(), true));
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "Failed to update addon {}: {}",
                                        addon_name_string, e
                                    );
                                    eprintln!("{}", error_msg);
                                    if let Some(sender) = error_sender {
                                        let _ = sender.send(error_msg);
                                    }
                                    if let Some(sender) = operation_sender {
                                        let _ = sender.send((addon_name_string.clone(), false));
                                    }
                                }
                            }
                        });
                    }
                }
            }
            "delete" => {
                self.confirming_delete = Some(addon_name.to_string());
            }
            _ => {}
        }
    }
}

impl eframe::App for WowAddonManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load config on first run
        if self.config.is_none() {
            self.load_config();
            let mut style = (*ctx.style()).clone();
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(20.0, egui::FontFamily::Proportional),
            );
            ctx.set_style(style);
        }

        // Check for errors from background threads
        if let Some(receiver) = &self.error_receiver {
            if let Ok(error_msg) = receiver.try_recv() {
                self.error = Some(error_msg.clone());
                self.add_log(format!("ERROR: {}", error_msg));
            }
        }

        // Check for operation results from background threads
        if let Some(receiver) = &self.operation_receiver {
            if let Ok((addon_name, success)) = receiver.try_recv() {
                if success {
                    self.add_log(format!("Operation completed successfully for {}", addon_name));
                    // Refresh only this addon's information
                    if let Some(runtime) = &self.runtime {
                        if let Some(addon_manager) = &self.addon_manager {
                            let addon_manager_clone = addon_manager.clone();
                            let runtime_clone = runtime.clone();
                            let addon_name_clone = addon_name.clone();
                            let addon_update_sender = self.addon_update_sender.clone();

                            std::thread::spawn(move || {
                                let result = runtime_clone.block_on(async {
                                    let manager = addon_manager_clone.lock().unwrap();
                                    manager.get_addon_info(&addon_name_clone).await
                                });

                                if let Ok(addon) = result {
                                    if let Some(sender) = addon_update_sender {
                                        let _ = sender.send(addon);
                                    }
                                }
                            });
                        }
                    }
                } else {
                    // Set error status
                    if let Some(addon) = self.addons.iter_mut().find(|a| a.name == addon_name) {
                        addon.status = crate::addon_status::AddonStatus::Error {
                            message: "Operation failed".to_string(),
                        };
                    }
                }
            }
        }

        // Check for single addon updates
        if let Some(receiver) = &self.addon_update_receiver {
            if let Ok(updated_addon) = receiver.try_recv() {
                // Update only this addon in the list
                if let Some(addon) = self.addons.iter_mut().find(|a| a.name == updated_addon.name) {
                    *addon = updated_addon;
                }
            }
        }

        // Check for addon updates from background thread
        if let Some(receiver) = &self.addon_receiver {
            if let Ok(addons) = receiver.try_recv() {
                self.addons = addons;
                self.loading = false;
                self.add_log("Addons loaded successfully".to_string());
                // Clear the receiver since we've processed the update
                self.addon_receiver = None;
            }
        }

        // Check for update results from background thread
        if let Some(receiver) = &self.update_receiver {
            if let Ok(updates) = receiver.try_recv() {
                self.updates_available = updates.clone();
                self.loading = false;
                self.checking_updates = false;

                // Update addon statuses based on update results
                for addon in &mut self.addons {
                    if let crate::addon_status::AddonStatus::Updating { .. } = addon.status {
                        // Set status based on whether update is available
                        if updates.contains(&addon.name) {
                            addon.status = crate::addon_status::AddonStatus::UpdateAvailable {
                                current: addon.version(),
                                latest: "available".to_string(),
                            };
                        } else if addon.is_installed() {
                            addon.status = crate::addon_status::AddonStatus::Installed {
                                version: addon.version(),
                            };
                        }
                    }
                }

                if self.updates_available.is_empty() {
                    self.add_log("All addons are up to date".to_string());
                } else {
                    self.add_log(
                        format!(
                            "Found {} addons with updates available",
                            self.updates_available.len()
                        )
                        .to_string(),
                    );
                }
                // Clear the receiver since we've processed the update
                self.update_receiver = None;
            }
        }

        // Top panel with title
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("WoW Addon Manager");
            ui.separator();
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show error banner if any
            if let Some(error) = &self.error {
                if error_banner::show_error_banner(ui, error) {
                    self.error = None;
                }
            }

            // Show loading indicator
            if self.loading && !self.checking_updates {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading...");
                });
                ui.separator();
            }

            // Show addons table
            ui.heading("Addons");
            ui.add_space(5.0);

            // Search and filters
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);

                ui.separator();

                ui.checkbox(&mut self.filter_installed_only, "Installed only");
                ui.checkbox(&mut self.filter_updates_only, "Updates available");
            });
            ui.add_space(10.0);

            if self.addons.is_empty() && !self.loading && !self.checking_updates {
                ui.label("No addons configured. Add repositories to your config.yaml file.");
            } else if !self.loading || self.checking_updates {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("addons_grid")
                        .striped(true)
                        .spacing([40.0, 4.0])
                        .min_col_width(150.0)
                        .show(ui, |ui| {
                            // Header
                            ui.label(egui::RichText::new("Addon").strong());
                            ui.label(egui::RichText::new("Version").strong());
                            ui.label(egui::RichText::new("Status").strong());
                            ui.label(egui::RichText::new("Actions").strong());
                            ui.end_row();

                            // Create a copy of addon names to avoid borrowing issues
                            let addon_names: Vec<String> =
                                self.addons.iter().map(|a| a.name.clone()).collect();
                            let addon_versions: Vec<String> =
                                self.addons.iter().map(|a| a.version()).collect();
                            let addon_installed: Vec<bool> =
                                self.addons.iter().map(|a| a.is_installed()).collect();

                            // Addon rows
                            for i in 0..self.addons.len() {
                                // Apply filters
                                let addon_name = &addon_names[i];

                                // Search filter
                                if !self.search_query.is_empty() {
                                    if !addon_name.to_lowercase().contains(&self.search_query.to_lowercase()) {
                                        continue;
                                    }
                                }

                                // Installed only filter
                                if self.filter_installed_only && !addon_installed[i] {
                                    continue;
                                }

                                // Updates available filter
                                if self.filter_updates_only && !self.updates_available.contains(addon_name) {
                                    continue;
                                }

                                let is_selected = self.selected_addon == Some(i);

                                if is_selected {
                                    ui.painter().rect_filled(
                                        ui.available_rect_before_wrap(),
                                        0.0,
                                        egui::Color32::from_rgba_premultiplied(100, 100, 150, 30),
                                    );
                                }

                                ui.label(&addon_names[i]);
                                ui.label(&addon_versions[i]);

                                // Status - use the state machine's display methods
                                let status = &self.addons[i].status;
                                ui.label(
                                    egui::RichText::new(status.display_text())
                                        .color(status.display_color()),
                                );

                                // Actions
                                ui.horizontal(|ui| {
                                    if !addon_installed[i] {
                                        if ui.button("Install").clicked() {
                                            self.handle_addon_action("install", &addon_names[i]);
                                        }
                                    } else {
                                        if ui.button("Update").clicked() {
                                            self.handle_addon_action("update", &addon_names[i]);
                                        }
                                        if ui.button("Delete").clicked() {
                                            self.handle_addon_action("delete", &addon_names[i]);
                                        }
                                    }
                                });

                                ui.end_row();
                            }
                        });
                });
            }
        });

        egui::TopBottomPanel::bottom("bot_panel")
            .min_height(300.0)
            .show(ctx, |ui| {
                ui.separator();

                // Control buttons
                ui.horizontal(|ui| {
                    if ui.button("Refresh").clicked() {
                        self.add_log("Refreshing addons list".to_string());
                        self.load_addons();
                    }

                    if ui.button("Check Updates").clicked() {
                        self.add_log("Checking for updates".to_string());
                        // Check for updates
                        if let Some(runtime) = &self.runtime {
                            if let Some(addon_manager) = &self.addon_manager {
                                self.checking_updates = true;
                                
                                // Set updating status for all installed addons
                                for addon in &mut self.addons {
                                    if addon.is_installed() {
                                        let current_version = addon.version();
                                        addon.status = crate::addon_status::AddonStatus::Updating {
                                            current_version,
                                            progress: None
                                        };
                                    }
                                }
                                
                                let (sender, receiver) = mpsc::channel();
                                self.update_receiver = Some(receiver);

                                let addon_manager_clone = addon_manager.clone();
                                let runtime_clone = runtime.clone();
                                let error_sender = self.error_sender.clone();

                                // Run update check in background
                                std::thread::spawn(move || {
                                    let result = runtime_clone.block_on(async {
                                        let manager = addon_manager_clone.lock().unwrap();
                                        manager.check_for_updates().await
                                    });

                                    match result {
                                        Ok(updates) => {
                                            let _ = sender.send(updates);
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to check for updates: {}", e);
                                            eprintln!("{}", error_msg);
                                            if let Some(err_sender) = error_sender {
                                                let _ = err_sender.send(error_msg);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }

                    if ui.button("Settings").clicked() {
                        self.add_log("Opening settings".to_string());
                        // Load current settings into temporary fields
                        if let Some(config) = &self.config {
                            self.settings_addon_dir = config.addon_dir.clone();
                            self.settings_github_token = config.github_token.clone().unwrap_or_default();
                        }
                        self.settings_open = true;
                    }
                });

                ui.separator();
                // Logs section
                ui.heading("Logs");
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for log in &self.logs {
                            ui.label(log);
                        }
                        // Занимаем всё доступное пространство
                        ui.allocate_space(ui.available_size());
                    });
            });

        // Show settings window if open
        if self.settings_open {
            let should_save = self.show_settings_window(ctx);
            if should_save {
                self.save_settings();
            }
        }

        // Handle delete confirmation outside the main UI update loop
        if let Some(addon_name) = self.confirming_delete.clone() {
            let mut confirmed = None;
            egui::Window::new("Confirm Delete")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Are you sure you want to delete {}?", addon_name));
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            confirmed = Some(true);
                        }
                        if ui.button("No").clicked() {
                            confirmed = Some(false);
                        }
                    });
                });

            if let Some(result) = confirmed {
                if result {
                    self.add_log(format!("Deleting addon: {}", addon_name));
                    // Delete the addon in a background thread
                    if let Some(addon_manager) = &self.addon_manager {
                        let addon_manager_clone = addon_manager.clone();
                        let addon_name_string = addon_name.clone();
                        let error_sender = self.error_sender.clone();

                        std::thread::spawn(move || {
                            let result = {
                                let manager = addon_manager_clone.lock().unwrap();
                                manager.remove_addon(&addon_name_string)
                            };

                            match result {
                                Ok(_) => {
                                    println!("Addon {} deleted successfully", addon_name_string);
                                }
                                Err(e) => {
                                    let error_msg = format!("Failed to delete addon {}: {}", addon_name_string, e);
                                    eprintln!("{}", error_msg);
                                    if let Some(err_sender) = error_sender {
                                        let _ = err_sender.send(error_msg);
                                    }
                                }
                            }
                        });

                        // Refresh the addon list to show updated status
                        self.load_addons();
                    }
                }
                self.confirming_delete = None;
            }
        }

        // Request repaint if loading
        if self.loading {
            ctx.request_repaint();
        }
    }
}
