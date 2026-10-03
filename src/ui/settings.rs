use eframe::egui;

pub struct SettingsWindow;

impl SettingsWindow {
    /// Show settings window and return (should_save, should_close)
    pub fn show(
        ctx: &egui::Context,
        is_open: &mut bool,
        addon_dir: &mut String,
        github_token: &mut String,
    ) -> (bool, bool) {
        let mut should_save = false;
        let mut should_close = false;

        egui::Window::new("Settings")
            .open(is_open)
            .collapsible(false)
            .resizable(false)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("Application Settings");
                ui.separator();

                ui.add_space(10.0);

                // WoW AddOns Directory
                ui.horizontal(|ui| {
                    ui.label("WoW AddOns Directory:");
                });
                ui.text_edit_singleline(addon_dir);
                ui.add_space(5.0);
                ui.label("Example: C:\\Program Files\\World of Warcraft\\Interface\\AddOns");

                ui.add_space(15.0);

                // GitHub Token
                ui.horizontal(|ui| {
                    ui.label("GitHub Token (optional):");
                });
                ui.add(
                    egui::TextEdit::singleline(github_token)
                        .password(true)
                        .hint_text("Leave empty to use GITHUB_TOKEN environment variable"),
                );
                ui.add_space(5.0);
                ui.label("⚠️ Token will be saved in config.yaml (not encrypted)");

                ui.add_space(20.0);
                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        should_save = true;
                        should_close = true;
                    }

                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        (should_save, should_close)
    }
}
