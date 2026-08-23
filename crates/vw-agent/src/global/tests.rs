// 验证全局状态目录、配置加载与进程级共享状态的约束。

// 路径 fixture 显式控制平台目录和测试主目录，以验证逐级回退顺序。
use super::*;

#[test]
fn global_paths_include_app_named_directories() {
    let paths = paths();

    assert!(paths.data.ends_with(APP));
    assert!(paths.config.ends_with(APP));
    assert!(paths.cache.ends_with(APP));
    assert!(paths.state.ends_with(APP));
    assert!(paths.bin.ends_with("bin"));
    assert!(paths.log.ends_with("log"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_paths_uses_platform_directories_with_state_dir() {
    let root = PathBuf::from("/tmp/vw-global-platform");
    let base = platform_dirs(&root, Some(root.join("state")));
    let paths = resolve_paths(Some(base), None);

    assert_eq!(paths.home, root.join("home"));
    assert_eq!(paths.data, root.join("data").join(APP));
    assert_eq!(paths.cache, root.join("cache").join(APP));
    assert_eq!(paths.config, root.join("config").join(APP));
    assert_eq!(paths.state, root.join("state").join(APP));
    assert_eq!(paths.bin, root.join("data").join(APP).join("bin"));
    assert_eq!(paths.log, root.join("data").join(APP).join("log"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_paths_uses_platform_data_dir_when_state_dir_is_missing() {
    let root = PathBuf::from("/tmp/vw-global-no-state");
    let base = platform_dirs(&root, None);
    let paths = resolve_paths(Some(base), None);

    assert_eq!(paths.state, root.join("data").join(APP));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_paths_uses_test_home_for_home_only_when_platform_dirs_exist() {
    let root = PathBuf::from("/tmp/vw-global-test-home");
    let base = platform_dirs(&root, Some(root.join("state")));
    let test_home = root.join("isolated-home");
    let paths = resolve_paths(Some(base), Some(test_home.clone()));

    assert_eq!(paths.home, test_home);
    assert_eq!(paths.data, root.join("data").join(APP));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_paths_falls_back_to_home_scoped_directories_without_platform_dirs() {
    let home = PathBuf::from("/tmp/vw-global-home");
    let paths = resolve_paths(None, Some(home.clone()));

    assert_eq!(paths.home, home);
    assert_eq!(paths.data, paths.home.join(".local").join("share").join(APP));
    assert_eq!(paths.cache, paths.home.join(".cache").join(APP));
    assert_eq!(paths.config, paths.home.join(".config").join(APP));
    assert_eq!(paths.state, paths.home.join(".local").join("state").join(APP));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_paths_falls_back_to_current_directory_without_platform_dirs_or_test_home() {
    let paths = resolve_paths(None, None);

    assert_eq!(paths.home, PathBuf::from("."));
    assert_eq!(paths.data, PathBuf::from(".").join(".local").join("share").join(APP));
    assert_eq!(paths.cache, PathBuf::from(".").join(".cache").join(APP));
    assert_eq!(paths.config, PathBuf::from(".").join(".config").join(APP));
    assert_eq!(paths.state, PathBuf::from(".").join(".local").join("state").join(APP));
    assert_eq!(paths.bin, paths.data.join("bin"));
    assert_eq!(paths.log, paths.data.join("log"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn prepare_paths_creates_directories_and_writes_missing_cache_version() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let paths = test_paths(temp.path().to_path_buf());

    prepare_paths(&paths);

    assert!(paths.data.is_dir());
    assert!(paths.config.is_dir());
    assert!(paths.state.is_dir());
    assert!(paths.log.is_dir());
    assert!(paths.bin.is_dir());
    assert!(paths.cache.is_dir());
    assert_eq!(
        std::fs::read_to_string(paths.cache.join("version")).expect("version should exist"),
        CACHE_VERSION
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn prepare_paths_preserves_cache_when_version_matches() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let paths = test_paths(temp.path().to_path_buf());
    std::fs::create_dir_all(&paths.cache).expect("cache should be created");
    std::fs::write(paths.cache.join("version"), format!(" {CACHE_VERSION}\n"))
        .expect("version should be written");
    std::fs::write(paths.cache.join("kept.txt"), "cached").expect("cache file should be written");

    prepare_paths(&paths);

    assert_eq!(
        std::fs::read_to_string(paths.cache.join("kept.txt")).expect("cache file should remain"),
        "cached"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn prepare_paths_clears_cache_when_version_is_empty() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let paths = test_paths(temp.path().to_path_buf());
    std::fs::create_dir_all(&paths.cache).expect("cache should be created");
    std::fs::write(paths.cache.join("version"), " \n").expect("version should be written");
    std::fs::write(paths.cache.join("stale.txt"), "stale").expect("stale file should be written");

    prepare_paths(&paths);

    assert!(!paths.cache.join("stale.txt").exists());
    assert_eq!(
        std::fs::read_to_string(paths.cache.join("version")).expect("version should be rewritten"),
        CACHE_VERSION
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn prepare_paths_rewrites_cache_when_version_path_is_directory() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let paths = test_paths(temp.path().to_path_buf());
    let version_dir = paths.cache.join("version");
    std::fs::create_dir_all(&version_dir).expect("version dir should be created");
    std::fs::write(paths.cache.join("stale.txt"), "stale").expect("stale file should be written");
    std::fs::write(version_dir.join("stale.txt"), "stale")
        .expect("nested stale file should be written");

    prepare_paths(&paths);

    assert!(!paths.cache.join("stale.txt").exists());
    assert!(paths.cache.join("version").is_file());
    assert_eq!(
        std::fs::read_to_string(paths.cache.join("version")).expect("version should be rewritten"),
        CACHE_VERSION
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn prepare_paths_clears_files_and_directories_when_version_is_stale() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let paths = test_paths(temp.path().to_path_buf());
    let stale_dir = paths.cache.join("nested");
    std::fs::create_dir_all(&stale_dir).expect("nested cache dir should be created");
    std::fs::write(paths.cache.join("version"), "old").expect("version should be written");
    std::fs::write(paths.cache.join("stale.txt"), "stale").expect("stale file should be written");
    std::fs::write(stale_dir.join("stale.txt"), "stale")
        .expect("nested stale file should be written");

    prepare_paths(&paths);

    assert!(!paths.cache.join("stale.txt").exists());
    assert!(!stale_dir.exists());
    assert_eq!(
        std::fs::read_to_string(paths.cache.join("version")).expect("version should be rewritten"),
        CACHE_VERSION
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn prepare_paths_tolerates_cache_path_that_is_not_a_directory() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let paths = test_paths(temp.path().to_path_buf());
    std::fs::create_dir_all(paths.cache.parent().expect("cache should have parent"))
        .expect("cache parent should be created");
    std::fs::write(&paths.cache, "not a directory").expect("cache path file should be written");

    prepare_paths(&paths);

    assert!(paths.cache.is_file());
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_dirs(root: &std::path::Path, state: Option<PathBuf>) -> PlatformDirs {
    PlatformDirs {
        home: root.join("home"),
        data: root.join("data"),
        cache: root.join("cache"),
        config: root.join("config"),
        state,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn test_paths(root: PathBuf) -> GlobalPaths {
    let home = root.join("home");
    let data = root.join("data");
    let bin = data.join("bin");
    let log = data.join("log");
    let cache = root.join("cache");
    let config = root.join("config");
    let state = root.join("state");

    GlobalPaths { home, data, bin, log, cache, config, state }
}
