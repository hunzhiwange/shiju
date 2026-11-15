//! 创建只读预览标签，不读取或写入工作区文件。
use super::PreviewTab;
use crate::components::editor::Editor;
use iced::{Font, widget};
pub fn open(
    tabs: &mut Vec<PreviewTab>,
    key: String,
    path: String,
    title: String,
    content: String,
    theme: iced::Theme,
    font_size: f32,
    line_height: f32,
    locale: config_ui::DesktopLocale,
) {
    if !tabs.iter().any(|tab| tab.path == key) {
        let extension = std::path::Path::new(&path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("plaintext");
        let mut editor = Editor::new(&content, extension);
        editor.set_font(Font::with_name("JetBrains Mono"));
        editor.set_theme(theme);
        editor.set_font_size(font_size);
        editor.set_line_height(line_height.clamp(10.0, 60.0));
        editor.set_ui_language(desktop_i18n::editor::language(locale));
        editor.set_search_replace_enabled(false);
        editor.configure_preview_context_menu(locale, false);
        tabs.push(PreviewTab {
            path: key.clone(),
            title: format!(
                "{title} · {}",
                desktop_i18n::I18n::new(locale).__("只读")
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
}
