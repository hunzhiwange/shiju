//! Open an already-authorized local directory.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_local_directory(path: &str) -> std::io::Result<()> {
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

pub fn subscription(term: &iced_term::Terminal) -> iced::Subscription<crate::message::TerminalMessage> {
    // Tab numbers restart for each project; subscriptions must follow the PTY instance.
    term.subscription()
        .with(term.widget_id().clone())
        .map(|(_, event)| crate::message::TerminalMessage::Event(event))
}

#[cfg(test)]
#[path = "local_subscription_tests.rs"]
mod subscription_tests;
