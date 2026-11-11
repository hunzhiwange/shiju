//! Only configured shortcut actions cross the OS boundary; other keystrokes are discarded.
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub(crate) const SHIFT: u32 = 1 << 17;
pub(crate) const CONTROL: u32 = 1 << 18;
pub(crate) const ALT: u32 = 1 << 19;
pub(crate) const COMMAND: u32 = 1 << 20;
pub(crate) const MODIFIERS: u32 = SHIFT | CONTROL | ALT | COMMAND;
const DOUBLE_TAP: Duration = Duration::from_millis(280);
// Physical ANSI keys: independent of the current input method, including Chinese IMEs.
const KEYS: &[(u16, &str)] = &[
    (0, "A"),
    (1, "S"),
    (2, "D"),
    (3, "F"),
    (4, "H"),
    (5, "G"),
    (6, "Z"),
    (7, "X"),
    (8, "C"),
    (9, "V"),
    (11, "B"),
    (12, "Q"),
    (13, "W"),
    (14, "E"),
    (15, "R"),
    (16, "Y"),
    (17, "T"),
    (31, "O"),
    (32, "U"),
    (34, "I"),
    (35, "P"),
    (37, "L"),
    (38, "J"),
    (40, "K"),
    (45, "N"),
    (46, "M"),
    (49, "Space"),
];
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub dictation: Option<Shortcut>,
    pub chat: Option<Shortcut>,
}
impl Config {
    pub(crate) fn validate(&self) -> Result<(), String> {
        for key in [self.dictation, self.chat].into_iter().flatten() {
            key.validate()?;
        }
        if self.dictation.is_some() && self.dictation == self.chat {
            return Err("听写和语音聊天快捷键不能相同。".into());
        }
        if self.enabled && (self.dictation.is_none() || self.chat.is_none()) {
            return Err("请先录制听写和语音聊天快捷键。".into());
        }
        if self.enabled && !cfg!(target_os = "macos") {
            return Err("当前平台不支持全局语音快捷键。".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shortcut {
    pub key: u16,
    pub modifiers: u32,
}
impl Shortcut {
    pub(crate) fn validate(self) -> Result<(), String> {
        if !KEYS.iter().any(|(key, _)| *key == self.key)
            || self.modifiers & !MODIFIERS != 0
            || self.modifiers & (CONTROL | ALT | COMMAND) == 0
        {
            return Err("请使用 Control、Option 或 Command 加字母/空格；不支持 Fn。".into());
        }
        if self.modifiers == COMMAND
            || (self.modifiers == COMMAND | SHIFT && self.key == 49)
            || (self.modifiers == COMMAND | ALT && [8, 13, 7].contains(&self.key))
        {
            return Err("该组合与系统或应用快捷键冲突，请换一个组合。".into());
        }
        Ok(())
    }
    pub(crate) fn label(self) -> String {
        let mut label = String::new();
        for (mask, name) in
            [(CONTROL, "Control+"), (ALT, "Option+"), (SHIFT, "Shift+"), (COMMAND, "Command+")]
        {
            if self.modifiers & mask != 0 {
                label.push_str(name);
            }
        }
        label.push_str(
            KEYS.iter().find(|(key, _)| *key == self.key).map(|(_, name)| *name).unwrap_or("?"),
        );
        label
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Start,
    Finish,
    Cancel,
    Chat,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Input {
    Press(bool),
    Release,
    Chat(bool),
    Escape,
    Unsafe,
}
#[derive(Default)]
pub(crate) struct Machine {
    pub(crate) active: bool,
    held: bool,
    hands_free: bool,
    pressed: Option<Instant>,
    released: Option<Instant>,
}
impl Machine {
    pub(crate) fn press(&mut self, now: Instant, repeat: bool) -> Option<Action> {
        if repeat || self.held {
            return None;
        }
        self.held = true;
        if self.hands_free {
            self.reset();
            return Some(Action::Finish);
        }
        if self.active {
            if self.released.is_some_and(|t| now.saturating_duration_since(t) <= DOUBLE_TAP) {
                self.released = None;
                self.hands_free = true;
                return None;
            }
            // A delayed timer must never convert a late press into hands-free.
            self.reset();
            return Some(Action::Finish);
        }
        self.active = true;
        self.pressed = Some(now);
        Some(Action::Start)
    }
    pub(crate) fn release(&mut self, now: Instant) -> Option<Action> {
        if !self.held {
            return None;
        }
        self.held = false;
        if !self.active || self.hands_free {
            return None;
        }
        if self.pressed.is_some_and(|t| now.saturating_duration_since(t) >= DOUBLE_TAP) {
            self.reset();
            Some(Action::Finish)
        } else {
            self.released = Some(now);
            None
        }
    }
    pub(crate) fn tick(&mut self, now: Instant) -> Option<Action> {
        if self.released.is_some_and(|t| now.saturating_duration_since(t) > DOUBLE_TAP) {
            self.reset();
            Some(Action::Finish)
        } else {
            None
        }
    }
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
    pub(crate) fn input(&mut self, input: Input, now: Instant) -> Option<Action> {
        match input {
            Input::Press(repeat) => self.press(now, repeat),
            Input::Release => self.release(now),
            Input::Chat(false) if !self.active => Some(Action::Chat),
            Input::Unsafe | Input::Escape => {
                let active = self.active;
                self.reset();
                active.then_some(Action::Cancel)
            }
            _ => None,
        }
    }
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub machine: Machine,
    loaded: bool,
    blocked: bool,
    pub error: Option<String>,
    #[cfg(target_os = "macos")]
    listener: Option<super::hotkeys_macos::Listener>,
}
impl Runtime {
    pub(crate) fn status(&self) -> String {
        if let Some(error) = &self.error {
            return error.clone();
        }
        #[cfg(target_os = "macos")]
        if let Some(listener) = &self.listener {
            return format!(
                "全局热键已启用：听写 {}；语音聊天 {}。",
                listener.config.dictation.map(Shortcut::label).unwrap_or_default(),
                listener.config.chat.map(Shortcut::label).unwrap_or_default(),
            );
        }
        if cfg!(target_os = "macos") {
            "全局热键未启用；在语音设置中录制并保存。".into()
        } else {
            "当前平台不支持全局语音快捷键。".into()
        }
    }
    pub(crate) fn interval(&self) -> Duration {
        #[cfg(target_os = "macos")]
        if self.listener.is_some() {
            return Duration::from_millis(50);
        }
        Duration::from_secs(1)
    }
    pub(crate) fn check(&self, config: &Config) -> Result<(), String> {
        config.validate()?;
        #[cfg(target_os = "macos")]
        super::hotkeys_macos::Listener::check(config, self.listener.as_ref())?;
        Ok(())
    }
    pub(crate) fn stop_session(&mut self) {
        self.reload();
        self.loaded = true;
        self.blocked = true;
        self.error = None;
    }
    /// Only a successful explicit save can clear a safety stop or manual disable.
    pub(crate) fn resume_after_save(&mut self) {
        self.blocked = false;
        self.error = None;
        self.reload();
    }
    pub(crate) fn reload(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.listener = None;
        }
        self.loaded = false;
        self.machine.reset();
    }
    pub(crate) fn poll(&mut self) -> Vec<Action> {
        if self.blocked {
            return Vec::new();
        }
        if !self.loaded {
            self.loaded = true;
            self.error = None;
            let result = super::preferences::Preferences::load().and_then(|prefs| {
                prefs.hotkeys.validate()?;
                if prefs.hotkeys.enabled {
                    #[cfg(target_os = "macos")]
                    {
                        self.listener = Some(super::hotkeys_macos::Listener::new(prefs.hotkeys)?);
                    }
                }
                Ok(())
            });
            if let Err(e) = result {
                self.error = Some(e);
                self.blocked = true;
            }
        }
        let mut actions = Vec::new();
        #[cfg(target_os = "macos")]
        if let Some(listener) = &self.listener {
            let inputs = listener.poll();
            for (input, at) in inputs {
                // A blocked UI must not start recording from an old queued shortcut.
                let input =
                    if at.elapsed() > Duration::from_secs(1) { Input::Unsafe } else { input };
                if let Some(action) = self.machine.input(input, at) {
                    actions.push(action);
                }
                if matches!(input, Input::Unsafe) {
                    self.error = Some("全局热键已停止：权限、锁屏、休眠或按键状态发生变化。请检查输入监控权限并重新保存设置。".into());
                    if actions.last() != Some(&Action::Cancel) {
                        actions.push(Action::Cancel);
                    }
                    self.listener = None;
                    self.blocked = true;
                    break;
                }
            }
        }
        if let Some(action) = self.machine.tick(Instant::now()) {
            actions.push(action);
        }
        actions
    }
}

#[cfg(test)]
#[path = "hotkeys_tests.rs"]
mod tests;

pub(crate) fn record_subscription() -> iced::Subscription<crate::app::Message> {
    iced::event::listen_raw(|event, _, _| {
        let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            physical_key,
            modifiers,
            repeat,
            ..
        }) = event
        else {
            return None;
        };
        if repeat {
            return None;
        }
        use iced::keyboard::key::{Code, Physical};
        let Physical::Code(code) = physical_key else {
            return None;
        };
        let key = match code {
            Code::Escape => {
                return Some(crate::app::Message::Settings(
                    crate::app::message::SettingsMessage::Voice(
                        crate::app::message::settings::voice::Event::RecordHotkey(None),
                    ),
                ));
            }
            Code::KeyA => 0,
            Code::KeyS => 1,
            Code::KeyD => 2,
            Code::KeyF => 3,
            Code::KeyH => 4,
            Code::KeyG => 5,
            Code::KeyZ => 6,
            Code::KeyX => 7,
            Code::KeyC => 8,
            Code::KeyV => 9,
            Code::KeyB => 11,
            Code::KeyQ => 12,
            Code::KeyW => 13,
            Code::KeyE => 14,
            Code::KeyR => 15,
            Code::KeyY => 16,
            Code::KeyT => 17,
            Code::KeyO => 31,
            Code::KeyU => 32,
            Code::KeyI => 34,
            Code::KeyP => 35,
            Code::KeyL => 37,
            Code::KeyJ => 38,
            Code::KeyK => 40,
            Code::KeyN => 45,
            Code::KeyM => 46,
            Code::Space => 49,
            Code::Fn => 63,
            _ => return None,
        };
        let flags = u32::from(modifiers.shift()) * SHIFT
            | u32::from(modifiers.control()) * CONTROL
            | u32::from(modifiers.alt()) * ALT
            | u32::from(modifiers.logo()) * COMMAND;
        Some(crate::app::Message::Settings(crate::app::message::SettingsMessage::Voice(
            crate::app::message::settings::voice::Event::RecordedHotkey(Shortcut {
                key,
                modifiers: flags,
            }),
        )))
    })
}
