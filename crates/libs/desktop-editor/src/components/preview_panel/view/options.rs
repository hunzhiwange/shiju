/// 当前文件外围面板；单一状态保证操作、设置与帮助互斥。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorPopover {
    Actions,
    Settings,
    Help,
}

/// 当前实例的公开布尔选项，不持久化到全局配置。
#[derive(Debug, Clone, Copy)]
pub enum EditorOption {
    LineNumbers,
    Wrap,
    Whitespace,
    IndentGuides,
    ColorPreviews,
    StickyScroll,
    BracketMatch,
    BracketColors,
    Folding,
    AutoIndent,
    AutoBrackets,
    Vim,
}
impl EditorOption {
    pub const ALL: [Self; 12] = [
        Self::LineNumbers,
        Self::Wrap,
        Self::Whitespace,
        Self::IndentGuides,
        Self::ColorPreviews,
        Self::StickyScroll,
        Self::BracketMatch,
        Self::BracketColors,
        Self::Folding,
        Self::AutoIndent,
        Self::AutoBrackets,
        Self::Vim,
    ];
    pub fn enabled(self, editor: &iced_code_editor::CodeEditor) -> bool {
        match self {
            Self::LineNumbers => editor.line_numbers_enabled(),
            Self::Wrap => editor.wrap_enabled(),
            Self::Whitespace => editor.show_whitespace(),
            Self::IndentGuides => editor.show_indent_guides(),
            Self::ColorPreviews => editor.show_color_previews(),
            Self::StickyScroll => editor.sticky_scroll_enabled(),
            Self::BracketMatch => editor.bracket_match_highlight_enabled(),
            Self::BracketColors => editor.bracket_pair_colorization_enabled(),
            Self::Folding => editor.folding_enabled(),
            Self::AutoIndent => editor.auto_indent_enabled(),
            Self::AutoBrackets => editor.auto_close_brackets(),
            Self::Vim => editor.vim_enabled(),
        }
    }
    pub fn set(self, editor: &mut iced_code_editor::CodeEditor, enabled: bool) {
        match self {
            Self::LineNumbers => editor.set_line_numbers_enabled(enabled),
            Self::Wrap => editor.set_wrap_enabled(enabled),
            Self::Whitespace => editor.set_show_whitespace(enabled),
            Self::IndentGuides => editor.set_show_indent_guides(enabled),
            Self::ColorPreviews => editor.set_show_color_previews(enabled),
            Self::StickyScroll => editor.set_sticky_scroll_enabled(enabled),
            Self::BracketMatch => editor.set_bracket_match_highlight_enabled(enabled),
            Self::BracketColors => editor.set_bracket_pair_colorization_enabled(enabled),
            Self::Folding => editor.set_folding_enabled(enabled),
            Self::AutoIndent => editor.set_auto_indent_enabled(enabled),
            Self::AutoBrackets => editor.set_auto_close_brackets(enabled),
            Self::Vim => editor.set_vim_enabled(enabled),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::LineNumbers => "行号",
            Self::Wrap => "自动换行",
            Self::Whitespace => "空白",
            Self::IndentGuides => "缩进线",
            Self::ColorPreviews => "颜色预览",
            Self::StickyScroll => "粘性滚动",
            Self::BracketMatch => "括号匹配",
            Self::BracketColors => "彩色括号",
            Self::Folding => "代码折叠",
            Self::AutoIndent => "自动缩进",
            Self::AutoBrackets => "自动括号",
            Self::Vim => "Vim",
        }
    }
}
