use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::keybindings::{KeyAction, KeyBindingRegistry, KeyContext};

#[test]
fn tui_keybinding_contexts_resolve_reference_defaults_and_safe_overrides() {
    let mut registry = KeyBindingRegistry::default();
    let now = Instant::now();
    assert_eq!(
        registry.resolve(
            &[KeyContext::Global, KeyContext::Chat],
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            now,
        ),
        Some(KeyAction::Submit)
    );
    assert_eq!(
        registry.shortcut(KeyContext::Chat, KeyAction::Submit).as_deref(),
        Some("Enter")
    );
    assert!(registry
        .apply_override(KeyContext::Chat, KeyAction::Submit, "ctrl+c")
        .unwrap_err()
        .contains("reserved"));
    registry
        .apply_override(KeyContext::Chat, KeyAction::Submit, "ctrl+k ctrl+s")
        .unwrap();
    assert_eq!(
        registry.resolve(
            &[KeyContext::Global, KeyContext::Chat],
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            now + Duration::from_millis(1),
        ),
        None
    );
    assert_eq!(
        registry.resolve(
            &[KeyContext::Global, KeyContext::Chat],
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            now + Duration::from_millis(2),
        ),
        Some(KeyAction::Submit)
    );
    assert!(registry
        .apply_override(KeyContext::Chat, KeyAction::Submit, "ctrl+unknown")
        .unwrap_err()
        .contains("unknown key"));
}
