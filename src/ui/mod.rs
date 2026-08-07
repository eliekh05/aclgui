pub mod ace_editor;
mod top_bar;
mod permissions_panel;
mod staged_panel;
mod raw_panel;

use crate::app::{AclApp, Panel};

pub fn draw(app: &mut AclApp, ctx: &egui::Context) {
    top_bar::draw(app, ctx);

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.panel, Panel::Permissions, "🔒 Permissions");
            ui.selectable_value(&mut app.panel, Panel::Staged,      "📋 Staged");
            ui.selectable_value(&mut app.panel, Panel::Raw,         "📄 Raw Output");
        });
        ui.separator();

        match app.panel {
            Panel::Permissions => permissions_panel::draw(app, ui),
            Panel::Staged      => staged_panel::draw(app, ui),
            Panel::Raw         => raw_panel::draw(app, ui),
        }
    });

    ace_editor::draw_dialog(app, ctx);
}
