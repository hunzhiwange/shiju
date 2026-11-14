//! TUI 的按键绑定原语、输入匹配和快捷键提示渲染。
//!
//! `KeyBinding` 处理终端对大写字母和原始 C0 控制字符上报不一致的问题；列表和选择器
//! 应通过这些辅助函数匹配导航，`is_plain_text_key_event` 则统一区分可搜索文本输入与命令。
//! Key binding primitives and input matching for the TUI.
//!
//! This module provides `KeyBinding`, the runtime representation of a single
//! keybinding (key code + modifier set), along with matching logic that handles
//! cross-terminal inconsistencies in how shifted letters and raw C0 control
//! characters are reported.
//!
//! List and picker code should match navigation through these helpers instead
//! of comparing `KeyEvent` values directly. The matcher owns compatibility for
//! terminals that report control chords as C0 characters, while
//! `is_plain_text_key_event` gives searchable pickers a shared boundary between
//! text input and navigation commands.
//!
//! It also supplies rendering helpers that convert bindings into styled
//! `ratatui::text::Span` values for UI hint display.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::style::Style;
use ratatui::text::Span;

#[cfg(test)]
const ALT_PREFIX: &str = "⌥ + ";
#[cfg(all(not(test), target_os = "macos"))]
const ALT_PREFIX: &str = "⌥ + ";
#[cfg(all(not(test), not(target_os = "macos")))]
const ALT_PREFIX: &str = "alt + ";
const CTRL_PREFIX: &str = "ctrl + ";
const SHIFT_PREFIX: &str = "shift + ";

/// 可触发 TUI 动作的一个具体按键；匹配兼容无 SHIFT 的大写字母和原始 C0 Ctrl 字符，
/// 但不负责多键和弦或部分匹配，这类状态由调用方持有。
/// One concrete key event that can trigger a TUI action.
///
/// Matching via `is_press` handles exact equality plus compatibility fallbacks
/// for terminals that report uppercase letters without SHIFT and Ctrl keys as
/// raw C0 control characters. This means a binding defined as `shift-a` will
/// match either `Shift+a` or plain `A`, and `ctrl-j` will match raw LF.
///
/// This does not model multi-key chords or partial matches; callers that need
/// sequences must keep that state outside this type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct KeyBinding {
    key: KeyCode,
    modifiers: KeyModifiers,
}

/// 由单键或双键和弦组成的用户可见快捷键。
/// A user-visible shortcut made from one key or a two-key chord.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ShortcutHint {
    Single(KeyBinding),
    Chord { prefix: KeyBinding, completion: KeyBinding },
}

impl ShortcutHint {
    pub fn display_label(self) -> String {
        match self {
            Self::Single(binding) => binding.display_label(),
            Self::Chord { prefix, completion } => {
                format!("{} {}", prefix.display_label(), completion.display_label())
            }
        }
    }

    pub fn is_press(self, event: KeyEvent) -> bool {
        matches!(self, Self::Single(binding) if binding.is_press(event))
    }
}

impl From<KeyBinding> for ShortcutHint {
    fn from(binding: KeyBinding) -> Self {
        Self::Single(binding)
    }
}

impl KeyBinding {
    pub const fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    #[allow(dead_code)]
    pub(crate) fn from_event(event: KeyEvent) -> Self {
        let (key, modifiers) = normalize_key_parts(event.code, event.modifiers);
        Self { key, modifiers }
    }

    pub fn is_press(&self, event: KeyEvent) -> bool {
        normalize_key_parts(self.key, self.modifiers)
            == normalize_key_parts(event.code, event.modifiers)
            && (event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat)
    }

    #[allow(dead_code)]
    pub(crate) const fn parts(&self) -> (KeyCode, KeyModifiers) {
        (self.key, self.modifiers)
    }

    pub fn display_label(&self) -> String {
        let modifiers = modifiers_to_string(self.modifiers);
        let key = match self.key {
            KeyCode::Enter => "enter".to_string(),
            KeyCode::Char(' ') => "space".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Right => "→".to_string(),
            KeyCode::PageUp => "pgup".to_string(),
            KeyCode::PageDown => "pgdn".to_string(),
            _ => self.key.to_string().to_ascii_lowercase(),
        };
        format!("{modifiers}{key}")
    }
}

pub(crate) fn normalize_key_parts(
    key: KeyCode,
    mut modifiers: KeyModifiers,
) -> (KeyCode, KeyModifiers) {
    // 统一 C0 控制字符与大写字母表示，使终端差异不影响键位匹配。
    let KeyCode::Char(ch) = key else {
        return (key, modifiers);
    };
    if modifiers.is_empty()
        && let Some(ctrl_char) = c0_control_char_to_ctrl_char(ch)
    {
        return (KeyCode::Char(ctrl_char), KeyModifiers::CONTROL | modifiers);
    }
    if ch.is_ascii_uppercase() {
        modifiers.insert(KeyModifiers::SHIFT);
        return (KeyCode::Char(ch.to_ascii_lowercase()), modifiers);
    }
    (key, modifiers)
}

fn c0_control_char_to_ctrl_char(ch: char) -> Option<char> {
    let code = u32::from(ch);
    match code {
        0x00 => Some(' '),
        0x01..=0x1a => char::from_u32(code - 0x01 + u32::from('a')),
        0x1c..=0x1f => char::from_u32(code - 0x1c + u32::from('4')),
        _ => None,
    }
}

/// 单个动作键位集合的匹配辅助；切片元素是等价候选，不按顺序决定分发优先级，
/// 顺序只供 `primary_binding` 选择界面主提示。
/// Matching helpers for one action's keybinding set.
///
/// Implementations are expected to treat the slice as alternatives for one
/// action. They should not interpret order as priority for dispatch; order is
/// reserved for UI hint selection via `primary_binding`.
pub trait KeyBindingListExt {
    /// 集合中任一绑定匹配 `event` 时为真。
    /// True when any binding in this set matches `event`.
    fn is_pressed(&self, event: KeyEvent) -> bool;
}

impl KeyBindingListExt for [KeyBinding] {
    fn is_pressed(&self, event: KeyEvent) -> bool {
        self.iter().any(|binding| binding.is_press(event))
    }
}

/// 判断事件是否应作为字面文本输入；可搜索选择器借此避免把普通 `j`、`k` 抢作导航，
/// 同时仍允许 `Ctrl+J` 等组合键移动。必须在键位规范化前调用以保留区别。
/// Returns whether an event should be treated as literal text input.
///
/// Searchable pickers use this to avoid stealing plain printable characters for
/// navigation when the same character might be a valid query. For example, a
/// list may bind `j` and `k` for movement, but a searchable list must let
/// plain `j` update the query while still allowing `Ctrl+J` to move. Calling
/// this after normalizing keybindings would blur that distinction and cause
/// printable search input to disappear.
pub fn is_plain_text_key_event(event: KeyEvent) -> bool {
    matches!(
        event,
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        } if !ch.is_ascii_control()
            && !modifiers.contains(KeyModifiers::CONTROL)
            && !modifiers.contains(KeyModifiers::ALT)
    )
}

pub const fn plain(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::NONE)
}

pub const fn alt(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::ALT)
}

pub const fn shift(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::SHIFT)
}

pub const fn ctrl(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::CONTROL)
}

pub const fn ctrl_alt(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::CONTROL.union(KeyModifiers::ALT))
}

fn modifiers_to_string(modifiers: KeyModifiers) -> String {
    let mut result = String::new();
    if modifiers.contains(KeyModifiers::CONTROL) {
        result.push_str(CTRL_PREFIX);
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        result.push_str(SHIFT_PREFIX);
    }
    if modifiers.contains(KeyModifiers::ALT) {
        result.push_str(ALT_PREFIX);
    }
    result
}

impl From<KeyBinding> for Span<'static> {
    fn from(binding: KeyBinding) -> Self {
        (&binding).into()
    }
}
impl From<&KeyBinding> for Span<'static> {
    fn from(binding: &KeyBinding) -> Self {
        Span::styled(binding.display_label(), key_hint_style())
    }
}

impl From<ShortcutHint> for Span<'static> {
    fn from(hint: ShortcutHint) -> Self {
        Span::styled(hint.display_label(), key_hint_style())
    }
}

fn key_hint_style() -> Style {
    Style::default().dim()
}

pub fn has_ctrl_or_alt(mods: KeyModifiers) -> bool {
    (mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT)) && !is_altgr(mods)
}

#[cfg(windows)]
#[inline]
pub fn is_altgr(mods: KeyModifiers) -> bool {
    mods.contains(KeyModifiers::ALT) && mods.contains(KeyModifiers::CONTROL)
}

#[cfg(not(windows))]
#[inline]
pub fn is_altgr(_mods: KeyModifiers) -> bool {
    false
}

#[cfg(test)]
#[path = "key_hint_tests.rs"]
mod tests;
