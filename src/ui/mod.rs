pub mod ace_editor;
mod top_bar;
mod permissions_panel;
mod chat_panel;
mod staged_panel;
mod raw_panel;

use crate::app::{AclApp, Panel};

pub fn draw(app: &mut AclApp, ctx: &egui::Context) {
    top_bar::draw(app, ctx);

    egui::CentralPanel::default().show(ctx, |ui| {
        // Tab selector
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.panel, Panel::Permissions, "🔒 Permissions");
            ui.selectable_value(&mut app.panel, Panel::Staged,      "📋 Staged");
            ui.selectable_value(&mut app.panel, Panel::Raw,         "📄 Raw Output");
            ui.selectable_value(&mut app.panel, Panel::Chat,        "💬 Help Chat");
        });
        ui.separator();

        match app.panel {
            Panel::Permissions => permissions_panel::draw(app, ui),
            Panel::Chat        => chat_panel::draw(app, ui),
            Panel::Staged      => staged_panel::draw(app, ui),
            Panel::Raw         => raw_panel::draw(app, ui),
        }
    });

    // ACE editor dialog
    ace_editor::draw_dialog(app, ctx);
}
