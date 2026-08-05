use acl_core::model::Change;
use crate::app::AclApp;

pub fn draw(app: &mut AclApp, ui: &mut egui::Ui) {
    if app.staged.changes.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No staged changes. Edit permissions in the Permissions tab.");
        });
        return;
    }

    ui.label(egui::RichText::new(format!("Target: {}", app.staged.path)).strong());
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut remove = None;

        for (i, change) in app.staged.changes.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{i}."));
                ui.label(describe_change(change));
                if ui.small_button("✖").clicked() {
                    remove = Some(i);
                }
            });
        }

        if let Some(i) = remove {
            app.staged.changes.remove(i);
        }
    });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("🗑 Discard all").clicked() {
            app.staged.changes.clear();
        }

        let apply_label = if app.tools.is_elevated {
            "✅ Apply"
        } else {
            "⚠ Apply (will request elevation)"
        };

        if ui.button(apply_label).clicked() {
            app.apply_staged();
            app.panel = crate::app::Panel::Permissions;
        }
    });

    if !app.tools.is_elevated {
        ui.colored_label(
            egui::Color32::from_rgb(220, 160, 40),
            "The process is not elevated. Applying will re-launch with admin/sudo privileges.",
        );
    }
}

fn describe_change(change: &Change) -> String {
    match change {
        Change::SetMode { octal }     => format!("Set mode → {octal:04o}"),
        Change::SetOwner { user }     => format!("Set owner → {user}"),
        Change::SetGroup { group }    => format!("Set group → {group}"),
        Change::AddAce { ace, default } => format!(
            "Add {} ACE for {} ({}, {})",
            if *default { "default" } else { "access" },
            ace.principal.display(),
            if ace.allow { "Allow" } else { "Deny" },
            ace.rights.summary(),
        ),
        Change::RemoveAce { index, default } => format!(
            "Remove {} ACE #{index}",
            if *default { "default" } else { "access" }
        ),
        Change::ModifyAce { index, ace, .. } => format!(
            "Modify ACE #{index} → {} {}",
            if ace.allow { "Allow" } else { "Deny" },
            ace.rights.summary(),
        ),
        Change::DisableInheritance { copy_existing } => format!(
            "Disable inheritance ({})",
            if *copy_existing { "copy existing" } else { "remove inherited" }
        ),
        Change::EnableInheritance  => "Enable inheritance".into(),
        Change::RemoveAllAces      => "Remove ALL ACEs".into(),
    }
}
