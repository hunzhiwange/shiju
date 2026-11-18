//! Explicit, user-triggered system language-server installation.
use std::sync::atomic::{AtomicBool, Ordering};

static INSTALLING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Server {
    Rust,
    Python,
    TypeScript,
    Go,
    Lua,
}

impl Server {
    pub const ALL: [Self; 5] = [Self::Rust, Self::Python, Self::TypeScript, Self::Go, Self::Lua];

    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::TypeScript => "JavaScript / TypeScript",
            Self::Go => "Go",
            Self::Lua => "Lua (macOS Homebrew)",
        }
    }

    fn command(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("rustup", &["component", "add", "rust-analyzer", "rust-src"]),
            Self::Python => ("npm", &["install", "-g", "pyright"]),
            Self::TypeScript => {
                ("npm", &["install", "-g", "typescript-language-server", "typescript@6"])
            }
            Self::Go => ("go", &["install", "golang.org/x/tools/gopls@latest"]),
            Self::Lua => ("brew", &["install", "lua-language-server"]),
        }
    }

    pub fn manual_command(self) -> String {
        let (program, args) = self.command();
        format!("{program} {}", args.join(" "))
    }
}

/// Identify installed servers using the same executable resolution as the editor.
pub async fn detect_installed() -> Vec<Server> {
    let resolved = tokio::task::spawn_blocking(|| {
        Server::ALL
            .into_iter()
            .filter_map(|server| {
                let key = match server {
                    Server::Rust => "rust-analyzer",
                    Server::Python => "pyright",
                    Server::TypeScript => "typescript-language-server",
                    Server::Go => "gopls",
                    Server::Lua => "lua-language-server",
                };
                let config = iced_code_editor::lsp_server_config(key)?;
                let command = crate::resolve_lsp_executable(config).ok()?;
                Some((server, command.program))
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();

    let mut installed = Vec::new();
    for (server, program) in resolved {
        // rustup may provide a proxy even when the component is missing.
        if server == Server::Rust {
            let mut probe = tokio::process::Command::new(program);
            #[cfg(windows)]
            probe.creation_flags(0x0800_0000);
            shell_command::apply_augmented_path_tokio_command(&mut probe);
            let status = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                probe
                    .arg("--version")
                    .kill_on_drop(true)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status(),
            )
            .await;
            if !matches!(status, Ok(Ok(status)) if status.success()) {
                continue;
            }
        }
        installed.push(server);
    }
    installed
}

/// Run only fixed package-manager arguments; never invoke a shell or elevate privileges.
pub async fn install(server: Server) -> Result<(), String> {
    if matches!(server, Server::Lua) && !cfg!(target_os = "macos") {
        return Err("Lua 请按官方说明安装：https://luals.github.io/#install".into());
    }
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("已有 LSP 安装正在进行，请等待完成。".into());
    }
    struct Release;
    impl Drop for Release {
        fn drop(&mut self) {
            INSTALLING.store(false, Ordering::SeqCst);
        }
    }
    let _release = Release;
    let (program, args) = server.command();
    let path = shell_command::resolve_executable(program)
        .ok_or_else(|| format!("未找到 {program}，请先安装对应工具并重启 ShiJu。"))?;
    let mut command = tokio::process::Command::new(path);
    shell_command::apply_augmented_path_tokio_command(&mut command);
    command
        .args(args)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let status = tokio::time::timeout(std::time::Duration::from_secs(600), command.status())
        .await
        .map_err(|_| "安装超时，请在终端运行所示命令排查。".to_string())?
        .map_err(|error| format!("无法启动安装：{error}"))?;
    if !status.success() {
        return Err(format!("安装失败（{status}）。请在终端运行所示命令，检查网络或目录权限。"));
    }
    Ok(())
}
