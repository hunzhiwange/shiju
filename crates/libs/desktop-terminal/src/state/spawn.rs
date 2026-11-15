use super::*;

impl<S: Copy + Into<&'static str>> TerminalState<S> {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn spawn_interactive_terminal(
        &self,
        cwd: Option<std::path::PathBuf>,
    ) -> Option<TerminalTab> {
        let id = self.next_id;

        let shell_path = self.shell.into().to_string();

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

        let shell_path = self.shell.into().to_string();

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
}
