#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
}

impl Shell {
    pub fn all() -> [Shell; 2] {
        [Shell::Bash, Shell::Zsh]
    }
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalTheme {
    System,
}

impl std::fmt::Display for TerminalTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalTheme::System => write!(f, "跟随系统"),
        }
    }
}

pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub term: iced_term::Terminal,
    #[cfg(target_arch = "wasm32")]
    pub term: (),
    pub edit_title: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
fn color_channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(not(target_arch = "wasm32"))]
fn to_hex(color: iced::Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        color_channel_to_u8(color.r),
        color_channel_to_u8(color.g),
        color_channel_to_u8(color.b)
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_palette(theme: &iced::Theme) -> iced_term::ColorPalette {
    let mut p = iced_term::ColorPalette::default();
    let palette = theme.extended_palette();
    let base = palette.background.base.color;
    let text = palette.background.base.text;
    let weak = palette.background.weak.color;
    let strong = palette.background.strong.color;

    p.background = to_hex(base);
    p.foreground = to_hex(text);
    p.black = to_hex(weak);
    p.white = to_hex(strong);
    p.bright_black = to_hex(weak.scale_alpha(0.85));
    p.bright_white = to_hex(strong.scale_alpha(0.85));

    p.red = to_hex(palette.danger.base.color);
    p.bright_red = to_hex(palette.danger.strong.color);
    p.green = to_hex(palette.success.base.color);
    p.bright_green = to_hex(palette.success.strong.color);
    p.yellow = to_hex(palette.warning.base.color);
    p.bright_yellow = to_hex(palette.warning.strong.color);
    p.blue = to_hex(palette.primary.base.color);
    p.bright_blue = to_hex(palette.primary.strong.color);
    p.magenta = to_hex(palette.secondary.base.color);
    p.bright_magenta = to_hex(palette.secondary.strong.color);
    p.cyan = to_hex(palette.secondary.base.color);
    p.bright_cyan = to_hex(palette.secondary.strong.color);

    p
}

#[cfg(target_arch = "wasm32")]
pub fn build_palette(_theme: &iced::Theme) -> () {
    ()
}

pub fn truncate_terminal_content(c: &mut iced::widget::text_editor::Content) {
    const LIMIT: usize = 4_000;
    let s = c.text();
    if s.len() <= LIMIT {
        return;
    }
    let mut cut = s.len() - LIMIT;
    while cut < s.len() && !s.is_char_boundary(cut) {
        cut += 1;
    }
    let new = s[cut..].to_string();
    *c = iced::widget::text_editor::Content::with_text(&new);
}

pub fn truncate_string_to_limit(s: &str, limit: usize) -> String {
    if s.len() <= limit {
        return s.to_string();
    }
    let mut cut = s.len() - limit;
    while cut < s.len() && !s.is_char_boundary(cut) {
        cut += 1;
    }
    s[cut..].to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn build_font_settings(font_family: &str, font_size: f32) -> iced_term::settings::FontSettings {
    let family = if font_family.trim().is_empty() {
        "JetBrains Mono"
    } else {
        Box::leak(font_family.trim().to_string().into_boxed_str())
    };
    iced_term::settings::FontSettings {
        size: 13.0,
        scale_factor: ((font_size / 13.0) * 1.2).max(0.5),
        font_type: iced::Font::with_name(family),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn enter_sends_cr_bindings() -> Vec<(
    iced_term::bindings::Binding<iced_term::bindings::InputKind>,
    iced_term::bindings::BindingAction,
)> {
    use iced::keyboard::key::Named;
    use iced_term::bindings::{Binding, BindingAction, InputKind};

    let empty = iced::keyboard::Modifiers::empty();
    let shift = iced::keyboard::Modifiers::SHIFT;
    let none = iced_term::TermMode::empty();

    vec![
        (
            Binding {
                target: InputKind::KeyCode(Named::Enter),
                modifiers: empty,
                terminal_mode_include: none,
                terminal_mode_exclude: none,
            },
            BindingAction::Char('\n'),
        ),
        (
            Binding {
                target: InputKind::KeyCode(Named::Enter),
                modifiers: shift,
                terminal_mode_include: none,
                terminal_mode_exclude: none,
            },
            BindingAction::Char('\n'),
        ),
        (
            Binding {
                target: InputKind::Char("\n".to_string()),
                modifiers: empty,
                terminal_mode_include: none,
                terminal_mode_exclude: none,
            },
            BindingAction::Char('\n'),
        ),
        (
            Binding {
                target: InputKind::Char("\n".to_string()),
                modifiers: shift,
                terminal_mode_include: none,
                terminal_mode_exclude: none,
            },
            BindingAction::Char('\n'),
        ),
        (
            Binding {
                target: InputKind::Char("\r".to_string()),
                modifiers: empty,
                terminal_mode_include: none,
                terminal_mode_exclude: none,
            },
            BindingAction::Char('\n'),
        ),
        (
            Binding {
                target: InputKind::Char("\r".to_string()),
                modifiers: shift,
                terminal_mode_include: none,
                terminal_mode_exclude: none,
            },
            BindingAction::Char('\n'),
        ),
    ]
}

#[cfg(not(target_arch = "wasm32"))]
use std::collections::VecDeque;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
pub struct TerminalRuntime {
    pub writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    pub output: Arc<Mutex<VecDeque<u8>>>,
    pub child: Arc<Mutex<Box<dyn portable_pty::Child + Send>>>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_pty_terminal(
    cwd: Option<&str>,
    #[cfg_attr(windows, allow(unused_variables))] shell_choice: Shell,
) -> Result<TerminalRuntime, String> {
    use portable_pty::{CommandBuilder, PtySize};
    use std::io::Read;
    use std::path::PathBuf;

    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    #[cfg(windows)]
    let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    #[cfg(not(windows))]
    let shell = match shell_choice {
        Shell::Bash => "/bin/bash".to_string(),
        Shell::Zsh => "/bin/zsh".to_string(),
    };

    let mut cmd = CommandBuilder::new(shell);
    if let Some(cwd) = cwd {
        cmd.cwd(PathBuf::from(cwd));
    }

    let child = pair.slave.spawn_command(cmd).map_err(|e| format!("{e:?}"))?;

    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let output = Arc::new(Mutex::new(VecDeque::new()));
    let out_clone = output.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut q) = out_clone.lock() {
                        q.extend(buf[..n].iter().copied());
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(TerminalRuntime {
        writer: Arc::new(Mutex::new(writer)),
        output,
        child: Arc::new(Mutex::new(child)),
    })
}

pub struct TerminalState {
    pub is_visible: bool,
    pub tabs: Vec<TerminalTab>,
    pub active_id: Option<u64>,
    pub next_id: u64,
    pub height: f32,
    pub is_dragging: bool,
    pub drag_anchor_y: Option<f32>,
    pub drag_start_height: f32,
    pub shell: Shell,
    pub theme: TerminalTheme,
    pub font_family: String,
    pub font_size: f32,
    pub tab_context_menu_id: Option<u64>,
    pub tab_context_menu_pos: Option<(f32, f32)>,
}

impl TerminalState {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn apply_app_theme(&mut self, theme: &iced::Theme) {
        self.apply_theme_palette(theme);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn apply_theme_palette(&mut self, theme: &iced::Theme) {
        let palette = build_palette(theme);
        for tab in &mut self.tabs {
            let _ = tab.term.handle(iced_term::Command::ChangeTheme(Box::new(palette.clone())));
        }
    }
    pub fn blank_with_settings(
        is_visible: bool,
        shell: Shell,
        theme: TerminalTheme,
        font_family: String,
        font_size: f32,
        height: f32,
    ) -> Self {
        Self {
            is_visible,
            tabs: vec![],
            active_id: None,
            next_id: 1,
            height,
            is_dragging: false,
            drag_anchor_y: None,
            drag_start_height: 0.0,
            shell,
            theme,
            font_family,
            font_size,
            tab_context_menu_id: None,
            tab_context_menu_pos: None,
        }
    }

    pub fn new(
        is_visible: bool,
        shell: Shell,
        theme: TerminalTheme,
        font_family: String,
        font_size: f32,
        _initial_cwd: Option<std::path::PathBuf>,
    ) -> Self {
        let _shell_path = match shell {
            Shell::Bash => "/bin/bash".to_string(),
            Shell::Zsh => "/bin/zsh".to_string(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        let mut terminals = Vec::new();
        #[cfg(target_arch = "wasm32")]
        let terminals = Vec::new();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let settings = iced_term::settings::Settings {
                backend: {
                    use std::collections::HashMap;

                    let mut env = HashMap::new();
                    env.insert("TERM".to_string(), "xterm-256color".to_string());
                    env.insert("CLICOLOR".to_string(), "1".to_string());
                    env.insert("LSCOLORS".to_string(), "ExFxBxDxCxegedabagacad".to_string());
                    iced_term::settings::BackendSettings {
                        program: _shell_path,
                        args: vec!["-i".to_string()],
                        env,
                        working_directory: _initial_cwd.or_else(|| std::env::current_dir().ok()),
                        ..Default::default()
                    }
                },
                ..Default::default()
            };

            if let Ok(mut term) = iced_term::Terminal::new(1, settings) {
                let fs = build_font_settings(&font_family, font_size);
                let _ = term.handle(iced_term::Command::ChangeFont(fs));
                let _ = term.handle(iced_term::Command::AddBindings(enter_sends_cr_bindings()));

                let first_terminal =
                    TerminalTab { id: 1, title: "终端 1".to_string(), term, edit_title: None };
                terminals.push(first_terminal);
            }
        }

        Self {
            is_visible,
            tabs: terminals,
            active_id: Some(1),
            next_id: 2,
            height: 200.0,
            is_dragging: false,
            drag_anchor_y: None,
            drag_start_height: 0.0,
            shell,
            theme,
            font_family,
            font_size,
            tab_context_menu_id: None,
            tab_context_menu_pos: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_terminal(&mut self, cwd: Option<std::path::PathBuf>) -> bool {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let shell_path = match self.shell {
            Shell::Bash => "/bin/bash".to_string(),
            Shell::Zsh => "/bin/zsh".to_string(),
        };

        let settings = iced_term::settings::Settings {
            backend: {
                use std::collections::HashMap;
                let mut env = HashMap::new();
                env.insert("TERM".to_string(), "xterm-256color".to_string());
                env.insert("CLICOLOR".to_string(), "1".to_string());
                env.insert("LSCOLORS".to_string(), "ExFxBxDxCxegedabagacad".to_string());
                iced_term::settings::BackendSettings {
                    program: shell_path,
                    args: vec!["-i".to_string()],
                    env,
                    working_directory: cwd,
                    ..Default::default()
                }
            },
            ..Default::default()
        };

        if let Ok(mut term) = iced_term::Terminal::new(id, settings) {
            let fs = build_font_settings(&self.font_family, self.font_size);
            let _ = term.handle(iced_term::Command::ChangeFont(fs));
            let _ = term.handle(iced_term::Command::AddBindings(enter_sends_cr_bindings()));

            self.tabs.push(TerminalTab {
                id,
                title: format!("终端 {}", id),
                term,
                edit_title: None,
            });
            self.active_id = Some(id);
            true
        } else {
            false
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_terminal_with_command(
        &mut self,
        title: String,
        cwd: Option<std::path::PathBuf>,
        command: String,
    ) -> Option<u64> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let shell_path = match self.shell {
            Shell::Bash => "/bin/bash".to_string(),
            Shell::Zsh => "/bin/zsh".to_string(),
        };

        let settings = iced_term::settings::Settings {
            backend: {
                use std::collections::HashMap;
                let mut env = HashMap::new();
                env.insert("TERM".to_string(), "xterm-256color".to_string());
                env.insert("CLICOLOR".to_string(), "1".to_string());
                env.insert("LSCOLORS".to_string(), "ExFxBxDxCxegedabagacad".to_string());
                iced_term::settings::BackendSettings {
                    program: shell_path,
                    args: vec!["-lc".to_string(), command],
                    env,
                    working_directory: cwd,
                    ..Default::default()
                }
            },
            ..Default::default()
        };

        let mut term = iced_term::Terminal::new(id, settings).ok()?;
        let fs = build_font_settings(&self.font_family, self.font_size);
        let _ = term.handle(iced_term::Command::ChangeFont(fs));
        let _ = term.handle(iced_term::Command::AddBindings(enter_sends_cr_bindings()));

        self.tabs.push(TerminalTab { id, title, term, edit_title: None });
        self.active_id = Some(id);
        Some(id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_bash_terminal_with_init(
        &mut self,
        title: String,
        cwd: Option<std::path::PathBuf>,
        init_script: String,
    ) -> Option<u64> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let settings = iced_term::settings::Settings {
            backend: {
                use std::collections::HashMap;
                let mut env = HashMap::new();
                env.insert("TERM".to_string(), "xterm-256color".to_string());
                env.insert("CLICOLOR".to_string(), "1".to_string());
                env.insert("LSCOLORS".to_string(), "ExFxBxDxCxegedabagacad".to_string());
                env.insert("VW_TOOL_INIT".to_string(), init_script);
                env.insert(
                    "PROMPT_COMMAND".to_string(),
                    r#"eval "$VW_TOOL_INIT"; unset VW_TOOL_INIT; unset PROMPT_COMMAND"#.to_string(),
                );

                iced_term::settings::BackendSettings {
                    program: "/bin/bash".to_string(),
                    args: vec!["--noprofile".to_string(), "--norc".to_string(), "-i".to_string()],
                    env,
                    working_directory: cwd,
                    ..Default::default()
                }
            },
            ..Default::default()
        };

        let mut term = iced_term::Terminal::new(id, settings).ok()?;
        let fs = build_font_settings(&self.font_family, self.font_size);
        let _ = term.handle(iced_term::Command::ChangeFont(fs));
        let _ = term.handle(iced_term::Command::AddBindings(enter_sends_cr_bindings()));

        self.tabs.push(TerminalTab { id, title, term, edit_title: None });
        self.active_id = Some(id);
        Some(id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_tool_terminal_with_init(
        &mut self,
        title: String,
        cwd: Option<std::path::PathBuf>,
        init_script: String,
    ) -> Option<u64> {
        if std::path::Path::new("/bin/zsh").exists() {
            self.add_zsh_terminal_with_init(title, cwd, init_script)
        } else {
            self.add_bash_terminal_with_init(title, cwd, init_script)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn add_zsh_terminal_with_init(
        &mut self,
        title: String,
        cwd: Option<std::path::PathBuf>,
        init_script: String,
    ) -> Option<u64> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let zdotdir = std::env::temp_dir().join("vibewindow_tool_shell_zsh");
        let _ = std::fs::create_dir_all(&zdotdir);
        let zshrc_path = zdotdir.join(".zshrc");
        let zshrc = r#"
if [ -f "$HOME/.zshrc" ]; then
  source "$HOME/.zshrc"
fi
if [[ -n "$VW_TOOL_INIT" ]]; then
  eval "$VW_TOOL_INIT"
  unset VW_TOOL_INIT
fi
"#
        .trim()
        .to_string();
        let write_rc = match std::fs::read_to_string(&zshrc_path) {
            Ok(existing) => existing != zshrc,
            Err(_) => true,
        };
        if write_rc {
            let _ = std::fs::write(&zshrc_path, zshrc);
        }

        let settings = iced_term::settings::Settings {
            backend: {
                use std::collections::HashMap;
                let mut env = HashMap::new();
                env.insert("TERM".to_string(), "xterm-256color".to_string());
                env.insert("CLICOLOR".to_string(), "1".to_string());
                env.insert("LSCOLORS".to_string(), "ExFxBxDxCxegedabagacad".to_string());
                env.insert("ZDOTDIR".to_string(), zdotdir.to_string_lossy().to_string());
                env.insert("VW_TOOL_INIT".to_string(), init_script);

                iced_term::settings::BackendSettings {
                    program: "/bin/zsh".to_string(),
                    args: vec!["-i".to_string()],
                    env,
                    working_directory: cwd,
                    ..Default::default()
                }
            },
            ..Default::default()
        };

        let mut term = iced_term::Terminal::new(id, settings).ok()?;
        let fs = build_font_settings(&self.font_family, self.font_size);
        let _ = term.handle(iced_term::Command::ChangeFont(fs));
        let _ = term.handle(iced_term::Command::AddBindings(enter_sends_cr_bindings()));

        self.tabs.push(TerminalTab { id, title, term, edit_title: None });
        self.active_id = Some(id);
        Some(id)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn add_terminal(&mut self, _cwd: Option<std::path::PathBuf>) -> bool {
        false
    }

    #[cfg(target_arch = "wasm32")]
    pub fn add_terminal_with_command(
        &mut self,
        _title: String,
        _cwd: Option<std::path::PathBuf>,
        _command: String,
    ) -> Option<u64> {
        None
    }

    pub fn select_terminal(&mut self, id: u64) {
        self.active_id = Some(id);
        self.is_visible = true;
        if self.height < 160.0 {
            self.height = 200.0;
        }
        // Refresh theme/font for the selected tab to ensure it matches current state
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            let fs = build_font_settings(&self.font_family, self.font_size);
            let _ = tab.term.handle(iced_term::Command::ChangeFont(fs));
        }
    }

    pub fn close_terminal(&mut self, id: u64) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            if self.active_id == Some(id) {
                self.active_id = self.tabs.first().map(|t| t.id);
            }
        }
    }

    pub fn start_rename(&mut self, id: u64) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.edit_title = Some(tab.title.clone());
        }
    }

    pub fn update_rename(&mut self, id: u64, new_title: String) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.edit_title = Some(new_title);
        }
    }

    pub fn save_rename(&mut self, id: u64) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id)
            && let Some(new) = tab.edit_title.take()
        {
            let name = new.trim().to_string();
            if !name.is_empty() {
                tab.title = name;
            }
        }
    }

    pub fn cancel_rename(&mut self, id: u64) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.edit_title = None;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
        let fs = build_font_settings(&self.font_family, self.font_size);
        for tab in &mut self.tabs {
            let _ = tab.term.handle(iced_term::Command::ChangeFont(fs.clone()));
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_font_family(&mut self, family: String) {
        self.font_family = family;
        let fs = build_font_settings(&self.font_family, self.font_size);
        for tab in &mut self.tabs {
            let _ = tab.term.handle(iced_term::Command::ChangeFont(fs.clone()));
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_font_family(&mut self, family: String) {
        self.font_family = family;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_theme(&mut self, theme: TerminalTheme) {
        self.theme = theme;
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_theme(&mut self, theme: TerminalTheme) {
        self.theme = theme;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn apply_palette(&mut self, palette: iced_term::ColorPalette) {
        for tab in &mut self.tabs {
            let _ = tab.term.handle(iced_term::Command::ChangeTheme(Box::new(palette.clone())));
        }
    }
}

impl Default for TerminalState {
    fn default() -> Self {
        Self::blank_with_settings(
            false,
            Shell::Bash,
            TerminalTheme::System,
            "JetBrains Mono".to_string(),
            13.0,
            200.0,
        )
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod terminal_tests;
