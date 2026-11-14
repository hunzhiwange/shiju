//! WebView 数据目录选择与日志追加。

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(not(any(target_arch = "wasm32", coverage)))]
pub fn webview_data_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("dev", "VibeWindow", "vibe-window") {
        #[cfg(target_os = "windows")]
        {
            let base = dirs.config_dir().parent().unwrap_or_else(|| dirs.config_dir());
            return base.join("webview_data");
        }

        #[cfg(not(target_os = "windows"))]
        {
            return dirs.data_dir().join("webview_data");
        }
    }

    fallback_webview_data_dir()
}

#[cfg(all(not(target_arch = "wasm32"), coverage))]
pub fn webview_data_dir() -> PathBuf {
    fallback_webview_data_dir()
}

#[cfg(not(target_arch = "wasm32"))]
fn fallback_webview_data_dir() -> PathBuf {
    let base_data_dir = directories::BaseDirs::new().map(|dirs| dirs.data_dir().to_path_buf());
    fallback_webview_data_dir_from(base_data_dir.as_deref())
}

#[cfg(not(target_arch = "wasm32"))]
fn fallback_webview_data_dir_from(base_data_dir: Option<&Path>) -> PathBuf {
    base_data_dir
        .map(|path| path.join("vibe-window").join("webview_data"))
        .unwrap_or_else(|| std::env::current_dir().unwrap().join("webview_data"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn append_log(log_path: &Path, s: &str) {
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut buf = String::with_capacity(s.len() + 1);
    buf.push_str(s);
    buf.push('\n');
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, buf.as_bytes()));
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "storage_tests.rs"]
mod tests;
