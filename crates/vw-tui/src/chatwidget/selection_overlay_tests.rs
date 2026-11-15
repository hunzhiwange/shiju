use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::*;
use crate::{V3Action, V3SelectionItem};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render(widget: &ChatWidget, width: u16) -> String {
    let height = widget.desired_height(width).max(1);
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn slash_input_opens_and_filters_command_palette() {
    let mut widget = ChatWidget::new(ChatWidgetConfig::default());
    widget.set_command_palette_items(vec![
        V3SelectionItem::new("help", "/help"),
        V3SelectionItem::new("status", "/status"),
        V3SelectionItem {
            id: "resume".to_string(),
            label: "/resume".to_string(),
            disabled: true,
            disabled_reason: Some("available after session routing is connected".to_string()),
            ..V3SelectionItem::default()
        },
    ]);

    assert_eq!(widget.handle_key_event(key(KeyCode::Char('/'))), V3Action::None);
    assert!(widget.command_palette_visible());
    let opened = render(&widget, 80);
    assert!(opened.contains("/help"));
    assert!(opened.contains("available after session routing is connected"));

    assert_eq!(widget.handle_key_event(key(KeyCode::Char('s'))), V3Action::None);
    assert_eq!(widget.handle_key_event(key(KeyCode::Char('t'))), V3Action::None);
    let filtered = render(&widget, 80);
    assert!(filtered.contains("/status"));
    assert!(!filtered.contains("/help"));
}

#[test]
fn generic_selection_routes_the_stable_item_id() {
    let mut widget = ChatWidget::new(ChatWidgetConfig::default());
    widget.show_selection_overlay(
        "sessions",
        "Resume session",
        vec![
            V3SelectionItem {
                id: "session-disabled".to_string(),
                label: "Archived session".to_string(),
                disabled: true,
                disabled_reason: Some("archived sessions cannot be resumed".to_string()),
                ..V3SelectionItem::default()
            },
            V3SelectionItem::new("session-stable-id", "Active session"),
        ],
    );

    let visible = render(&widget, 80);
    assert!(visible.contains("archived sessions cannot be resumed"));
    assert_eq!(
        widget.handle_key_event(key(KeyCode::Enter)),
        V3Action::Select {
            surface: "sessions".to_string(),
            item_id: "session-stable-id".to_string(),
        }
    );
    assert!(!widget.has_overlay());
}
