//! File tree interaction state, independent of preview, Git and chat runtimes.
use iced::Point;
use desktop_file_tree::selection::FileTreeSelection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTreeClipboardMode { Copy, Cut }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTreeClipboard {
    pub mode: FileTreeClipboardMode,
    pub src_paths: Vec<String>,
}

#[derive(Debug, Default)]
pub struct InteractionState {
    pub file_tree_selection: FileTreeSelection,
    pub file_tree_menu_path: Option<String>,
    pub file_tree_menu_anchor: Option<Point>,
    pub file_tree_menu_source: Option<String>,
    pub file_tree_clipboard: Option<FileTreeClipboard>,
    pub dragging_file_paths: Vec<String>,
    pub file_tree_drag_source: Option<(String, String)>,
    pub file_tree_drop_target: Option<String>,
    pub file_tree_operation_in_progress: bool,
    pub dragging_file_position: Option<(usize, usize)>,
    pub pending_drop_file_paths: Vec<String>,
    pub pending_drop_file_position: Option<(usize, usize)>,
    pub input_drop_hovered: bool,
    pub file_tree_rename_path: Option<String>,
    pub file_tree_rename_value: String,
    pub file_tree_pending_delete: Option<(String, Vec<String>)>,
}
impl InteractionState {
    /// Discard an unclaimed native file drop without resetting other drag state.
    pub fn discard_pending_drop(&mut self) { self.pending_drop_file_paths.clear(); }

    pub fn hover_external_file(&mut self, path: String) {
        if self.dragging_file_paths.is_empty()
            && !self.pending_drop_file_paths.iter().any(|existing| existing == &path)
        {
            self.pending_drop_file_paths.push(path);
            self.pending_drop_file_position = None;
        }
    }
    pub fn drop_external_file(&mut self, path: String) {
        self.file_tree_drag_source = None;
        self.file_tree_drop_target = None;
        self.dragging_file_paths.clear();
        self.dragging_file_position = None;
        self.pending_drop_file_paths = vec![path];
        self.pending_drop_file_position = None;
    }
    pub fn leave_external_files(&mut self) {
        if self.dragging_file_paths.is_empty() {
            self.pending_drop_file_paths.clear();
            self.pending_drop_file_position = None;
            self.input_drop_hovered = false;
        }
    }
    /// Finalize a chat drop after the host has given internal tree moves priority.
    pub fn release_chat_drop(&mut self) -> bool {
        self.file_tree_drag_source = None;
        self.file_tree_drop_target = None;
        self.end_chat_drag();
        let hovered = self.input_drop_hovered;
        self.input_drop_hovered = false;
        if hovered && !self.pending_drop_file_paths.is_empty() {
            return true;
        }
        self.pending_drop_file_paths.clear();
        self.pending_drop_file_position = None;
        false
    }
    pub fn clear_drag(&mut self) {
        self.file_tree_drag_source = None;
        self.file_tree_drop_target = None;
        self.dragging_file_paths.clear();
        self.dragging_file_position = None;
        self.pending_drop_file_paths.clear();
        self.pending_drop_file_position = None;
        self.input_drop_hovered = false;
    }
    pub fn close_menu(&mut self) {
        self.file_tree_menu_path = None;
        self.file_tree_menu_source = None;
        self.file_tree_menu_anchor = None;
    }
    pub fn cancel_rename(&mut self) {
        self.file_tree_rename_path = None;
        self.file_tree_rename_value.clear();
    }
    pub fn start_drag(&mut self, path: String, position: Option<(usize, usize)>) {
        self.file_tree_drag_source = None;
        self.file_tree_drop_target = None;
        self.dragging_file_paths = vec![path];
        self.dragging_file_position = position;
        self.pending_drop_file_paths.clear();
        self.pending_drop_file_position = None;
        self.input_drop_hovered = false;
    }
    pub fn end_chat_drag(&mut self) {
        if self.dragging_file_paths.is_empty() { return; }
        self.pending_drop_file_paths = self.dragging_file_paths.clone();
        self.pending_drop_file_position = self.dragging_file_position;
        self.dragging_file_paths.clear();
        self.dragging_file_position = None;
    }
    pub fn menu_paths(&self, root: Option<&str>, visible: &[String]) -> Vec<String> {
        let Some(path) = &self.file_tree_menu_path else { return vec![] };
        if matches!(self.file_tree_menu_source.as_deref(), Some("tree-file" | "tree-dir"))
            && let Some(root) = root
            && self.file_tree_selection.contains(root, path)
        {
            let paths = visible.iter().filter(|path| self.file_tree_selection.contains(root, path)).cloned().collect::<Vec<_>>();
            if !paths.is_empty() { return paths; }
        }
        vec![path.clone()]
    }
}

impl InteractionState {
    /// Consume confirmation once, reject stale projects, then optimistically remove roots.
    pub fn prepare_delete(&mut self, current_root: Option<&str>, files: &mut Vec<String>) -> Option<(String, Vec<(String, bool)>)> {
        let (root, paths) = self.file_tree_pending_delete.take()?;
        if current_root != Some(root.as_str()) || self.file_tree_operation_in_progress { return None; }
        let paths = crate::file_tree::top_level_paths(paths).into_iter().map(|path| {
            let recursive = crate::file_tree::is_directory_path(&path, files);
            (path, recursive)
        }).collect::<Vec<_>>();
        if paths.is_empty() { return None; }
        for (path, recursive) in &paths {
            crate::file_tree::remove_local_file_tree_path(files, path, *recursive);
        }
        self.file_tree_operation_in_progress = true;
        Some((root, paths))
    }
    pub fn prepare_paste(&mut self, current_root: Option<&str>, files: &[String], destination: &str) -> Option<(String, FileTreeClipboard, Vec<(String, String)>)> {
        let clipboard = self.file_tree_clipboard.clone()?;
        let root = current_root?.to_string();
        let targets = crate::file_tree::paste_targets(files, destination, &clipboard.src_paths);
        if targets.is_empty() { return None; }
        self.file_tree_operation_in_progress = true;
        Some((root, clipboard, targets))
    }
}

impl InteractionState {
    /// Returns whether the selected entry should also open or toggle in its host.
    pub fn select(&mut self, root: &str, path: &str, command: bool, shift: bool, visible: &[String]) -> bool {
        if !visible.iter().any(|item| item == path) { return false; }
        self.cancel_rename();
        self.file_tree_selection.select(root, path.to_string(), command, shift, visible);
        !command && !shift
    }
    pub fn right_click(&mut self, root: Option<&str>, path: String, source: String, anchor: Point, visible: &[String]) {
        if matches!(source.as_str(), "tree-file" | "tree-dir")
            && let Some(root) = root
            && !self.file_tree_selection.contains(root, &path)
        {
            self.file_tree_selection.select(root, path.clone(), false, false, visible);
        }
        self.file_tree_menu_path = Some(path);
        self.file_tree_menu_source = Some(source);
        self.file_tree_menu_anchor = Some(anchor);
    }
    pub fn complete_paste(&mut self, clipboard: FileTreeClipboard, completed_sources: &[String]) {
        self.file_tree_operation_in_progress = false;
        if matches!(clipboard.mode, FileTreeClipboardMode::Cut) && self.file_tree_clipboard.as_ref() == Some(&clipboard) {
            let mut remaining = clipboard;
            remaining.src_paths.retain(|path| !completed_sources.contains(path));
            self.file_tree_clipboard = (!remaining.src_paths.is_empty()).then_some(remaining);
        }
    }
    pub fn complete_delete(&mut self, current: bool, deleted_paths: &[String]) {
        self.file_tree_operation_in_progress = false;
        if current {
            self.file_tree_selection.paths.retain(|path| !deleted_paths.iter().any(|deleted| std::path::Path::new(path).starts_with(deleted)));
        }
    }
}

impl InteractionState {
    pub fn relocate(&mut self, root: &str, source: &str, target: &str, expanded: &mut [String]) {
        self.file_tree_selection.relocate(root, source, target);
        if let (Ok(old), Ok(new)) = (std::path::Path::new(source).strip_prefix(root), std::path::Path::new(target).strip_prefix(root)) {
            let old = old.to_string_lossy();
            let new = new.to_string_lossy();
            for key in expanded {
                if let Some(updated) = crate::file_tree::replace_path_prefix(key, &old, &new) { *key = updated; }
            }
        }
    }
}

pub struct Rename {
    pub root: String,
    pub source: String,
    pub target: String,
}
pub struct InvalidName;
impl InteractionState {
    pub fn prepare_rename(&mut self, root: Option<&str>, files: &[String]) -> Result<Option<Rename>, InvalidName> {
        let Some(source) = self.file_tree_rename_path.clone() else { return Ok(None); };
        let name = self.file_tree_rename_value.trim();
        if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') { return Err(InvalidName); }
        let old = std::path::Path::new(&source);
        let Some(parent) = old.parent() else { return Ok(None); };
        if old.file_name().and_then(|name| name.to_str()) == Some(name) {
            self.cancel_rename();
            return Ok(None);
        }
        let existing = files.iter().cloned().collect();
        let target = crate::file_tree::unique_name_for_target(parent, name, &existing);
        let Some(root) = root else { return Ok(None); };
        let rename = Rename { root: root.to_string(), source, target };
        self.cancel_rename();
        Ok(Some(rename))
    }
    pub fn start_internal_drag(&mut self, root: String, source: String, visible: &[String]) {
        let paths = if self.file_tree_selection.contains(&root, &source) {
            visible.iter().filter(|path| self.file_tree_selection.contains(&root, path)).cloned().collect::<Vec<_>>()
        } else {
            if visible.contains(&source) { self.file_tree_selection.select(&root, source.clone(), false, false, visible); }
            vec![source.clone()]
        };
        self.start_drag(source.clone(), None);
        self.dragging_file_paths = if paths.is_empty() { vec![source.clone()] } else { paths };
        self.file_tree_drag_source = Some((root, source));
    }
}

impl InteractionState {
    pub fn set_clipboard(&mut self, mode: FileTreeClipboardMode, paths: Vec<String>) {
        self.file_tree_clipboard = Some(FileTreeClipboard { mode, src_paths: crate::file_tree::top_level_paths(paths) });
    }
    pub fn begin_rename(&mut self, path: String) {
        self.file_tree_rename_value = std::path::Path::new(&path).file_name().and_then(|name| name.to_str()).unwrap_or("").to_string();
        self.file_tree_rename_path = Some(path);
    }
    pub fn set_drop_target(&mut self, current_root: Option<&str>, target: Option<String>) {
        self.file_tree_drop_target = if self.file_tree_drag_source.as_ref().is_some_and(|(root, _)| current_root == Some(root.as_str())) && !self.file_tree_operation_in_progress { target } else { None };
    }
}

/// Handle interactions that need no filesystem, navigation or editor ownership.
pub fn handle_local<P, H, U>(state: &mut InteractionState, message: &crate::project_message::message::ProjectMessage<P, H, U>) -> bool {
    use crate::project_message::message::ProjectMessage as M;
    match message {
        M::FileTreeMenuClose => state.close_menu(),
        M::FileTreeRenameChanged(value) => state.file_tree_rename_value = value.clone(),
        M::FileTreeRenameCancel => state.cancel_rename(),
        M::FileTreeDragStart(path, position) => state.start_drag(path.clone(), *position),
        M::FileTreeDragEnd if state.file_tree_drag_source.is_none() || state.file_tree_drop_target.is_none() => state.end_chat_drag(),
        M::FileTreeDeleteCanceled => state.file_tree_pending_delete = None,
        _ => return false,
    }
    true
}

impl InteractionState {
    /// Consume a frozen gesture before validation, preventing a second chat delivery.
    pub fn prepare_move(&mut self, current_root: Option<&str>, root: &str, source: &str, directory: &str, files: &[String]) -> Option<Vec<(String, String)>> {
        if self.file_tree_operation_in_progress
            || self.file_tree_drag_source.as_ref() != Some(&(root.to_string(), source.to_string())) {
            return None;
        }
        let sources = crate::file_tree::top_level_paths(self.dragging_file_paths.clone());
        self.clear_drag();
        if current_root != Some(root) { return None; }
        let moves = crate::file_tree::plan_move(root, directory, files, sources)?;
        if moves.is_empty() { return None; }
        self.file_tree_operation_in_progress = true;
        Some(moves)
    }
    pub fn prepare_import(&mut self, current_root: Option<&str>, root: &str, source: &str, directory: &str, files: &[String]) -> bool {
        self.clear_drag();
        current_root == Some(root) && crate::file_tree::valid_external_drop(root, source, directory, files)
    }
}
