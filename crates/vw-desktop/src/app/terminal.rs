//! 定义桌面终端标签、Shell 选项及原生终端实例的生命周期。

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod panes;
#[cfg(not(target_arch = "wasm32"))]
use iced::widget::pane_grid::Pane;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 桌面终端支持启动的交互式 Shell。
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
/// 终端配色策略；当前仅跟随应用主题。
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

/// 单个终端标签及其原生终端实例和重命名草稿。
pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    pub default_title: bool,
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
/// 从应用主题生成保证前景、背景和 ANSI 色可读的终端调色板。
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

/// 限制 Web 端文本终端内容长度，保留最新尾部并避免无限增长。
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

/// 按 UTF-8 字符边界保留字符串尾部，使结果字节数不超过上限。
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
        size: font_size,
        scale_factor: 1.4,
        font_type: iced::Font::with_name(family),
    }
}

#[cfg(not(target_arch = "wasm32"))]
use std::collections::VecDeque;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
/// 新启动 PTY 的终端实例及后台读写任务。
pub struct TerminalRuntime {
    pub writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    pub output: Arc<Mutex<VecDeque<u8>>>,
    pub child: Arc<Mutex<Box<dyn portable_pty::Child + Send>>>,
}

#[cfg(not(target_arch = "wasm32"))]
/// 创建 PTY、启动 Shell 并返回终端实例与后台任务。
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

/// 多标签终端的活动项、字体、主题与待回收后台任务。
pub struct TerminalState {
    pub is_visible: bool,
    pub(crate) is_fullscreen: bool,
    pub tabs: Vec<TerminalTab>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) panes: panes::TerminalPanes,
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
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) content_menu: Option<(u64, iced::Point)>,
    /// Closed terminals kept alive until iced cancels their subscriptions.
    ///
    /// Dropping `iced_term::Terminal` while its subscription stream is still
    /// running can panic with "terminal event channel closed unexpected".
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) retiring: Vec<iced_term::Terminal>,
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
            is_visible: is_visible && !cfg!(target_arch = "wasm32"),
            is_fullscreen: false,
            tabs: vec![],
            #[cfg(not(target_arch = "wasm32"))]
            panes: panes::TerminalPanes::default(),
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
            #[cfg(not(target_arch = "wasm32"))]
            content_menu: None,
            #[cfg(not(target_arch = "wasm32"))]
            retiring: Vec::new(),
        }
    }

    pub fn new(
        is_visible: bool,
        shell: Shell,
        theme: TerminalTheme,
        font_family: String,
        font_size: f32,
        initial_cwd: Option<std::path::PathBuf>,
    ) -> Self {
        let mut state =
            Self::blank_with_settings(is_visible, shell, theme, font_family, font_size, 200.0);
        // Only spawn a PTY when the panel will be shown; keep hidden sessions empty.
        if is_visible {
            let _ = state.ensure_default_terminal(initial_cwd);
        }
        state
    }

    /// Create the first interactive terminal tab if none exist yet.
    ///
    /// Returns `true` when a new PTY tab was created.
    #[cfg(not(target_arch = "wasm32"))]
    /// 在没有标签时创建默认终端；已存在标签时不重复创建。
    pub fn ensure_default_terminal(&mut self, cwd: Option<std::path::PathBuf>) -> bool {
        if !self.tabs.is_empty() {
            return false;
        }
        self.add_terminal(cwd.or_else(|| std::env::current_dir().ok()))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn ensure_default_terminal(&mut self, _cwd: Option<std::path::PathBuf>) -> bool {
        false
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_terminal(&mut self, cwd: Option<std::path::PathBuf>) -> bool {
        self.add_terminal_to_pane(self.panes.focused, cwd)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn add_terminal_to_pane(
        &mut self,
        pane: Pane,
        cwd: Option<std::path::PathBuf>,
    ) -> bool {
        if self.panes.grid.get(pane).is_none() {
            return false;
        }
        let tab = self.spawn_interactive_terminal(cwd);
        self.next_id = self.next_id.wrapping_add(1);
        if let Some(tab) = tab {
            self.push_tab(pane, tab);
            true
        } else {
            false
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn push_tab(&mut self, pane: Pane, tab: TerminalTab) {
        self.panes.add(pane, tab.id);
        self.panes.split_menu = None;
        self.active_id = Some(tab.id);
        self.tabs.push(tab);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_interactive_terminal(&self, cwd: Option<std::path::PathBuf>) -> Option<TerminalTab> {
        let id = self.next_id;

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

        let mut term = iced_term::Terminal::new(id, settings).ok()?;
        let fs = build_font_settings(&self.font_family, self.font_size);
        let _ = term.handle(iced_term::Command::ChangeFont(fs));
        Some(TerminalTab {
            id,
            title: format!("终端 {id}"),
            default_title: true,
            term,
            edit_title: None,
        })
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

        self.push_tab(
            self.panes.focused,
            TerminalTab { id, title, default_title: false, term, edit_title: None },
        );
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

        self.push_tab(
            self.panes.focused,
            TerminalTab { id, title, default_title: false, term, edit_title: None },
        );
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

        self.push_tab(
            self.panes.focused,
            TerminalTab { id, title, default_title: false, term, edit_title: None },
        );
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn activate_visible_tab(&mut self, id: u64) -> bool {
        if !self.is_visible || !self.panes.is_visible_tab(id) {
            return false;
        }
        self.panes.select(id);
        self.active_id = Some(id);
        true
    }

    pub fn select_terminal(&mut self, id: u64) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if !self.panes.select(id) {
            return false;
        }
        #[cfg(target_arch = "wasm32")]
        if !self.tabs.iter().any(|tab| tab.id == id) {
            return false;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.content_menu = None;
            self.panes.split_menu = None;
        }
        self.active_id = Some(id);
        self.is_visible = true;
        if self.height < 160.0 {
            self.height = 200.0;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
            let font = build_font_settings(&self.font_family, self.font_size);
            let _ = tab.term.handle(iced_term::Command::ChangeFont(font));
        }
        true
    }

    pub(crate) fn set_visible(&mut self, visible: bool) {
        self.is_visible = visible && !cfg!(target_arch = "wasm32");
        if !self.is_visible {
            self.set_fullscreen(false);
        }
    }

    pub(crate) fn set_fullscreen(&mut self, fullscreen: bool) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.panes.split_menu = None;
        }
        if self.is_fullscreen == fullscreen {
            return;
        }
        self.is_fullscreen = fullscreen;
        self.is_dragging = false;
        self.drag_anchor_y = None;
        self.tab_context_menu_id = None;
        self.tab_context_menu_pos = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.content_menu = None;
        }
    }

    /// 关闭指定标签并选择相邻标签；找不到 ID 时返回 `false`。
    pub fn close_terminal(&mut self, id: u64) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            let tab = self.tabs.remove(pos);
            #[cfg(not(target_arch = "wasm32"))]
            if self.content_menu.is_some_and(|(menu_id, _)| menu_id == id) {
                self.content_menu = None;
            }
            if self.tab_context_menu_id == Some(id) {
                self.tab_context_menu_id = None;
                self.tab_context_menu_pos = None;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.active_id = self.panes.remove(id);
            }
            #[cfg(target_arch = "wasm32")]
            if self.active_id == Some(id) {
                self.active_id = self.tabs.first().map(|t| t.id);
            }
            #[cfg(not(target_arch = "wasm32"))]
            self.retiring.push(tab.term);
            if self.tabs.is_empty() {
                self.set_fullscreen(false);
            }
            #[cfg(target_arch = "wasm32")]
            let _ = tab;
            true
        } else {
            false
        }
    }

    /// Absorb terminals from a replaced [`TerminalState`] so they are not dropped
    /// while iced may still be running their subscriptions.
    /// 接管被替换状态的后台任务，延后到安全时机统一回收。
    pub fn absorb_replaced(&mut self, old: Self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.retiring.extend(old.retiring);
            self.retiring.extend(old.tabs.into_iter().map(|tab| tab.term));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = old;
        }
    }

    pub fn has_retired(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            !self.retiring.is_empty()
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    /// 中止并清空所有已退役终端的后台任务。
    pub fn reap_retired(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.retiring.clear();
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
        let settings = vw_config_types::ui::AppSystemSettingsConfig::default();
        Self::blank_with_settings(
            false,
            Shell::Bash,
            TerminalTheme::System,
            settings.terminal_font_family,
            settings.terminal_font_size,
            200.0,
        )
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod terminal_tests;

/// Open a local directory without interpreting it as shell code or an `open` option.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn open_local_directory(path: &str) -> std::io::Result<()> {
    if !crate::app::config::gateway_client_endpoint().is_loopback() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "远端终端只能通过网关 PTY 打开，不能启动本机终端",
        ));
    }
    #[cfg(target_os = "macos")]
    {
        let directory = std::fs::canonicalize(path)?;
        if !directory.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "终端工作目录必须是目录",
            ));
        }
        let status = std::process::Command::new("/usr/bin/open")
            .args(["-a", "Terminal"])
            .arg(directory)
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other(format!("打开终端失败：{status}")));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "当前平台不支持打开本机终端"))
    }
}
