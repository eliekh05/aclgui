// Prevent a console window on Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod elevation;
mod ui;

fn main() -> eframe::Result {
    // If relaunched elevated with --path=<p>, auto-load that path.
    let initial_path: Option<std::path::PathBuf> = std::env::args()
        .find(|a| a.starts_with("--path="))
        .map(|a| std::path::PathBuf::from(a.trim_start_matches("--path=")));

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
        Box::new(move |cc| {
            let mut app = app::AclApp::new(cc);
            if let Some(path) = initial_path {
                if path.exists() {
                    app.path_input = path.to_string_lossy().into();
                    app.load_path(path);
                }
            }
            Ok(Box::new(app))
        }),
    )
}
