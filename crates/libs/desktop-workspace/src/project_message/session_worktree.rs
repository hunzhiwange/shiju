//! Worktree picker policy; the desktop executes session, preference and gateway effects.
pub fn order_options(list: &mut Vec<(String, String)>, last_directory: Option<&String>) {
    if let Some(last_directory) = last_directory
        && let Some(pos) = list.iter().position(|(directory, _)| directory == last_directory)
    {
        let item = list.remove(pos);
        let insert_at = if !list.is_empty() && list[0].1 == "主工作区" { 1 } else { 0 };
        list.insert(insert_at, item);
    }
}
pub fn requested_name(value: &str) -> Result<String, &'static str> {
    let requested_name = value.trim().to_string();
    if requested_name.is_empty() { return Err("请输入 worktree 英文名称"); }
    if !requested_name.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
        return Err("worktree 名称仅支持小写英文、数字、连字符(-)");
    }
    Ok(requested_name)
}
pub fn is_primary(options: &[(String, String)], project: Option<&str>, directory: &str) -> bool {
    options.iter().any(|(d, label)| d == directory && label == "主工作区")
        || directory == project.unwrap_or_default()
}

/// A mutable projection of the existing picker storage, without copying its state.
pub struct Picker<'a> {
    pub project: &'a mut Option<String>,
    pub options: &'a mut Vec<(String, String)>,
    pub last_directory: &'a mut Option<String>,
    pub name: &'a mut String,
    pub creating: &'a mut bool,
    pub confirm_delete: &'a mut Option<String>,
    pub force_delete: &'a mut Option<String>,
    pub delete_error: &'a mut Option<String>,
    pub confirm_reset: &'a mut Option<String>,
    pub reset_error: &'a mut Option<String>,
}
#[derive(Clone)]
pub enum Event {
    Begin(String),
    Loaded { project_path: String, options: Result<Vec<(String, String)>, String> },
    Close,
    Name(String),
    Picked { project_path: String, directory: String },
    Create(String),
    Created { project_path: String, generation: u64, result: Result<String, String> },
    Delete(String),
    DeleteCancel,
    DeleteConfirmed,
    ForceDeleteConfirmed,
    Deleted { project_path: String, directory: String, result: Result<(), String> },
    Reset(String),
    ResetCancel,
    ResetConfirmed,
    ResetDone { project_path: String, result: Result<(), String> },
}
#[derive(Clone, Copy)]
pub enum ErrorField { General, Delete, Reset }
pub enum Effect {
    InputError(ErrorField, &'static str, Vec<String>),
    ClearError(ErrorField),
    ClearHover,
    OpenDraft { project_path: String, directory: String },
    Create { project_path: String, name: String, generation: u64 },
    CreateFailed(String),
    Delete { project_path: String, directory: String, force: bool },
    Reload { project_path: String, clear_last_directory: bool },
    Reset { project_path: String, directory: String },
    ResetNotification,
}
impl Picker<'_> {
    fn clear_messages(&mut self, effects: &mut Vec<Effect>) {
        super::session_common::clear_picker_messages(self.confirm_delete, self.force_delete,
            self.delete_error, self.confirm_reset, self.reset_error);
        effects.push(Effect::ClearError(ErrorField::Delete));
        effects.push(Effect::ClearError(ErrorField::Reset));
    }
    fn reset_selection(&mut self, effects: &mut Vec<Effect>) {
        super::session_common::reset_picker_selection(self.project, self.options, self.name);
        self.clear_messages(effects);
    }
}
/// Decide picker updates before the shell performs gateway/session/preference effects.
pub fn update(picker: &mut Picker<'_>, current_project: Option<&str>, generation: u64,
    event: Event) -> Vec<Effect> {
    let mut effects = Vec::new();
    match event {
        Event::Begin(project_path) => {
            *picker.project = Some(project_path);
            picker.options.clear();
            picker.name.clear();
            picker.clear_messages(&mut effects);
        }
        Event::Loaded { project_path, options } => {
            if picker.project.as_ref() != Some(&project_path) { return effects; }
            match options {
                Ok(mut list) => {
                    order_options(&mut list, picker.last_directory.as_ref());
                    *picker.options = list;
                }
                Err(error) => {
                    effects.push(Effect::InputError(ErrorField::General, "加载工作区列表失败: %s", vec![error]));
                    *picker.options = vec![("__create_worktree__".into(), "创建新的独立工作区".into())];
                }
            }
        }
        Event::Close => {
            effects.push(Effect::ClearHover);
            picker.reset_selection(&mut effects);
        }
        Event::Name(name) => *picker.name = name,
        Event::Picked { project_path, directory } => {
            if *picker.creating || current_project != Some(project_path.as_str()) { return effects; }
            picker.reset_selection(&mut effects);
            effects.push(Effect::OpenDraft { project_path, directory });
        }
        Event::Create(project_path) => {
            if *picker.creating { return effects; }
            let name = match requested_name(picker.name) {
                Ok(name) => name,
                Err(error) => {
                    effects.push(Effect::InputError(ErrorField::General, error, vec![]));
                    return effects;
                }
            };
            *picker.creating = true;
            picker.reset_selection(&mut effects);
            effects.push(Effect::Create { project_path, name, generation });
        }
        Event::Created { project_path, generation: response_generation, result } => {
            *picker.creating = false;
            if current_project != Some(project_path.as_str()) || generation != response_generation { return effects; }
            effects.push(match result {
                Ok(directory) => Effect::OpenDraft { project_path, directory },
                Err(error) => Effect::CreateFailed(error),
            });
        }
        Event::Delete(directory) => {
            if is_primary(picker.options, picker.project.as_deref(), &directory) {
                effects.push(Effect::InputError(ErrorField::Delete, "主工作区禁止删除", vec![]));
                *picker.confirm_delete = None;
                *picker.force_delete = None;
                *picker.confirm_reset = None;
                return effects;
            }
            *picker.confirm_delete = Some(directory);
            effects.push(Effect::ClearError(ErrorField::Delete));
            *picker.delete_error = None;
            *picker.force_delete = None;
            *picker.confirm_reset = None;
            effects.push(Effect::ClearError(ErrorField::Reset));
            *picker.reset_error = None;
        }
        Event::DeleteCancel => picker.clear_messages(&mut effects),
        Event::DeleteConfirmed => {
            let Some(project_path) = picker.project.clone() else { return effects; };
            let Some(directory) = picker.confirm_delete.clone() else { return effects; };
            if directory == project_path {
                effects.push(Effect::InputError(ErrorField::Delete, "主工作区禁止删除", vec![]));
                *picker.confirm_delete = None;
                *picker.force_delete = None;
                return effects;
            }
            picker.clear_messages(&mut effects);
            effects.push(Effect::Delete { project_path, directory, force: false });
        }
        Event::ForceDeleteConfirmed => {
            let Some(project_path) = picker.project.clone() else { return effects; };
            let Some(directory) = picker.force_delete.clone() else { return effects; };
            *picker.force_delete = None;
            effects.push(Effect::Delete { project_path, directory, force: true });
        }
        Event::Deleted { project_path, directory, result } => {
            if picker.project.as_ref() != Some(&project_path) { return effects; }
            match result {
                Ok(()) => {
                    effects.push(Effect::ClearError(ErrorField::Delete));
                    *picker.delete_error = None;
                    *picker.force_delete = None;
                    effects.push(Effect::ClearError(ErrorField::Reset));
                    *picker.reset_error = None;
                    let clear_last_directory = picker.last_directory.as_ref() == Some(&directory);
                    if clear_last_directory { *picker.last_directory = None; }
                    effects.push(Effect::Reload { project_path, clear_last_directory });
                }
                Err(error) => {
                    effects.push(Effect::ClearError(ErrorField::Delete));
                    *picker.delete_error = Some(error);
                    *picker.force_delete = Some(directory);
                }
            }
        }
        Event::Reset(directory) => {
            if is_primary(picker.options, picker.project.as_deref(), &directory) {
                effects.push(Effect::InputError(ErrorField::Reset, "主工作区禁止重置", vec![]));
                *picker.confirm_reset = None;
                *picker.confirm_delete = None;
                *picker.force_delete = None;
                return effects;
            }
            *picker.confirm_reset = Some(directory);
            effects.push(Effect::ClearError(ErrorField::Reset));
            *picker.reset_error = None;
            *picker.confirm_delete = None;
            *picker.force_delete = None;
            effects.push(Effect::ClearError(ErrorField::Delete));
            *picker.delete_error = None;
        }
        Event::ResetCancel => {
            *picker.confirm_reset = None;
            effects.push(Effect::ClearError(ErrorField::Reset));
            *picker.reset_error = None;
        }
        Event::ResetConfirmed => {
            let Some(project_path) = picker.project.clone() else { return effects; };
            let Some(directory) = picker.confirm_reset.clone() else { return effects; };
            if directory == project_path {
                effects.push(Effect::InputError(ErrorField::Reset, "主工作区禁止重置", vec![]));
                *picker.confirm_reset = None;
                return effects;
            }
            *picker.confirm_reset = None;
            effects.push(Effect::ClearError(ErrorField::Reset));
            *picker.reset_error = None;
            effects.push(Effect::Reset { project_path, directory });
        }
        Event::ResetDone { project_path, result } => {
            if picker.project.as_ref() != Some(&project_path) { return effects; }
            effects.push(Effect::ClearError(ErrorField::Reset));
            match result {
                Ok(()) => { *picker.reset_error = None; effects.push(Effect::ResetNotification); }
                Err(error) => *picker.reset_error = Some(error),
            }
        }
    }
    effects
}

#[cfg(test)]
#[path = "session_worktree_tests.rs"]
mod tests;
