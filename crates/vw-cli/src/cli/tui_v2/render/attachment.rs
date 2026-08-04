//! 附件与安全链接渲染。

use std::path::{Component, Path, PathBuf};

use ratatui::text::Line;

use super::TuiTheme;
use crate::cli::tui_v2::model::{UiAttachment, UiAttachmentKind};
use crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AttachmentOpenIntent {
    LocalPath { path: String },
    Url { url: String },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct AttachmentRender {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) open_intent: Option<AttachmentOpenIntent>,
}

pub(super) fn render_attachment(
    attachment: &UiAttachment,
    image_index: usize,
    capabilities: TerminalCapabilities,
    workspace_root: Option<&str>,
    theme: TuiTheme,
    width: u16,
) -> AttachmentRender {
    let name = sanitize_visible_text(attachment.name.as_str());
    let target = safe_open_intent(attachment, workspace_root);
    let width = usize::from(width);

    let lines = match attachment.kind {
        UiAttachmentKind::Image => {
            let mut metadata = Vec::new();
            if let Some(media_type) = attachment.media_type.as_deref() {
                metadata.push(sanitize_visible_text(media_type));
            }
            if let (Some(width), Some(height)) = (attachment.width_px, attachment.height_px) {
                metadata.push(format!("{width}×{height}"));
            }
            if let Some(size) = attachment.size_bytes {
                metadata.push(format_size(size));
            }
            if let Some(source) = attachment.source.as_deref() {
                metadata.push(sanitize_visible_text(source));
            }
            let summary = if metadata.is_empty() {
                name
            } else {
                metadata.join(" · ")
            };
            vec![Line::styled(
                truncate_end(format!("[Image #{image_index}] {summary}").as_str(), width),
                theme.body_style(),
            )]
        }
        UiAttachmentKind::PastedText => {
            let preview = attachment
                .preview
                .as_deref()
                .unwrap_or_default()
                .lines()
                .map(sanitize_visible_text)
                .collect::<Vec<_>>();
            let first = preview.first().cloned().unwrap_or_default();
            let second = preview.get(1).cloned();
            let mut lines = vec![Line::styled(
                truncate_end(format!("[Pasted text #{image_index}] {first}").as_str(), width),
                theme.body_style(),
            )];
            if let Some(second) = second {
                lines.push(Line::styled(
                    truncate_end(format!("  {second}").as_str(), width),
                    theme.muted_style(),
                ));
            }
            if preview.len() > 2 {
                lines.push(Line::styled("  (Ctrl+O to expand)", theme.muted_style()));
            }
            lines
        }
        UiAttachmentKind::Resource | UiAttachmentKind::Link => {
            let label = if attachment.kind == UiAttachmentKind::Resource {
                format!("[Resource] {name}")
            } else {
                format!("[Link] {name}")
            };
            let rendered = match target.as_ref() {
                Some(AttachmentOpenIntent::Url { url }) if capabilities.osc8_hyperlinks => {
                    osc8_link(url, label.as_str())
                }
                Some(_) => label,
                None => format!("{label} (unsafe target blocked)"),
            };
            vec![Line::styled(truncate_end(rendered.as_str(), width.max(rendered.len())), theme.body_style())]
        }
        UiAttachmentKind::File => {
            let (label, allowed) = match target.as_ref() {
                Some(AttachmentOpenIntent::LocalPath { path }) => {
                    let display_path = compact_path(path, workspace_root);
                    (format!("[File] {name} · {display_path}"), true)
                }
                _ => ("[File] outside workspace (blocked)".to_string(), false),
            };
            let label = if allowed {
                truncate_middle(label.as_str(), width)
            } else {
                truncate_end(label.as_str(), width)
            };
            let rendered = if allowed && capabilities.osc8_hyperlinks {
                let AttachmentOpenIntent::LocalPath { path } =
                    target.as_ref().expect("allowed local target")
                else {
                    unreachable!("allowed file target must be local");
                };
                osc8_link(file_url(path).as_str(), label.as_str())
            } else {
                label
            };
            vec![Line::styled(rendered, theme.body_style())]
        }
    };

    AttachmentRender { lines, open_intent: target }
}

fn safe_open_intent(
    attachment: &UiAttachment,
    workspace_root: Option<&str>,
) -> Option<AttachmentOpenIntent> {
    let uri = attachment.uri.trim();
    if uri.chars().any(char::is_control) {
        return None;
    }
    match attachment.kind {
        UiAttachmentKind::Resource | UiAttachmentKind::Link => {
            let lower = uri.to_ascii_lowercase();
            (lower.starts_with("https://") || lower.starts_with("http://"))
                .then(|| AttachmentOpenIntent::Url { url: uri.to_string() })
        }
        UiAttachmentKind::Image | UiAttachmentKind::File => {
            let path = uri.strip_prefix("file://").unwrap_or(uri);
            safe_workspace_path(path, workspace_root)
                .map(|path| AttachmentOpenIntent::LocalPath { path })
        }
        UiAttachmentKind::PastedText => None,
    }
}

fn safe_workspace_path(path: &str, workspace_root: Option<&str>) -> Option<String> {
    let root = Path::new(workspace_root?);
    let candidate = Path::new(path);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    if joined.components().any(|component| matches!(component, Component::ParentDir)) {
        return None;
    }
    let normalized_root = lexical_normalize(root);
    let normalized = lexical_normalize(&joined);
    normalized
        .starts_with(&normalized_root)
        .then(|| normalized.to_string_lossy().into_owned())
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn compact_path(path: &str, workspace_root: Option<&str>) -> String {
    workspace_root
        .and_then(|root| Path::new(path).strip_prefix(root).ok())
        .map(|path| path.to_string_lossy().into_owned())
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| path.to_string())
}

fn sanitize_visible_text(value: &str) -> String {
    value.chars().filter(|character| !character.is_control()).collect()
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_024 {
        format!("{} KB", bytes.saturating_add(512) / 1_024)
    } else {
        format!("{bytes} B")
    }
}

fn osc8_link(target: &str, label: &str) -> String {
    format!("\u{1b}]8;;{target}\u{1b}\\{label}\u{1b}]8;;\u{1b}\\")
}

fn file_url(path: &str) -> String {
    let mut encoded = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(format!("%{byte:02X}").as_str());
        }
    }
    encoded
}

fn truncate_end(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut output = value.chars().take(width.saturating_sub(1)).collect::<String>();
    output.push('…');
    output
}

fn truncate_middle(value: &str, width: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return value.to_string();
    }
    if width < 3 {
        return "…".chars().take(width).collect();
    }
    let left = (width.saturating_sub(1)) / 2;
    let right = width.saturating_sub(1).saturating_sub(left);
    format!(
        "{}…{}",
        chars[..left].iter().collect::<String>(),
        chars[chars.len().saturating_sub(right)..].iter().collect::<String>()
    )
}
