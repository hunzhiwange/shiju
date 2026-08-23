//! 全局路径测试固定应用目录与缓存版本契约，并验证路径集合在进程内只初始化一次。
//!
//! 本地平台用例还覆盖缓存版本匹配时保留内容、版本过期时清理旧结构，以及异常路径下
//! 的容错行为；临时目录 fixture 将可能破坏数据的清理限制在测试专用路径内。

#[test]
fn constants_match_application_cache_contract() {
    assert_eq!(super::APP, vw_config_types::paths::APP_DIR_NAME);
    assert_eq!(super::CACHE_VERSION, "21");
}

#[test]
fn paths_returns_stable_global_directories() {
    let first = super::paths();
    let second = super::paths();

    assert!(std::ptr::eq(first, second));
    assert!(!first.home.as_os_str().is_empty());
    assert!(first.data.ends_with(super::APP));
    assert!(first.cache.ends_with(super::APP));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn paths_creates_data_cache_and_version_marker() {
    let paths = super::paths();

    assert!(paths.data.is_dir());
    assert!(paths.cache.is_dir());
    assert_eq!(
        std::fs::read_to_string(paths.cache.join("version")).unwrap_or_default().trim(),
        super::CACHE_VERSION
    );
}

#[cfg(not(target_arch = "wasm32"))]
struct TempDir {
    path: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir()
            .join(format!("vibe-window-provider-global-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir should be created");
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ensure_cache_version_preserves_current_cache_contents() {
    let temp = TempDir::new("current-version");
    let cache = temp.path();
    std::fs::write(cache.join("version"), format!("  {} \n", super::CACHE_VERSION))
        .expect("version should be written");
    std::fs::write(cache.join("model-cache.json"), "{}").expect("cache file should be written");

    super::ensure_cache_version(cache);

    assert_eq!(
        std::fs::read_to_string(cache.join("version")).expect("version should remain readable"),
        format!("  {} \n", super::CACHE_VERSION)
    );
    assert!(cache.join("model-cache.json").is_file());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ensure_cache_version_clears_stale_files_and_directories() {
    let temp = TempDir::new("stale-version");
    let cache = temp.path();
    std::fs::write(cache.join("version"), "20").expect("stale version should be written");
    std::fs::write(cache.join("models.json"), "stale").expect("cache file should be written");
    let nested = cache.join("models");
    std::fs::create_dir_all(&nested).expect("cache dir should be created");
    std::fs::write(nested.join("provider.json"), "stale")
        .expect("nested cache file should be written");

    super::ensure_cache_version(cache);

    assert_eq!(
        std::fs::read_to_string(cache.join("version")).expect("new version should be written"),
        super::CACHE_VERSION
    );
    assert!(!cache.join("models.json").exists());
    assert!(!nested.exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ensure_cache_version_treats_blank_version_as_stale() {
    let temp = TempDir::new("blank-version");
    let cache = temp.path();
    std::fs::write(cache.join("version"), " \n\t").expect("blank version should be written");
    std::fs::write(cache.join("old-cache"), "stale").expect("cache file should be written");

    super::ensure_cache_version(cache);

    assert_eq!(
        std::fs::read_to_string(cache.join("version")).expect("new version should be written"),
        super::CACHE_VERSION
    );
    assert!(!cache.join("old-cache").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ensure_cache_version_tolerates_non_directory_cache_path() {
    let temp = TempDir::new("cache-is-file");
    let cache_file = temp.path().join("cache-file");
    std::fs::write(&cache_file, "not a directory").expect("cache fixture should be written");

    super::ensure_cache_version(&cache_file);

    assert_eq!(
        std::fs::read_to_string(cache_file).expect("cache fixture should remain readable"),
        "not a directory"
    );
}
