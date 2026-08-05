use crate::app::AclApp;

pub fn draw(app: &mut AclApp, ui: &mut egui::Ui) {
    let Some(acl) = &app.acl else {
        ui.label("No path loaded.");
        return;
    };

    let tool_label = match acl.kind {
        acl_core::AclKind::PosixMode   => "stat",
        acl_core::AclKind::PosixAcl    => "getfacl",
        acl_core::AclKind::MacosAcl    => "ls -le",
        acl_core::AclKind::WindowsDacl => "icacls",
        acl_core::AclKind::Nfs4Acl     => "nfs4_getfacl",
        acl_core::AclKind::Unknown     => "unknown",
    };

    ui.label(format!("Raw output from {tool_label}:"));
    ui.separator();

    let raw = acl.raw_output.clone();
    egui::ScrollArea::both().show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut raw.as_str())
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY),
        );
    });
}
