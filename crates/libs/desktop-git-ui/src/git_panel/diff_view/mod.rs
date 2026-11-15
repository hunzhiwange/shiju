use iced::widget::text_editor;
#[derive(Debug, Clone)]
pub struct GitDiffLineRange {
    /// 文件路径
    pub file: String,
    /// 起始行号
    pub start: usize,
    /// 结束行号
    pub end: usize,
    /// 是否为旧版本
    pub is_old: bool,
}
#[derive(Debug, Clone)]
pub struct GitDiffContextMenuState {
    /// 文件路径
    pub file: String,
    /// 行号
    pub line: usize,
    /// 是否为旧版本
    pub is_old: bool,
    /// 菜单锚点 X 坐标
    pub x: f32,
    /// 菜单锚点 Y 坐标
    pub y: f32,
}
#[derive(Debug, Clone)]
pub struct GitDiffCommentDraft {
    /// 评论的行范围
    pub range: GitDiffLineRange,
    /// 评论编辑器内容
    pub editor: text_editor::Content,
}
#[derive(Debug, Clone)]
pub enum DiffMessage {
    DiffCommentEditorAction(text_editor::Action), DiffCommentCancel, DiffCommentSubmit,
    OpenDiffContextMenu { file: String, line: usize, is_old: bool, text: String, x: f32, y: f32 },
    OpenDiffCommentDraft, SelectDiffContextStageLines, ClearDiffContextStageLines,
    DiscardDiffSelection, CopyDiffSelection, InsertDiffSelectionToChat, CloseDiffContextMenu,
}
pub mod comment_editor;
pub mod context_menu;
#[cfg(test)]
mod comment_editor_tests;
#[cfg(test)]
mod context_menu_tests;
