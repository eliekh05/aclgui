use crate::app::AclApp;

pub fn draw(app: &mut AclApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("ACL GUI");
            ui.separator();

            ui.label("Path:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut app.path_input)
                    .hint_text("Enter path or pick below…")
                    .desired_width(420.0),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let path = std::path::PathBuf::from(&app.path_input);
                if path.exists() {
                    app.load_path(path);
                } else {
                    app.status = format!("Path does not exist: {}", app.path_input);
                }
            }

            if ui.button("📁 File").clicked() { app.pick_file(); }
            if ui.button("📂 Dir").clicked()  { app.pick_dir();  }
            if ui.button("🔄 Reload").clicked() { app.reload(); }

            ui.separator();

            // Elevation status badge
            if app.tools.is_elevated {
                ui.colored_label(egui::Color32::from_rgb(100, 220, 100), "✔ Elevated");
            } else {
                ui.colored_label(egui::Color32::from_rgb(220, 120, 60), "⚠ Not elevated");
                if ui.button("Re-launch as admin").clicked() {
                    if let Err(e) = crate::elevation::relaunch_elevated() {
                        app.status = format!("Elevation failed: {e}");
                    }
                }
            }

            // OS badge
            ui.separator();
            let os_label = if cfg!(target_os = "windows") { "🪟 Windows" }
                      else if cfg!(target_os = "macos")   { " macOS"   }
                      else if cfg!(target_os = "linux")   { "🐧 Linux"  }
                      else                                 { "❓ Unknown" };
            ui.label(os_label);
        });

        if !app.status.is_empty() {
            ui.separator();
            let color = if app.status.starts_with("Error") {
                egui::Color32::from_rgb(230, 80, 80)
            } else {
                egui::Color32::GRAY
            };
            ui.colored_label(color, &app.status);
        }
    });
}
