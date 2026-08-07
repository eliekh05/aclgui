use std::path::PathBuf;

use acl_core::{
    ChangeSet, PathAcl, ToolAvailability,
    apply_changes, read_path, probe_tools,
};

use crate::ui;

/// The panel currently shown on the right.
#[derive(PartialEq, Eq)]
pub enum Panel {
    Permissions,
    Staged,
    Raw,
}

pub struct AclApp {
    // Path selection
    pub current_path: Option<PathBuf>,
    pub path_input: String,

    // Loaded state
    pub acl: Option<PathAcl>,
    pub tools: ToolAvailability,

    // Staged changes
    pub staged: ChangeSet,

    // Apply result
    pub apply_result: Option<Result<String, String>>,

    // UI state
    pub panel: Panel,
    pub status: String,

    // ACE editor dialog state
    pub ace_editor: ui::ace_editor::AceEditorState,
}

impl AclApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let tools = probe_tools();
        Self {
            current_path: None,
            path_input: String::new(),
            acl: None,
            tools,
            staged: ChangeSet::default(),
            apply_result: None,
            panel: Panel::Permissions,
            status: String::new(),
            ace_editor: ui::ace_editor::AceEditorState::default(),
        }
    }

    pub fn load_path(&mut self, path: PathBuf) {
        self.staged = ChangeSet { path: path.to_string_lossy().into(), changes: vec![] };
        self.apply_result = None;
        let acl = read_path(&path, &self.tools);
        self.status = if let Some(ref e) = acl.error {
            format!("Error: {e}")
        } else {
            format!("Loaded: {}", path.display())
        };
        self.acl = Some(acl);
        self.current_path = Some(path);
    }

    pub fn pick_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.path_input = path.to_string_lossy().into();
            self.load_path(path);
        }
    }

    pub fn pick_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.path_input = path.to_string_lossy().into();
            self.load_path(path);
        }
    }

    pub fn reload(&mut self) {
        if let Some(path) = self.current_path.clone() {
            self.load_path(path);
        }
    }

    pub fn apply_staged(&mut self) {
        if !self.tools.is_elevated {
            let path = self.current_path.as_ref()
                .map(|p| p.to_string_lossy().into_owned());
            match crate::elevation::relaunch_elevated(path.as_deref()) {
                Ok(_) => {} // process will exit
                Err(e) => {
                    self.apply_result = Some(Err(format!(
                        "Could not elevate: {e}. Try using 'Re-launch as admin' in the top bar."
                    )));
                }
            }
            return;
        }
        let result = apply_changes(&self.staged, &self.tools);
        self.apply_result = Some(result);
        if let Some(path) = self.current_path.clone() {
            self.load_path(path);
        }
        self.staged.changes.clear();
    }
}

impl eframe::App for AclApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::draw(self, ctx);
    }
}
