use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KeyContext {
    Global,
    Chat,
    Autocomplete,
    Confirmation,
    Tabs,
    Transcript,
    HistorySearch,
    Task,
    Scroll,
    Footer,
    Select,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KeyAction {
    Submit,
    Cancel,
    Quit,
    Accept,
    Previous,
    Next,
    Search,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyStroke {
    modifiers: KeyModifiers,
    code: KeyCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyChord(Vec<KeyStroke>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyBinding {
    pub(crate) context: KeyContext,
    pub(crate) action: KeyAction,
    pub(crate) chord: KeyChord,
}

#[derive(Debug, Clone)]
pub(crate) struct KeyBindingRegistry {
    bindings: Vec<KeyBinding>,
    pending: Vec<KeyStroke>,
    pending_since: Option<Instant>,
    chord_timeout: Duration,
}

impl Default for KeyBindingRegistry {
    fn default() -> Self {
        Self {
            bindings: default_bindings(),
            pending: Vec::new(),
            pending_since: None,
            chord_timeout: Duration::from_millis(750),
        }
    }
}

impl KeyBindingRegistry {
    pub(crate) fn apply_override(
        &mut self,
        context: KeyContext,
        action: KeyAction,
        spec: &str,
    ) -> Result<(), String> {
        let chord = parse_key_chord(spec)?;
        if chord.0.iter().any(is_reserved_stroke) {
            return Err(format!("reserved key cannot be rebound: {spec}"));
        }
        self.bindings.retain(|binding| !(binding.context == context && binding.action == action));
        self.bindings.push(KeyBinding { context, action, chord });
        Ok(())
    }

    pub(crate) fn resolve(
        &mut self,
        contexts: &[KeyContext],
        key: KeyEvent,
        now: Instant,
    ) -> Option<KeyAction> {
        if self
            .pending_since
            .is_some_and(|started| now.duration_since(started) > self.chord_timeout)
        {
            self.pending.clear();
            self.pending_since = None;
        }
        self.pending.push(KeyStroke::from_event(key));
        self.pending_since.get_or_insert(now);

        for context in contexts.iter().rev() {
            if let Some(binding) = self
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.context == *context && binding.chord.0 == self.pending)
            {
                let action = binding.action;
                self.pending.clear();
                self.pending_since = None;
                return Some(action);
            }
        }
        let is_prefix = contexts.iter().any(|context| {
            self.bindings.iter().any(|binding| {
                binding.context == *context && binding.chord.0.starts_with(&self.pending)
            })
        });
        if !is_prefix {
            self.pending.clear();
            self.pending_since = None;
        }
        None
    }

    pub(crate) fn shortcut(&self, context: KeyContext, action: KeyAction) -> Option<String> {
        self.bindings
            .iter()
            .rev()
            .find(|binding| binding.context == context && binding.action == action)
            .map(|binding| display_chord(&binding.chord))
    }
}

impl KeyStroke {
    fn from_event(key: KeyEvent) -> Self {
        Self { modifiers: normalized_modifiers(key.modifiers), code: key.code }
    }
}

pub(crate) fn is_reserved_exit(key: KeyEvent) -> bool {
    normalized_modifiers(key.modifiers) == KeyModifiers::CONTROL
        && matches!(key.code, KeyCode::Char('c' | 'd'))
}

pub(crate) fn is_prompt_selection_copy(key: KeyEvent) -> bool {
    normalized_modifiers(key.modifiers) == KeyModifiers::CONTROL
        && matches!(key.code, KeyCode::Char('c'))
}

pub(crate) fn parse_key_chord(spec: &str) -> Result<KeyChord, String> {
    let strokes = spec.split_ascii_whitespace().map(parse_stroke).collect::<Result<Vec<_>, _>>()?;
    if strokes.is_empty() || strokes.len() > 2 {
        return Err("key chord must contain one or two strokes".to_string());
    }
    Ok(KeyChord(strokes))
}

fn parse_stroke(spec: &str) -> Result<KeyStroke, String> {
    let mut modifiers = KeyModifiers::NONE;
    let mut code = None;
    for token in spec.split('+') {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
            "alt" | "option" => modifiers.insert(KeyModifiers::ALT),
            "shift" => modifiers.insert(KeyModifiers::SHIFT),
            "enter" => code = Some(KeyCode::Enter),
            "esc" | "escape" => code = Some(KeyCode::Esc),
            "tab" => code = Some(KeyCode::Tab),
            "up" => code = Some(KeyCode::Up),
            "down" => code = Some(KeyCode::Down),
            "left" => code = Some(KeyCode::Left),
            "right" => code = Some(KeyCode::Right),
            value if value.len() > 1 && value.starts_with('f') => {
                let number =
                    value[1..].parse::<u8>().map_err(|_| format!("unknown key token: {value}"))?;
                code = Some(KeyCode::F(number));
            }
            value if value.chars().count() == 1 => {
                code = value.chars().next().map(KeyCode::Char);
            }
            value => return Err(format!("unknown key token: {value}")),
        }
    }
    let code = code.ok_or_else(|| format!("missing key in binding: {spec}"))?;
    Ok(KeyStroke { modifiers: normalized_modifiers(modifiers), code })
}

fn normalized_modifiers(modifiers: KeyModifiers) -> KeyModifiers {
    modifiers
        & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::SUPER)
}

fn is_reserved_stroke(stroke: &KeyStroke) -> bool {
    stroke.modifiers == KeyModifiers::CONTROL && matches!(stroke.code, KeyCode::Char('c' | 'd'))
}

fn default_bindings() -> Vec<KeyBinding> {
    [
        (KeyContext::Global, KeyAction::Quit, "ctrl+c"),
        (KeyContext::Global, KeyAction::Help, "f1"),
        (KeyContext::Chat, KeyAction::Submit, "enter"),
        (KeyContext::Chat, KeyAction::Cancel, "esc"),
        (KeyContext::Autocomplete, KeyAction::Accept, "tab"),
        (KeyContext::Autocomplete, KeyAction::Previous, "up"),
        (KeyContext::Autocomplete, KeyAction::Next, "down"),
        (KeyContext::Transcript, KeyAction::Search, "ctrl+f"),
        (KeyContext::HistorySearch, KeyAction::Accept, "enter"),
        (KeyContext::Confirmation, KeyAction::Accept, "enter"),
        (KeyContext::Task, KeyAction::Previous, "up"),
        (KeyContext::Task, KeyAction::Next, "down"),
        (KeyContext::Scroll, KeyAction::Previous, "up"),
        (KeyContext::Scroll, KeyAction::Next, "down"),
        (KeyContext::Tabs, KeyAction::Next, "ctrl+tab"),
        (KeyContext::Footer, KeyAction::Help, "?"),
        (KeyContext::Select, KeyAction::Accept, "enter"),
    ]
    .into_iter()
    .filter_map(|(context, action, spec)| {
        parse_key_chord(spec).ok().map(|chord| KeyBinding { context, action, chord })
    })
    .collect()
}

fn display_chord(chord: &KeyChord) -> String {
    chord.0.iter().map(display_stroke).collect::<Vec<_>>().join(" ")
}

fn display_stroke(stroke: &KeyStroke) -> String {
    let mut parts = Vec::new();
    if stroke.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if stroke.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if stroke.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift".to_string());
    }
    parts.push(match stroke.code {
        KeyCode::Char(ch) => ch.to_ascii_uppercase().to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::F(number) => format!("F{number}"),
        _ => "?".to_string(),
    });
    parts.join("+")
}
