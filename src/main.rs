// Prevent a console window on Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod elevation;
mod ui;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ACL GUI — Cross-platform ACL & Permission Manager")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([800.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "aclgui",
        native_options,
        Box::new(|cc| Ok(Box::new(app::AclApp::new(cc)))),
    )
}
