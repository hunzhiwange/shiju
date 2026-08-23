use super::*;
use crate::vibe::action::{
    V3ApprovalChoice, V3ApprovalDecision, V3ApprovalKind, V3ApprovalRequest,
};

fn render_frame(widget: &ChatWidget, width: u16, height: u16) -> String {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    buffer
        .content
        .chunks(usize::from(width))
        .map(|row| {
            row.iter().map(ratatui::buffer::Cell::symbol).collect::<String>().trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sample_widget() -> ChatWidget {
    ChatWidget::new(ChatWidgetConfig {
        footer_data: V3FooterData {
            model: Some("vibewindow".into()),
            permission: Some("default".into()),
            directory: Some("/workspace".into()),
            ..V3FooterData::default()
        },
        placeholder: "Ask VibeWindow".into(),
    })
}

#[test]
fn v3_chatwidget_renders_without_codex_app_state() {
    let source = include_str!("../chatwidget.rs");
    let fields = source
        .split("pub struct ChatWidget {")
        .nth(1)
        .and_then(|tail| tail.split("}\n").next())
        .expect("ChatWidget field block");
    for expected in [
        "bottom_pane",
        "transcript",
        "input_queue",
        "status_state",
        "overlay",
        "last_rendered_width",
    ] {
        assert!(fields.contains(expected), "missing render field: {expected}");
    }
    for forbidden in [
        "app_event",
        "app_server",
        "codex_op",
        "model_catalog",
        "plugin",
        "login",
        "telemetry",
        "gateway",
    ] {
        assert!(!fields.contains(forbidden), "runtime field leaked: {forbidden}");
    }

    let mut widget = sample_widget();
    widget.push_history(V3DisplayEvent::User {
        text: "Summarize the renderer migration.".into(),
        attachments: Vec::new(),
    });
    widget.push_history(V3DisplayEvent::Assistant {
        markdown: "The copied **ChatWidget** now owns UI state only.".into(),
    });

    insta::assert_snapshot!("chatwidget_idle_layout", render_frame(&widget, 72, 14));
}

#[test]
fn chatwidget_running_layout() {
    let mut widget = sample_widget();
    widget.push_history(V3DisplayEvent::User {
        text: "Run the focused checks.".into(),
        attachments: Vec::new(),
    });
    widget.set_active_event(Some(V3DisplayEvent::Thinking {
        text: "Inspecting the copied layout".into(),
    }));
    widget
        .set_status(V3ChatStatus::running("Working", Some("Preparing renderer snapshots".into())));

    insta::assert_snapshot!("chatwidget_running_layout", render_frame(&widget, 72, 16));
}

#[test]
fn chatwidget_exec_and_status_layout() {
    let mut widget = sample_widget();
    widget.push_history(V3DisplayEvent::ToolCall {
        call_id: "tool-11".into(),
        tool: "exec".into(),
        command: "cargo test -p vw-tui-v3 chatwidget::rendering --lib".into(),
    });
    widget.set_live_tail_event(Some(V3DisplayEvent::Progress {
        label: "exec".into(),
        detail: Some("running focused snapshots".into()),
    }));
    widget.set_status(V3ChatStatus::running("Running command", Some("cargo test".into())));

    insta::assert_snapshot!("chatwidget_exec_and_status_layout", render_frame(&widget, 72, 17));
}

#[test]
fn chatwidget_approval_overlay_layout() {
    let mut widget = sample_widget();
    widget.push_history(V3DisplayEvent::Assistant {
        markdown: "The next action requires an explicit decision.".into(),
    });
    widget.show_approval(V3ApprovalRequest::new(
        "approval-11",
        V3ApprovalKind::Command,
        "Run command?",
        "$ cargo clippy --all-targets --all-features -- -D warnings",
        vec![
            V3ApprovalChoice::new("allow", "Allow", V3ApprovalDecision::Allow),
            V3ApprovalChoice::new("deny", "Deny", V3ApprovalDecision::Deny),
            V3ApprovalChoice::new("cancel", "Cancel", V3ApprovalDecision::Cancel),
        ],
    ));

    insta::assert_snapshot!("chatwidget_approval_overlay_layout", render_frame(&widget, 72, 20));
}
