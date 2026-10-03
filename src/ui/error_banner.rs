use eframe::egui;

/// Display an error banner with dismiss button
/// Returns true if the error should be dismissed
pub fn show_error_banner(ui: &mut egui::Ui, error: &str) -> bool {
    let mut should_dismiss = false;
    let error_text = error.to_string();

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(255, 200, 200))
        .inner_margin(10.0)
        .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 0, 0)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚠")
                        .size(24.0)
                        .color(egui::Color32::from_rgb(200, 0, 0)),
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Error")
                            .strong()
                            .color(egui::Color32::from_rgb(150, 0, 0)),
                    );
                    ui.label(&error_text);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕ Dismiss").clicked() {
                        should_dismiss = true;
                    }
                });
            });
        });

    ui.add_space(10.0);
    should_dismiss
}
