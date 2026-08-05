use crate::app::AclApp;
use acl_core::model::*;

pub fn draw(app: &mut AclApp, ui: &mut egui::Ui) {
    let Some(acl) = app.acl.as_ref() else {
        ui.centered_and_justified(|ui| {
            ui.label("No path loaded. Use the top bar to select a file or directory.");
        });
        return;
    };

    // Clone what we need to avoid borrowing issues
    let acl = acl.clone();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── Owner / Group / Kind ──────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("File Info").strong());
            egui::Grid::new("info_grid")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    ui.label("ACL type:");
                    ui.label(format!("{:?}", acl.kind));
                    ui.end_row();

                    ui.label("Kind:");
                    ui.label(if acl.is_dir { "Directory" } else { "File" });
                    ui.end_row();

                    if let Some(o) = &acl.owner {
                        ui.label("Owner:");
                        ui.label(o);
                        ui.end_row();
                    }
                    if let Some(g) = &acl.group {
                        ui.label("Group:");
                        ui.label(g);
                        ui.end_row();
                    }
                });
        });

        ui.add_space(8.0);

        // ── POSIX mode bits ───────────────────────────────────────────
        if let Some(mode) = &acl.posix_mode {
            ui.group(|ui| {
                ui.label(egui::RichText::new("POSIX Mode Bits").strong());
                ui.horizontal(|ui| {
                    ui.monospace(format!("{:04o}", mode.to_octal()));
                    ui.label("→");
                    ui.monospace(mode.symbolic());
                    if mode.setuid {
                        ui.label("setuid");
                    }
                    if mode.setgid {
                        ui.label("setgid");
                    }
                    if mode.sticky {
                        ui.label("sticky");
                    }
                });
                draw_mode_grid(ui, mode);
            });
            ui.add_space(8.0);
        }

        // ── ACE table ─────────────────────────────────────────────────
        if !acl.aces.is_empty() || !acl.default_aces.is_empty() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Access Control Entries").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ Add ACE").clicked() {
                            app.ace_editor.open_add();
                        }
                    });
                });

                draw_ace_table(ui, &acl.aces, false, app);

                if !acl.default_aces.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Default ACEs (inherited by new items)").italics(),
                    );
                    draw_ace_table(ui, &acl.default_aces, true, app);
                }
            });
        }

        // ── Apply result ──────────────────────────────────────────────
        if let Some(ref result) = app.apply_result {
            ui.add_space(8.0);
            ui.group(|ui| match result {
                Ok(msg) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(80, 200, 80),
                        "✔ Applied successfully",
                    );
                    ui.label(msg);
                }
                Err(e) => {
                    ui.colored_label(egui::Color32::from_rgb(230, 80, 80), format!("✘ {e}"));
                }
            });
        }
    });
}

fn draw_mode_grid(ui: &mut egui::Ui, mode: &PosixMode) {
    egui::Grid::new("mode_grid")
        .num_columns(4)
        .spacing([12.0, 2.0])
        .show(ui, |ui| {
            ui.label("");
            ui.label("Read");
            ui.label("Write");
            ui.label("Execute");
            ui.end_row();

            let bit = |b: bool| if b { "✔" } else { "✖" };

            ui.label("Owner");
            ui.label(bit(mode.owner_read));
            ui.label(bit(mode.owner_write));
            ui.label(bit(mode.owner_execute));
            ui.end_row();

            ui.label("Group");
            ui.label(bit(mode.group_read));
            ui.label(bit(mode.group_write));
            ui.label(bit(mode.group_execute));
            ui.end_row();

            ui.label("Other");
            ui.label(bit(mode.other_read));
            ui.label(bit(mode.other_write));
            ui.label(bit(mode.other_execute));
            ui.end_row();
        });
}

fn draw_ace_table(ui: &mut egui::Ui, aces: &[Ace], default: bool, app: &mut AclApp) {
    egui::Grid::new(if default {
        "ace_grid_default"
    } else {
        "ace_grid"
    })
    .num_columns(7)
    .striped(true)
    .spacing([8.0, 4.0])
    .show(ui, |ui| {
        // Header
        ui.label(egui::RichText::new("Principal").strong());
        ui.label(egui::RichText::new("Type").strong());
        ui.label(egui::RichText::new("r w x").strong());
        ui.label(egui::RichText::new("Delete").strong());
        ui.label(egui::RichText::new("Inherit").strong());
        ui.label(egui::RichText::new("").strong());
        ui.end_row();

        let mut remove_idx: Option<usize> = None;

        for (i, ace) in aces.iter().enumerate() {
            ui.label(ace.principal.display());

            let (type_text, type_color) = if ace.allow {
                ("Allow", egui::Color32::from_rgb(80, 200, 80))
            } else {
                ("Deny", egui::Color32::from_rgb(230, 80, 80))
            };
            ui.colored_label(type_color, type_text);

            // rwx summary
            let r = &ace.rights;
            ui.monospace(format!(
                "{}{}{}",
                bit(r.read || r.list),
                bit(r.write || r.create_file),
                bit(r.execute)
            ));

            ui.label(if r.delete { "✔" } else { "" });

            // Inherit summary
            let ih = &ace.inherit;
            let mut inh = String::new();
            if ih.file_inherit || ih.object_inherit {
                inh.push('f');
            }
            if ih.dir_inherit || ih.container_inherit {
                inh.push('d');
            }
            if ih.inherited {
                inh.push_str("(i)");
            }
            ui.label(inh);

            // Actions
            ui.horizontal(|ui| {
                if ui.small_button("✏").clicked() {
                    app.ace_editor.open_edit(i, ace.clone(), default);
                }
                if ui.small_button("🗑").clicked() {
                    remove_idx = Some(i);
                }
            });
            ui.end_row();
        }

        if let Some(idx) = remove_idx {
            app.staged.changes.push(Change::RemoveAce {
                index: idx,
                default,
            });
            app.status = format!("Staged: remove ACE #{idx}. Apply when ready.");
        }
    });
}

fn bit(b: bool) -> char {
    if b {
        '●'
    } else {
        '○'
    }
}
