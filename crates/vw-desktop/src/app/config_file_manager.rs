//! 客户端文件管理器边界：网关路径只允许在直连本机时交给本机系统。

/// 不对远端网关路径做本机探测或系统调用。
pub(crate) async fn reveal_gateway_path_locally(
    endpoint: &vw_gateway_client::GatewayEndpoint,
    path: &str,
) -> Result<(), String> {
    if !endpoint.is_loopback() {
        return Err("远端路径不能在本机文件管理器中打开".into());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Err("浏览器不支持打开本机文件管理器".into())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 原始文件路径不做 URL 解码，保留文件名中的百分号。
        let path = if path.starts_with("file:") {
            let url = reqwest::Url::parse(path).map_err(|_| "无效的文件 URL".to_string())?;
            if url.host_str().is_some_and(|host| host != "localhost")
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err("只支持本机文件 URL".into());
            }
            url.to_file_path().map_err(|_| "无效的本机文件路径".to_string())?
        } else {
            std::path::PathBuf::from(path)
        };
        if !path.is_absolute()
            || path.to_string_lossy().starts_with("//")
            || path.to_string_lossy().starts_with("\\\\")
            || path.to_string_lossy().chars().any(char::is_control)
        {
            return Err("只支持本机绝对路径".into());
        }
        tokio::task::spawn_blocking(move || {
            if !path.try_exists().map_err(|error| error.to_string())? {
                return Err("文件或目录不存在".into());
            }
            #[cfg(target_os = "macos")]
            let status = std::process::Command::new("/usr/bin/open")
                .arg("-R")
                .arg(&path)
                .status();
            #[cfg(windows)]
            let status = std::process::Command::new("explorer")
                .arg(format!("/select,{}", path.display()))
                .status();
            #[cfg(all(not(target_os = "macos"), not(windows)))]
            let status = std::process::Command::new("xdg-open")
                .arg(if path.is_dir() { path.as_path() } else { path.parent().unwrap_or(&path) })
                .status();
            let status = status.map_err(|error| error.to_string())?;
            if !status.success() {
                return Err(format!("文件管理器打开失败：{status}"));
            }
            Ok(())
        })
        .await
        .map_err(|error| error.to_string())?
    }
}
