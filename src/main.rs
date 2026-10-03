#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

mod addon;
mod addon_status;
mod app;
mod config;
mod github;
mod ui;

fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([300.0, 220.0]),
        default_theme: eframe::Theme::Light,
        ..Default::default()
    };

    eframe::run_native(
        "WoW Addon Manager",
        options,
        Box::new(|_cc| Box::new(app::WowAddonManagerApp::default()) as Box<dyn eframe::App>),
    )
}
