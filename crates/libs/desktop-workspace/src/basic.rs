//! Platform project pickers; desktop owns opening the selected resource.
pub async fn pick_file() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = rfd::AsyncFileDialog::new().pick_file().await;
        handle.map(|file| file.path().to_string_lossy().to_string())
    }
    #[cfg(target_arch = "wasm32")]
    { None }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn pick_attachments() -> Option<Vec<String>> {
    rfd::AsyncFileDialog::new().pick_files().await.map(|handles| {
        handles.into_iter().map(|file| file.path().to_string_lossy().to_string()).collect()
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn pick_folder() -> Option<String> {
    rfd::FileDialog::new().pick_folder()?.to_str().map(str::to_string)
}

pub fn project_path_to_open(input: &str) -> Option<String> {
    if !input.is_empty() { return Some(input.to_string()); }
    #[cfg(not(target_arch = "wasm32"))]
    { std::env::current_dir().ok()?.to_str().map(str::to_string) }
    #[cfg(target_arch = "wasm32")]
    { None }
}

pub fn file_mention(root: &str, path: &str, line: usize, column: usize) -> Option<String> {
    let relative = std::path::Path::new(path).strip_prefix(root).ok()?;
    let relative = relative.to_string_lossy().replace('\\', "/");
    Some(format!("@{relative}:{line}:{column} "))
}
