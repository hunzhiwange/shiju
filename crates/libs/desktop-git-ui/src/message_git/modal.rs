use desktop_workspace::workspace_state::ChatTextDiff;
use iced::widget::text_editor;

pub fn open_custom_diff_modal(
    show_git_custom_diff_modal: &mut bool,
    git_custom_diff_hide_inputs: &mut bool,
    git_custom_diff_title: &mut String,
) {
    (*show_git_custom_diff_modal) = true;
    (*git_custom_diff_hide_inputs) = false;
    if (*git_custom_diff_title).trim().is_empty() {
        (*git_custom_diff_title) = "自定义对比".to_string();
    }
}

pub fn close_custom_diff_modal(show_git_custom_diff_modal: &mut bool, git_custom_diff_hide_inputs: &mut bool) {
    (*show_git_custom_diff_modal) = false;
    (*git_custom_diff_hide_inputs) = false;
}

pub fn custom_diff_title_changed(git_custom_diff_title: &mut String, title: String) {
    (*git_custom_diff_title) = title;
}

pub fn custom_diff_before_editor_action(
    git_custom_diff_before_editor: &mut text_editor::Content,
    action: text_editor::Action,
) {
    (*git_custom_diff_before_editor).perform(action);
}

pub fn custom_diff_after_editor_action(
    git_custom_diff_after_editor: &mut text_editor::Content,
    action: text_editor::Action,
) {
    (*git_custom_diff_after_editor).perform(action);
}

pub fn custom_diff_swap(
    git_custom_diff_before_editor: &mut text_editor::Content,
    git_custom_diff_after_editor: &mut text_editor::Content,
) {
    std::mem::swap(
        &mut (*git_custom_diff_before_editor),
        &mut (*git_custom_diff_after_editor),
    );
}

pub fn close_chat_text_diff(
    chat_text_diff: &mut Option<ChatTextDiff>,
    show_git_custom_diff_modal: &mut bool,
    git_custom_diff_hide_inputs: &mut bool,
) {
    (*chat_text_diff) = None;
    (*show_git_custom_diff_modal) = false;
    (*git_custom_diff_hide_inputs) = false;
}

pub fn close_copy_modal(show_git_copy_modal: &mut bool) {
    (*show_git_copy_modal) = false;
}

pub fn copy_modal_editor_action(git_copy_modal_editor: &mut text_editor::Content, action: text_editor::Action) {
    (*git_copy_modal_editor).perform(action);
}

/// 打开复制弹窗；主题与字体参数由宿主投影。
pub fn open_copy_modal_with_text(
    show: &mut bool, use_color: &mut bool,
    editor: &mut iced::widget::text_editor::Content,
    code_editor: &mut iced_code_editor::CodeEditor,
    text: String, theme: iced::Theme, font_size: f32, line_height: f32,
    language: iced_code_editor::i18n::Language,
) {
    *show = true;
    *use_color = !super::shared::text_too_large_for_code_editor(&text);
    *editor = iced::widget::text_editor::Content::with_text(&text);
    if *use_color {
        *code_editor = iced_code_editor::CodeEditor::new(&text, "diff");
        super::shared::configure_git_code_editor(theme, font_size.clamp(10.0, 30.0), line_height.clamp(10.0, 60.0), language, code_editor);
    }
}

pub fn toggle_copy_modal_colored(
    use_color: &mut bool,
    editor: &mut iced::widget::text_editor::Content,
    code_editor: &mut iced_code_editor::CodeEditor,
    enabled: bool, theme: iced::Theme, font_size: f32, line_height: f32,
    language: iced_code_editor::i18n::Language,
) {
    *use_color = enabled;
    if enabled {
        let text = editor.text();
        if super::shared::text_too_large_for_code_editor(&text) {
            *use_color = false;
            return;
        }
        *code_editor = iced_code_editor::CodeEditor::new(&text, "diff");
        super::shared::configure_git_code_editor(theme, font_size.clamp(10.0, 30.0), line_height.clamp(10.0, 60.0), language, code_editor);
    } else {
        *editor = iced::widget::text_editor::Content::with_text(&code_editor.content());
    }
}

pub fn diff_patch(project_path: Option<&str>, file: String) -> String {
            #[cfg(target_arch = "wasm32")]
            let _ = &file;
            #[cfg(not(target_arch = "wasm32"))]
            let mut patch = String::new();
            #[cfg(target_arch = "wasm32")]
            let patch = String::new();

            if let Some(_path) = project_path.map(str::to_owned) {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    patch = (|| {
                        let repo = git2::Repository::open(&_path).ok()?;
                        let head = repo.head().ok()?;
                        let tree = head.peel_to_tree().ok()?;
                        let mut opts = git2::DiffOptions::new();
                        opts.include_untracked(true);
                        opts.recurse_untracked_dirs(true);
                        opts.pathspec(file.clone());
                        let diff = repo
                            .diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))
                            .ok()?;
                        let mut out = String::new();
                        diff.print(git2::DiffFormat::Patch, |_d, _h, line| {
                            out.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
                            true
                        })
                        .ok()?;
                        Some(out)
                    })()
                    .unwrap_or_default();
                }
            }

    patch
}

pub fn current_copy_content(
    use_color: bool,
    editor: &text_editor::Content,
    code_editor: &iced_code_editor::CodeEditor,
) -> String {
    if use_color { code_editor.content() } else { editor.text() }
}
pub fn open_custom_diff_result(show: &mut bool, hide_inputs: &mut bool, current_title: &mut String,
    before_editor: &mut text_editor::Content, after_editor: &mut text_editor::Content,
    title: String, before: String, after: String) {
    *show = true;
    *hide_inputs = true;
    *current_title = title;
    *before_editor = text_editor::Content::with_text(&before);
    *after_editor = text_editor::Content::with_text(&after);
}

#[cfg(test)]
#[path = "modal_tests.rs"]
mod tests;
