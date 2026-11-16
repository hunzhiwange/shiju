//! Text preview policy and localized rejection messages.
use preview_content::file_looks_like_text;

/// 读取并格式化文件文本，保留桌面截断提示。
pub fn safe_preview_text(path: &str) -> String {
    let (content, truncated) = preview_content::read_preview_text(path);
    if truncated { format!("文件较大，预览已截断。\n\n{content}") } else { content }
}

fn unsupported_preview_kind(path: &str) -> Option<&'static str> {
    preview_content::unsupported_preview_kind(path).map(|kind| match kind {
        preview_content::UnsupportedPreviewKind::Archive => "压缩包",
        preview_content::UnsupportedPreviewKind::Pdf => "PDF",
    })
}

pub fn preview_open_error(path: &str) -> Option<String> {
    preview_open_error_message(path).map(|message| {
        message
            .render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
            .expect("preview error template")
    })
}

pub fn preview_open_error_message(path: &str) -> Option<desktop_i18n::format::Message> {
    use desktop_i18n::format::Message;
    if let Some(kind) = unsupported_preview_kind(path) {
        let kind =
            if kind == "PDF" { Message::literal(kind.into()) } else { Message::new(kind, vec![]) };
        return Some(Message::nested("不支持展开该文件：%s 文件无法打开".into(), vec![kind]));
    }
    if !file_looks_like_text(path) {
        return Some(Message::new("不支持展开该文件：二进制文件无法打开", vec![]));
    }
    None
}

#[cfg(test)]
#[path = "file_validation_tests.rs"]
mod tests;
