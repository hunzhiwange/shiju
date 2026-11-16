//! 构建和读取项目文件索引。
//! 本模块用受限深度、数量上限和缓存校验控制文件系统扫描成本。

use super::*;
use std::fs;
use std::path::Path;

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent dir should be created");
    }
    fs::write(path, content).expect("file should be written");
}

fn rel_files(root: &Path, files: Vec<String>) -> Vec<String> {
    files
        .into_iter()
        .map(|file| {
            Path::new(&file)
                .strip_prefix(root)
                .expect("file should be inside root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

#[test]
fn refresh_file_index_saves_cache_and_load_file_index_reads_cached_files() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let cache = tempfile::tempdir().expect("cache dir should be created");
    let root_string = root.to_string_lossy().to_string();

    write_file(&root.join("src/main.rs"), "fn main() {}");
    write_file(&root.join("README.md"), "docs");

    let refreshed = rel_files(root, refresh_file_index(&root_string, Some(cache.path())));
    let loaded = load_file_index(&root_string, Some(cache.path()));
    let loaded_files = rel_files(root, loaded.files);

    assert_eq!(refreshed, vec!["README.md", "src/main.rs"]);
    assert_eq!(loaded_files, refreshed);
    assert!(loaded.needs_refresh);
}
