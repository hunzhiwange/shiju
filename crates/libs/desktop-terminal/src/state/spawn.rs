use super::*;
use std::collections::HashMap;

fn terminal_env() -> HashMap<String, String> {
    let mut env = HashMap::from([
        ("TERM".to_string(), "xterm-256color".to_string()),
        ("CLICOLOR".to_string(), "1".to_string()),
        ("LSCOLORS".to_string(), "ExFxBxDxCxegedabagacad".to_string()),
    ]);

    #[cfg(unix)]
    {
        let has_locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .into_iter()
            .any(|key| std::env::var_os(key).is_some_and(|value| !value.is_empty()));
        if !has_locale {
            // GUI launches may have no locale, leaving zsh in byte-oriented C mode.
            let locale = if cfg!(target_os = "macos") {
                "en_US.UTF-8"
            } else {
                "C.UTF-8"
            };
            env.insert("LC_CTYPE".to_string(), locale.to_string());
        }
    }

    env
}

impl<S: Copy + Into<&'static str>> TerminalState<S> {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn spawn_interactive_terminal(
        &self,
        cwd: Option<std::path::PathBuf>,
    ) -> Option<TerminalTab> {
        self.spawn_interactive_tab(self.next_id, self.shell.into(), cwd)
    }

    pub(crate) fn spawn_interactive_tab(
        &self,
        id: u64,
        shell_program: &'static str,
        cwd: Option<std::path::PathBuf>,
    ) -> Option<TerminalTab> {
        let settings = iced_term::settings::Settings {
            backend: iced_term::settings::BackendSettings {
                program: shell_program.to_string(),
                args: vec!["-i".to_string()],
                env: terminal_env(),
                working_directory: cwd,
                ..Default::default()
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
            shell_program,
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

        let shell_program = self.shell.into();

        let settings = iced_term::settings::Settings {
            backend: iced_term::settings::BackendSettings {
                program: shell_program.to_string(),
                args: vec!["-lc".to_string(), command],
                env: terminal_env(),
                working_directory: cwd,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut term = iced_term::Terminal::new(id, settings).ok()?;
        let fs = build_font_settings(&self.font_family, self.font_size);
        let _ = term.handle(iced_term::Command::ChangeFont(fs));

        self.push_tab(
            self.panes.focused,
            TerminalTab { id, title, default_title: false, shell_program, term, edit_title: None },
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
                let mut env = terminal_env();
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
            TerminalTab {
                id,
                title,
                default_title: false,
                shell_program: "/bin/bash",
                term,
                edit_title: None,
            },
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
                let mut env = terminal_env();
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
            TerminalTab {
                id,
                title,
                default_title: false,
                shell_program: "/bin/zsh",
                term,
                edit_title: None,
            },
        );
        Some(id)
    }
}
