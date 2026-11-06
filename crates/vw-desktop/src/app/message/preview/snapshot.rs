//! 复用普通预览编辑器打开只读内容，标签身份与工作区文件分离。
use super::{PreviewMessage, tab};
use crate::app::{App, Message, PreviewTab, components::editor::Editor};
use iced::{Font, Task, widget};

pub(super) fn open(
    app: &mut App,
    key: String,
    path: String,
    title: String,
    content: String,
) -> Task<Message> {
    if !app.preview_tabs.iter().any(|tab| tab.path == key) {
        let extension = std::path::Path::new(&path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("plaintext");
        let mut editor = Editor::new(&content, extension);
        editor.set_font(Font::with_name("JetBrains Mono"));
        editor.set_theme(app.effective_editor_theme());
        editor.set_font_size(app.current_font_size);
        editor.set_line_height(app.current_line_height.clamp(10.0, 60.0));
        editor.set_ui_language(crate::app::i18n::editor::language(app.desktop_locale));
        editor.set_search_replace_enabled(false);
        editor.configure_preview_context_menu(app.desktop_locale, false);
        app.preview_tabs.push(PreviewTab {
            path: key.clone(),
            title: format!(
                "{title} · {}",
                crate::app::i18n::I18n::new(app.desktop_locale).__("只读")
            ),
            content,
            is_dirty: false,
            truncated: false,
            read_only: true,
            auto_save_revision: 0,
            editor,
            scroll_id: widget::Id::unique(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_server_key: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_uri: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_language_id: None,
        });
    }
    tab::update(app, PreviewMessage::Open(key))
}
