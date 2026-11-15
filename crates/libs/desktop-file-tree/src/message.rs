use iced::widget::text_editor;
#[derive(Debug, Clone, PartialEq, Eq)]
/// FileTreeAction 表示该流程中可枚举的状态或用户动作。
///
/// 变体与界面事件或后台任务结果保持对应，便于在消息分发时显式匹配。
pub enum FileTreeAction {
    Open,
    RevealInFinder,
    OpenInTerminal,
    AddToChat,
    FindInFolder,
    Cut,
    Copy,
    Duplicate,
    Paste,
    CopyPath,
    CopyRelativePath,
    Rename,
    Delete,
}

#[derive(Debug, Clone)]
pub enum ProjectMessage {
FileTreeRightClicked(String, String, f32, f32),
FileTreeMenuClose,
FileTreeAction(FileTreeAction),
FileTreeRenameChanged(String),
FileTreeRenameSave,
FileTreeFindScopeChanged(String, String),
FileTreeFindQueryEditorAction(String, text_editor::Action),
FileTreeFindReplaceEditorAction(String, text_editor::Action),
FileTreeFindCaseSensitiveToggled(String, bool),
FileTreeFindWholeWordToggled(String, bool),
FileTreeFindRegexToggled(String, bool),
FileTreeFindResultsPageChanged(String, usize),
FileTreeFindRun(String),
FileTreeFindInProject,
FileTreeFindMatchSelected {
        path: String,
        line: usize,
        column: usize,
    },
FileTreeFindTabSelected(String),
FileTreeFindTabClosed(String),
FileTreeFindTabRightClicked(String, f32, f32),
FileTreeFindTabMenuClose,
FileTreeFindTabMenuCloseLeft(String),
FileTreeFindTabMenuCloseRight(String),
FileTreeFindTabMenuCloseAll,
FileTreeDragStart(String, Option<(usize, usize)>),
FileTreeDragEnd,
}
#[derive(Debug, Clone)]
pub enum Message { None, Project(ProjectMessage) }
