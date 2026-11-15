//! Telegram 测试辅助工具模块。
//!
//! 本模块提供跨平台的符号链接创建函数，供附件和路径安全相关测试复用。
//! Unix 与 Windows 的目录符号链接 API 不同，因此这里通过条件编译隐藏平台差异。

use std::path::Path;

#[cfg(unix)]
/// 在 Unix 平台创建文件符号链接。
///
/// # 参数
/// - `src`: 符号链接指向的源文件路径。
/// - `dst`: 要创建的符号链接路径。
///
/// # 错误处理
/// 创建失败会触发 panic，因为测试 fixture 无法建立时应立即失败。
pub fn symlink_file(src: &Path, dst: &Path) {
    std::os::unix::fs::symlink(src, dst).expect("symlink should be created");
}

#[cfg(windows)]
/// 在 Windows 平台创建文件符号链接。
///
/// # 参数
/// - `src`: 符号链接指向的源文件路径。
/// - `dst`: 要创建的符号链接路径。
///
/// # 错误处理
/// 创建失败会触发 panic，因为测试 fixture 无法建立时应立即失败。
pub fn symlink_file(src: &Path, dst: &Path) {
    std::os::windows::fs::symlink_file(src, dst).expect("symlink should be created");
}

#[cfg(unix)]
/// 在 Unix 平台创建目录符号链接。
///
/// # 参数
/// - `src`: 符号链接指向的源目录路径。
/// - `dst`: 要创建的符号链接路径。
///
/// # 错误处理
/// 创建失败会触发 panic，因为测试 fixture 无法建立时应立即失败。
pub fn symlink_dir(src: &Path, dst: &Path) {
    std::os::unix::fs::symlink(src, dst).expect("symlink should be created");
}

#[cfg(windows)]
/// 在 Windows 平台创建目录符号链接。
///
/// # 参数
/// - `src`: 符号链接指向的源目录路径。
/// - `dst`: 要创建的符号链接路径。
///
/// # 错误处理
/// 创建失败会触发 panic，因为测试 fixture 无法建立时应立即失败。
pub fn symlink_dir(src: &Path, dst: &Path) {
    std::os::windows::fs::symlink_dir(src, dst).expect("symlink should be created");
}
