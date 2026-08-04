use ratatui::text::Line;

use super::TuiTheme;
use super::attachment::{AttachmentOpenIntent, render_attachment};
use crate::cli::tui_v2::model::{
    UiAttachment, UiAttachmentKind, UiMessageBase, UiMessageId,
};
use crate::cli::tui_v2::terminal_capabilities::{
    TerminalCapabilities, TuiBackground,
};

#[test]
fn tui_attachments_and_links_match_reference_and_sanitize_targets() {
    let hyperlinks = TerminalCapabilities::from_environment_values(
        false,
        Some("truecolor"),
        Some("xterm-256color"),
        Some("WezTerm"),
        TuiBackground::Dark,
    );
    assert!(hyperlinks.osc8_hyperlinks);
    let plain = TerminalCapabilities::from_environment_values(
        false,
        None,
        Some("xterm"),
        Some("Apple_Terminal"),
        TuiBackground::Dark,
    );
    assert!(!plain.osc8_hyperlinks);

    let image = attachment(
        UiAttachmentKind::Image,
        "diagram\u{1b}]8;;https://evil.test\u{7}.png",
        "file:///workspace/project/assets/diagram.png",
    )
    .with_media_type("image/png")
    .with_dimensions(1024, 768)
    .with_size_bytes(12_288)
    .with_source("paste");
    let rendered = render_attachment(
        &image,
        2,
        hyperlinks,
        Some("/workspace/project"),
        TuiTheme::default(),
        54,
    );
    let text = line_texts(&rendered.lines).join("\n");
    assert!(text.contains("[Image #2]"));
    assert!(text.contains("image/png · 1024×768 · 12 KB · paste"));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains('\u{7}'));
    assert!(matches!(
        rendered.open_intent,
        Some(AttachmentOpenIntent::LocalPath { ref path }) if path == "/workspace/project/assets/diagram.png"
    ));

    let pasted = attachment(UiAttachmentKind::PastedText, "Pasted text", "paste://3")
        .with_preview("first line\nsecond line\nsecret third line");
    let pasted = render_attachment(
        &pasted,
        3,
        plain,
        Some("/workspace/project"),
        TuiTheme::default(),
        40,
    );
    assert_eq!(
        line_texts(&pasted.lines),
        vec!["[Pasted text #3] first line", "  second line", "  (Ctrl+O to expand)"]
    );
    assert!(pasted.open_intent.is_none());

    let resource = attachment(
        UiAttachmentKind::Resource,
        "package docs",
        "https://docs.example.test/resource",
    );
    let resource = render_attachment(
        &resource,
        4,
        hyperlinks,
        Some("/workspace/project"),
        TuiTheme::default(),
        50,
    );
    assert!(line_texts(&resource.lines)[0].contains("\u{1b}]8;;https://docs.example.test/resource"));
    assert!(matches!(resource.open_intent, Some(AttachmentOpenIntent::Url { .. })));

    let dangerous = attachment(UiAttachmentKind::Link, "click", "javascript:alert(1)");
    let dangerous = render_attachment(
        &dangerous,
        5,
        hyperlinks,
        Some("/workspace/project"),
        TuiTheme::default(),
        40,
    );
    assert_eq!(line_texts(&dangerous.lines), vec!["[Link] click (unsafe target blocked)"]);
    assert!(dangerous.open_intent.is_none());
    assert!(!line_texts(&dangerous.lines).join("").contains('\u{1b}'));

    let outside = attachment(
        UiAttachmentKind::File,
        "secrets.txt",
        "file:///workspace/other/secrets.txt",
    );
    let outside = render_attachment(
        &outside,
        6,
        hyperlinks,
        Some("/workspace/project"),
        TuiTheme::default(),
        32,
    );
    assert!(outside.open_intent.is_none());
    assert!(line_texts(&outside.lines)[0].contains("outside workspace"));

    let long_path = attachment(
        UiAttachmentKind::File,
        "long.rs",
        "file:///workspace/project/src/components/really/long/path/to/long.rs",
    );
    let long_path = render_attachment(
        &long_path,
        7,
        plain,
        Some("/workspace/project"),
        TuiTheme::default(),
        36,
    );
    let long_text = line_texts(&long_path.lines)[0].clone();
    assert!(long_text.chars().count() <= 36);
    assert!(long_text.contains('…'));
    assert!(!long_text.contains('\u{1b}'));
}

fn attachment(kind: UiAttachmentKind, name: &str, uri: &str) -> UiAttachment {
    UiAttachment::new(
        UiMessageBase::new(UiMessageId::local("attachment")),
        kind,
        name,
        uri,
    )
}

fn line_texts(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            line.spans.iter().fold(String::new(), |mut out, span| {
                out.push_str(span.content.as_ref());
                out
            })
        })
        .collect()
}
