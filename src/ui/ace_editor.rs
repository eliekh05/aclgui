use crate::app::AclApp;
use acl_core::model::*;

#[derive(Default)]
pub struct AceEditorState {
    pub open: bool,
    pub mode: AceEditorMode,
    pub edit_index: usize,
    pub is_default: bool,

    // Fields
    pub principal_kind: PrincipalKind,
    pub principal_name: String,
    pub allow: bool,
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub delete: bool,
    pub append: bool,
    pub create_file: bool,
    pub create_dir: bool,
    pub read_security: bool,
    pub write_security: bool,
    pub take_ownership: bool,
    pub file_inherit: bool,
    pub dir_inherit: bool,
    pub inherit_only: bool,
    pub is_default_ace: bool,
}

#[derive(Default, PartialEq)]
pub enum AceEditorMode {
    #[default]
    Add,
    Edit,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum PrincipalKind {
    #[default]
    User,
    Group,
    Everyone,
}

impl AceEditorState {
    pub fn open_add(&mut self) {
        *self = AceEditorState::default();
        self.open = true;
        self.allow = true;
    }

    pub fn open_edit(&mut self, index: usize, ace: Ace, default: bool) {
        *self = AceEditorState::default();
        self.open = true;
        self.mode = AceEditorMode::Edit;
        self.edit_index = index;
        self.is_default = default;
        self.allow = ace.allow;

        match ace.principal {
            Principal::User(n) => {
                self.principal_kind = PrincipalKind::User;
                self.principal_name = n;
            }
            Principal::Group(n) => {
                self.principal_kind = PrincipalKind::Group;
                self.principal_name = n;
            }
            Principal::Everyone => {
                self.principal_kind = PrincipalKind::Everyone;
            }
            other => {
                self.principal_name = other.display();
            }
        }

        let r = &ace.rights;
        self.read = r.read;
        self.write = r.write;
        self.execute = r.execute;
        self.delete = r.delete;
        self.append = r.append;
        self.create_file = r.create_file;
        self.create_dir = r.create_dir;
        self.read_security = r.read_security;
        self.write_security = r.write_security;
        self.take_ownership = r.take_ownership;

        let ih = &ace.inherit;
        self.file_inherit = ih.file_inherit || ih.object_inherit;
        self.dir_inherit = ih.dir_inherit || ih.container_inherit;
        self.inherit_only = ih.inherit_only;
        self.is_default_ace = ace.is_default;
    }

    fn to_ace(&self) -> Ace {
        let principal = match self.principal_kind {
            PrincipalKind::User => Principal::User(self.principal_name.clone()),
            PrincipalKind::Group => Principal::Group(self.principal_name.clone()),
            PrincipalKind::Everyone => Principal::Everyone,
        };
        Ace {
            principal,
            allow: self.allow,
            rights: Rights {
                read: self.read,
                write: self.write,
                execute: self.execute,
                delete: self.delete,
                append: self.append,
                create_file: self.create_file,
                create_dir: self.create_dir,
                read_security: self.read_security,
                write_security: self.write_security,
                take_ownership: self.take_ownership,
                list: self.read,
                ..Default::default()
            },
            inherit: InheritFlags {
                file_inherit: self.file_inherit,
                object_inherit: self.file_inherit,
                dir_inherit: self.dir_inherit,
                container_inherit: self.dir_inherit,
                inherit_only: self.inherit_only,
                ..Default::default()
            },
            is_default: self.is_default_ace,
        }
    }
}

pub fn draw_dialog(app: &mut AclApp, ctx: &egui::Context) {
    if !app.ace_editor.open {
        return;
    }

    let title = match app.ace_editor.mode {
        AceEditorMode::Add => "Add ACE",
        AceEditorMode::Edit => "Edit ACE",
    };

    let mut open = true;
    let mut stage = false;
    let mut cancel = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .min_width(360.0)
        .open(&mut open)
        .show(ctx, |ui| {
            let ed = &mut app.ace_editor;

            ui.group(|ui| {
                ui.label("Principal");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut ed.principal_kind, PrincipalKind::User, "User");
                    ui.selectable_value(&mut ed.principal_kind, PrincipalKind::Group, "Group");
                    ui.selectable_value(
                        &mut ed.principal_kind,
                        PrincipalKind::Everyone,
                        "Everyone",
                    );
                });
                if ed.principal_kind != PrincipalKind::Everyone {
                    ui.text_edit_singleline(&mut ed.principal_name);
                }
            });

            ui.horizontal(|ui| {
                ui.radio_value(&mut ed.allow, true, "Allow");
                ui.radio_value(&mut ed.allow, false, "Deny");
            });

            ui.group(|ui| {
                ui.label("Rights");
                ui.columns(3, |cols| {
                    cols[0].checkbox(&mut ed.read, "Read");
                    cols[0].checkbox(&mut ed.write, "Write");
                    cols[0].checkbox(&mut ed.execute, "Execute");
                    cols[1].checkbox(&mut ed.delete, "Delete");
                    cols[1].checkbox(&mut ed.append, "Append");
                    cols[1].checkbox(&mut ed.create_file, "Create file");
                    cols[2].checkbox(&mut ed.create_dir, "Create dir");
                    cols[2].checkbox(&mut ed.read_security, "Read perm");
                    cols[2].checkbox(&mut ed.write_security, "Write perm");
                });
            });

            ui.group(|ui| {
                ui.label("Inheritance");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut ed.file_inherit, "File inherit");
                    ui.checkbox(&mut ed.dir_inherit, "Dir inherit");
                    ui.checkbox(&mut ed.inherit_only, "Inherit only");
                });
                ui.checkbox(
                    &mut ed.is_default_ace,
                    "Default ACE (Linux/NFSv4 directories)",
                );
            });

            ui.separator();
            ui.horizontal(|ui| {
                let has_rights = ed.read || ed.write || ed.execute || ed.delete
                    || ed.append || ed.create_file || ed.create_dir
                    || ed.read_security || ed.write_security;
                if ui.add_enabled(has_rights, egui::Button::new("Stage")).clicked() {
                    stage = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });

    if stage {
        let ace = app.ace_editor.to_ace();
        let change = match app.ace_editor.mode {
            AceEditorMode::Add => {
                let is_default = app.ace_editor.is_default_ace;
                Change::AddAce {
                    ace,
                    default: is_default,
                }
            }
            AceEditorMode::Edit => Change::ModifyAce {
                index: app.ace_editor.edit_index,
                ace,
                default: app.ace_editor.is_default,
            },
        };
        app.staged.changes.push(change);
        app.status = "ACE staged. Switch to the Staged tab to review.".into();
        app.ace_editor.open = false;
    }
    if cancel || !open {
        app.ace_editor.open = false;
    }
}
