//! Git 消息入口的局部输入与宿主效果。
use super::messages::GitMessage;
use crate::state::State;
use iced::Task;
pub mod diff;
pub mod modal;
pub mod refresh;
pub struct Context {
    pub project_path: Option<String>,
    pub repo_path: Option<String>,
    pub context_path: Option<String>,
    pub editor_theme: iced::Theme,
    pub font_size: f32,
    pub line_height: f32,
    pub language: iced_code_editor::i18n::Language,
    pub manual_refresh: bool,
    pub animate: bool,
}
#[derive(Debug, Clone)]
pub enum Effect {
    PersistHighlight(bool),
    DismissPreview,
    Preview(String),
    CopyFile(String),
    CopyCode(String),
    AppendChat(String),
    ShowDiff { changes: bool },
    ClearPreview,
    Fullscreen(bool),
    Discard(super::diff::DiscardAction),
    LoadWorktrees(String),
    LoadChangedFiles(String),
    LoadDiffMetas(String),
    LoadDiff { repo_path: String, file: String, meta: crate::git_panel::ops::DiffFileMeta },
    InputError { message: &'static str, error: String },
    ChangedFilesApplied,
    ClearManualRefresh,
    Commit(super::stage_commit::Effect),
}
pub fn refresh_task() -> Task<GitMessage> {
    Task::batch(
        [
            GitMessage::RefreshRepository,
            GitMessage::RefreshWorktreeOptions,
            GitMessage::RefreshChangedFiles,
            GitMessage::RefreshDiffFileMetas,
        ]
        .map(Task::done),
    )
}
pub fn load_missing(
    app: &mut State,
    repo_path: Option<String>,
    effects: &mut Vec<Effect>,
    files: impl IntoIterator<Item = String>,
) -> Task<GitMessage> {
    let mut tasks = Vec::new();
    for file in files {
        match prepare_load(app, repo_path.clone(), &file) {
            super::refresh::DiffContentLoad::None => {}
            super::refresh::DiffContentLoad::RefreshMetas => {
                tasks.push(Task::done(GitMessage::RefreshDiffFileMetas))
            }
            super::refresh::DiffContentLoad::Load { repo_path, meta } => {
                effects.push(Effect::LoadDiff { repo_path, meta, file })
            }
        }
    }
    Task::batch(tasks)
}
pub fn dismiss(app: &mut State, effects: &mut Vec<Effect>) {
    app.git_diff_file_menu = None;
    effects.push(Effect::DismissPreview);
}

impl Default for Context {
    fn default() -> Self {
        Self {
            project_path: None,
            repo_path: None,
            context_path: None,
            editor_theme: iced::Theme::Dark,
            font_size: 16.0,
            line_height: 1.5,
            language: iced_code_editor::i18n::Language::ChineseSimplified,
            manual_refresh: false,
            animate: false,
        }
    }
}

pub fn update(
    app: &mut State,
    ctx: &Context,
    effects: &mut Vec<Effect>,
    message: GitMessage,
) -> Task<GitMessage> {
    match message {
        message @ (GitMessage::RefreshGitPanelData
        | GitMessage::RefreshWorktreeOptions
        | GitMessage::WorktreeOptionsReady { .. }
        | GitMessage::SelectGitWorktree(_)
        | GitMessage::ToggleGitWorktreeMenu(_)
        | GitMessage::RefreshChangedFiles
        | GitMessage::ChangedFilesReady { .. }
        | GitMessage::RefreshDiffFileMetas
        | GitMessage::DiffFileMetasReady { .. }
        | GitMessage::LoadDiffContent(_)
        | GitMessage::DiffContentReady { .. }) => refresh::update(app, ctx, effects, message),
        message @ (GitMessage::ToggleDiffHighlight(_)
        | GitMessage::ToggleFullscreen
        | GitMessage::ToggleHalfFullscreen
        | GitMessage::ToggleDiffLineSelection(_, _, _, _)
        | GitMessage::DiffDragSelectStart(_, _, _, _)
        | GitMessage::DiffDragSelectHover(_, _, _)
        | GitMessage::DiffDragSelectEnd
        | GitMessage::OpenDiffContextMenu { .. }
        | GitMessage::CloseDiffContextMenu
        | GitMessage::OpenDiffFileMenu(_)
        | GitMessage::CloseDiffFileMenu
        | GitMessage::PreviewDiffFile(_)
        | GitMessage::CopyDiffFile { .. }
        | GitMessage::RevertDiffFile(_)
        | GitMessage::OpenDiffCommentDraft
        | GitMessage::SelectDiffContextStageLines
        | GitMessage::ClearDiffContextStageLines
        | GitMessage::DiscardDiffSelection
        | GitMessage::InsertDiffSelectionToChat
        | GitMessage::DiffHoverEnter(_, _, _)
        | GitMessage::DiffHoverExit(_, _, _)
        | GitMessage::DiffCommentEditorAction(_)
        | GitMessage::DiffCommentCancel
        | GitMessage::DiffCommentSubmit
        | GitMessage::CopyDiffSelection
        | GitMessage::InsertDiffSelectionComment
        | GitMessage::ConfirmDiscardFile(_)
        | GitMessage::CancelDiscardFile
        | GitMessage::DiscardFile(_)
        | GitMessage::DiscardHunk(_, _)
        | GitMessage::RevertLineDelete(_, _)
        | GitMessage::RevertLineRestore(_, _, _)
        | GitMessage::ToggleExpandHunk(_, _)
        | GitMessage::ToggleExpandFile(_)
        | GitMessage::FocusFile(_)
        | GitMessage::DiffScrollChanged { .. }
        | GitMessage::DiffThemeSelected(_)
        | GitMessage::ExpandContext(_, _, _)) => diff::update(app, ctx, effects, message),
        message @ (GitMessage::OpenDiffCopyMode(_)
        | GitMessage::OpenCopyModalWithText(_)
        | GitMessage::OpenCopyModalFromPath(_)
        | GitMessage::CloseCopyModal
        | GitMessage::ToggleCopyModalColored(_)
        | GitMessage::CopyModalEditorAction(_)
        | GitMessage::CopyModalCodeEditorEvent(_)
        | GitMessage::CopyModalCopyCurrent
        | GitMessage::InsertCopyModalToChatCurrent
        | GitMessage::InsertCopyModalToChat(_)
        | GitMessage::OpenCustomDiffModal
        | GitMessage::CloseCustomDiffModal
        | GitMessage::CustomDiffTitleChanged(_)
        | GitMessage::CustomDiffBeforeEditorAction(_)
        | GitMessage::CustomDiffAfterEditorAction(_)
        | GitMessage::CustomDiffSwap
        | GitMessage::OpenCustomDiffResult { .. }
        | GitMessage::OpenChatTextDiff { .. }
        | GitMessage::CloseChatTextDiff) => modal::update(app, ctx, effects, message),
        message @ (GitMessage::CommitMessageChanged(_)
        | GitMessage::CommitTypeSelected(_)
        | GitMessage::CommitScopeChanged(_)
        | GitMessage::CommitDescriptionChanged(_)
        | GitMessage::CommitDescriptionEditorAction(_)
        | GitMessage::CommitHelpOpen
        | GitMessage::CommitHelpClose
        | GitMessage::FilterHelpOpen
        | GitMessage::FilterHelpClose
        | GitMessage::ToggleFilterOptions(_)
        | GitMessage::FilterQueryChanged(_)
        | GitMessage::FilterToggleIncluded(_)
        | GitMessage::FilterToggleExcluded(_)
        | GitMessage::FilterToggleNew(_)
        | GitMessage::FilterToggleModified(_)
        | GitMessage::FilterToggleDeleted(_)
        | GitMessage::ClearFilters
        | GitMessage::ToggleStageFile(_, _)
        | GitMessage::ToggleStageHunk(_, _, _)
        | GitMessage::ToggleStageLine(_, _, _)
        | GitMessage::ToggleStageOldLine(_, _, _)
        | GitMessage::SelectAllFileLines(_)
        | GitMessage::SelectAllVisibleFileLines(_)
        | GitMessage::InvertVisibleFileLines(_)
        | GitMessage::ClearAllFileLines(_)
        | GitMessage::HoverFileHeaderEnter(_)
        | GitMessage::HoverFileHeaderExit(_)
        | GitMessage::HoverGitPanelHeaderEnter
        | GitMessage::HoverGitPanelHeaderExit
        | GitMessage::CommitSelected
        | GitMessage::StageAll
        | GitMessage::StageAllFinished(_)
        | GitMessage::CommitAll
        | GitMessage::CommitSelectedFinished(_)) => update_stage(app, ctx, effects, message),
        _ => unreachable!("repository messages are dispatched to the repository controller"),
    }
}

pub fn update_stage(
    app: &mut State,
    ctx: &Context,
    effects: &mut Vec<Effect>,
    message: GitMessage,
) -> Task<GitMessage> {
    use crate::message_git::stage_commit;
    let result = stage_commit::update(
        &mut stage_commit::UpdateInput {
            context_expansions: &mut app.context_expansions,
            expanded_files: &mut app.expanded_files,
            expanded_files_set: &mut app.expanded_files_set,
            expanded_hunks: &mut app.expanded_hunks,
            git_commit_description: &mut app.git_commit_description,
            git_commit_description_editor: &mut app.git_commit_description_editor,
            git_commit_in_progress: &mut app.git_commit_in_progress,
            git_commit_message: &mut app.git_commit_message,
            git_commit_scope: &mut app.git_commit_scope,
            git_commit_type: &mut app.git_commit_type,
            git_filter_deleted: &mut app.git_filter_deleted,
            git_filter_excluded: &mut app.git_filter_excluded,
            git_filter_included: &mut app.git_filter_included,
            git_filter_modified: &mut app.git_filter_modified,
            git_filter_new: &mut app.git_filter_new,
            git_filter_query: &mut app.git_filter_query,
            git_hovered_file_header: &mut app.git_hovered_file_header,
            git_panel_header_hovered: &mut app.git_panel_header_hovered,
            show_git_commit_help_modal: &mut app.show_git_commit_help_modal,
            show_git_filter_help_modal: &mut app.show_git_filter_help_modal,
            show_git_filter_options: &mut app.show_git_filter_options,
            staged_files_selected: &mut app.staged_files_selected,
            staged_hunks_selected: &mut app.staged_hunks_selected,
            staged_lines_selected: &mut app.staged_lines_selected,
            staged_old_lines_selected: &mut app.staged_old_lines_selected,
            remote_busy: app.git_repository.remote_busy,
            context_path: ctx.context_path.clone(),
            repo_path: ctx.repo_path.clone(),
            animate: ctx.animate,
        },
        message,
    );
    effects.extend(result.into_iter().map(Effect::Commit));
    Task::none()
}

pub fn after_mutation(app: &mut State) -> Task<GitMessage> {
    super::refresh::reset_loading_after_mutation(
        &mut app.git_changed_files_loading,
        &mut app.git_changed_files_repo_path,
        &mut app.git_diff_file_metas_loading,
        &mut app.git_diff_file_metas_repo_path,
        &mut app.git_diff_contents_loading,
    );
    refresh_task()
}
pub fn prepare_load(
    app: &mut State,
    path: Option<String>,
    file: &str,
) -> super::refresh::DiffContentLoad {
    super::refresh::prepare_diff_content(
        &app.git_diff_contents,
        &mut app.git_diff_contents_loading,
        app.git_diff_file_metas_repo_path.as_deref(),
        &app.git_diff_file_metas,
        path,
        file,
    )
}
