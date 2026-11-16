//! 桌面文件索引门面，保留应用缓存目录和原有调用签名。

pub use file_index::{FileIndexLoadResult, index_files};

/// 从应用数据目录读取项目文件索引。
pub fn load_file_index(root: &str) -> FileIndexLoadResult {
    let cache_dir = super::project_dirs()
        .map(|dirs| dirs.data_local_dir().join("file_index_cache"));
    file_index::load_file_index(root, cache_dir.as_deref())
}

/// 刷新项目索引并保存至应用数据目录。
pub fn refresh_file_index(root: &str) -> Vec<String> {
    let cache_dir = super::project_dirs()
        .map(|dirs| dirs.data_local_dir().join("file_index_cache"));
    file_index::refresh_file_index(root, cache_dir.as_deref())
}
