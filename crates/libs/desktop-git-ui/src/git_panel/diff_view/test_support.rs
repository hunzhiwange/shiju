use super::{DiffRenderCtx, DiffViewState, GitDiffContextMenuState, GitDiffLineRange, GitDiffCommentDraft};
pub(crate) struct Fixture {
    pub(crate) merge_view: bool,
    pub(crate) show_git_diff_highlight: bool,
    pub(crate) git_diff_drag_range: Option<GitDiffLineRange>,
    pub(crate) git_diff_selected_range: Option<GitDiffLineRange>,
    pub(crate) git_diff_comment_draft: Option<GitDiffCommentDraft>,
    pub(crate) staged_lines_selected: Vec<(String, usize)>,
    pub(crate) staged_old_lines_selected: Vec<(String, usize)>,
    pub(crate) staged_files_selected: Vec<String>,
    pub(crate) git_diff_selected_lines: Vec<GitDiffSelectedLine>,
    pub(crate) git_diff_hovered_line: Option<(String, usize, bool)>,
    pub(crate) git_diff_context_menu: Option<GitDiffContextMenuState>,
    pub(crate) expanded_hunks: Vec<(String, usize)>,
    pub(crate) context_expansions: std::collections::HashMap<(String, usize), (usize, usize)>,
}
pub(crate) struct GitDiffSelectedLine { pub file: String, pub line: usize, pub is_old: bool, pub text: String }
impl Fixture {
    pub(crate) fn render_ctx(&self) -> DiffRenderCtx<'_> {
        DiffRenderCtx {
            read_only: false, history_file: None, history_notice: None,
            selected_new_lines: self.staged_lines_selected.iter().map(|(file,line)| (file.as_str(), *line)).collect(),
            selected_old_lines: self.staged_old_lines_selected.iter().map(|(file,line)| (file.as_str(), *line)).collect(),
            selected_diff_lines: self.git_diff_selected_lines.iter().map(|line| (line.file.as_str(), line.line, line.is_old)).collect(),
            selected_files: self.staged_files_selected.iter().map(String::as_str).collect(),
        }
    }
    pub(crate) fn view_state(&self) -> DiffViewState<'_> {
        DiffViewState {
            merge_view: self.merge_view, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            expanded_hunks: &self.expanded_hunks, context_expansions: &self.context_expansions,
            active_range: self.git_diff_drag_range.as_ref().or(self.git_diff_selected_range.as_ref()).or_else(|| self.git_diff_comment_draft.as_ref().map(|draft| &draft.range)), hovered_line: self.git_diff_hovered_line.as_ref(), context_menu: self.git_diff_context_menu.as_ref(),
        }
    }
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            merge_view: true, show_git_diff_highlight: true,
            git_diff_drag_range: None, git_diff_selected_range: None, git_diff_comment_draft: None,
            staged_lines_selected: Vec::new(), staged_old_lines_selected: Vec::new(), staged_files_selected: Vec::new(),
            git_diff_selected_lines: Vec::new(), git_diff_hovered_line: None, git_diff_context_menu: None,
            expanded_hunks: Vec::new(), context_expansions: Default::default(),
        }
    }
}
