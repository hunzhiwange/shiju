//! 测试能力探测模块
//!
//! 本模块提供轻量级的能力探测函数，用于在受限环境（如沙箱、容器）中运行的测试场景。
//!
//! ## 主要功能
//!
//! - **环境检测**：检测主目录是否可从环境变量获取
//! - **写入能力检测**：通过创建和删除探测文件验证目录是否可写
//! - **网络能力检测**：验证回环地址绑定能力，用于本地模拟服务器测试
//!
//! ## 使用场景
//!
//! 当测试运行在受限沙箱环境中时，某些操作（如回环绑定、主目录写入）可能被禁止。
//! 这些辅助函数允许测试在能力不足时优雅地跳过，而不是直接失败。
//!
//! ## 示例
//!
//! ```ignore
//! use vibe_agent::test_capabilities::{check_writable_dir, check_loopback_bind};
//!
//! // 检查目录是否可写
//! if let Err(e) = check_writable_dir(Path::new("/tmp")) {
//!     println!("目录不可写，跳过相关测试: {}", e);
//! }
//!
//! // 检查是否可以绑定回环地址
//! if let Err(e) = check_loopback_bind() {
//!     println!("无法绑定回环地址，跳过网络测试: {}", e);
//! }
//! ```

use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 从环境变量获取配置的主目录路径
///
/// 此函数按优先级顺序检查以下环境变量来获取用户主目录：
/// 1. `HOME` - Unix/Linux 系统的标准主目录变量
/// 2. `USERPROFILE` - Windows 系统的主目录变量
///
/// # 返回值
///
/// - `Some(PathBuf)` - 如果找到非空的环境变量值，返回对应路径
/// - `None` - 如果所有相关环境变量都未设置或为空
///
/// # 示例
///
/// ```ignore
/// use vibe_agent::test_capabilities::home_dir_from_env;
///
/// if let Some(home) = home_dir_from_env() {
///     println!("用户主目录: {:?}", home);
/// } else {
///     println!("无法确定用户主目录");
/// }
/// ```
pub fn home_dir_from_env() -> Option<PathBuf> {
    // 首先尝试 HOME 环境变量（Unix/Linux 标准）
    env::var_os("HOME")
        // 过滤掉空值，确保 USERPROFILE 仍可作为回退路径
        .filter(|value| !value.is_empty())
        // 如果 HOME 不存在，尝试 USERPROFILE（Windows 标准）
        .or_else(|| env::var_os("USERPROFILE").filter(|value| !value.is_empty()))
        // 将 OsString 转换为 PathBuf
        .map(PathBuf::from)
}

/// 通过创建和删除探测文件来验证目录是否可写
///
/// 此函数通过以下步骤验证目录的写入能力：
/// 1. 递归创建目录（如果不存在）
/// 2. 创建一个唯一的探测文件
/// 3. 向探测文件写入少量数据
/// 4. 删除探测文件进行清理
///
/// # 参数
///
/// * `path` - 要检查写入权限的目录路径
///
/// # 返回值
///
/// - `Ok(())` - 目录存在且可正常写入和删除文件
/// - `Err(String)` - 操作失败，包含详细的错误描述信息
///
/// # 错误情况
///
/// 函数可能在以下情况下返回错误：
/// - 无法创建目录（权限不足或路径无效）
/// - 无法写入探测文件（磁盘已满或权限问题）
/// - 无法删除探测文件（清理失败）
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
/// use vibe_agent::test_capabilities::check_writable_dir;
///
/// let test_dir = Path::new("/tmp/test_app");
/// match check_writable_dir(test_dir) {
///     Ok(()) => println!("目录可写"),
///     Err(e) => println!("目录不可写: {}", e),
/// }
/// ```
pub fn check_writable_dir(path: &Path) -> Result<(), String> {
    // 步骤 1: 递归创建目录（包括所有父目录）
    // 如果目录已存在，此操作会成功返回
    fs::create_dir_all(path)
        .map_err(|err| format!("无法为能力探测创建目录 {}: {err}", path.display()))?;

    // 步骤 2: 生成唯一的探测文件名
    // 使用当前时间戳（纳秒级）和进程 ID 确保文件名唯一性
    // 这样可以避免多个测试并发运行时产生冲突
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH) // 获取自 Unix 纪元以来的时间
        .map(|duration| duration.as_nanos()) // 转换为纳秒
        .unwrap_or(0); // 如果获取失败，使用 0 作为回退值
    let probe_name = format!(".vibewindow-capability-probe-{}-{nanos}", std::process::id());
    let probe_path = path.join(probe_name);

    // 步骤 3: 写入探测文件
    // 写入少量字节数据以验证写入权限
    fs::write(&probe_path, b"probe")
        .map_err(|err| format!("无法写入探测文件 {}: {err}", probe_path.display()))?;

    // 步骤 4: 清理探测文件
    // 删除文件以验证删除权限并保持环境整洁
    if let Err(err) = fs::remove_file(&probe_path) {
        return Err(format!("无法清理探测文件 {}: {err}", probe_path.display()));
    }

    Ok(())
}

/// 验证回环地址绑定能力
///
/// 此函数尝试绑定回环地址（127.0.0.1）的任意可用端口，
/// 用于验证测试环境是否具备本地网络服务能力。
///
/// # 返回值
///
/// - `Ok(())` - 成功绑定回环地址，表明可以在此环境运行本地模拟服务器测试
/// - `Err(String)` - 绑定失败，包含错误描述
///
/// # 实现细节
///
/// - 使用地址 `127.0.0.1:0`，其中端口 0 表示让操作系统分配任意可用端口
/// - 绑定成功后立即释放监听器（通过 drop）
/// - 此操作不会长时间占用端口
///
/// # 使用场景
///
/// 此函数主要用于以下测试场景：
/// - 本地 HTTP 模拟服务器测试
/// - WebSocket 连接测试
/// - RPC 服务端点测试
///
/// # 示例
///
/// ```ignore
/// use vibe_agent::test_capabilities::check_loopback_bind;
///
/// match check_loopback_bind() {
///     Ok(()) => {
///         // 可以安全运行需要本地网络服务的测试
///         run_network_tests();
///     }
///     Err(e) => {
///         // 跳过需要网络能力的测试
///         println!("跳过网络测试: {}", e);
///     }
/// }
/// ```
pub fn check_loopback_bind() -> Result<(), String> {
    // 尝试绑定回环地址的任意可用端口
    // 端口 0 让操作系统自动分配一个空闲端口
    TcpListener::bind("127.0.0.1:0")
        .map(|listener| {
            // 绑定成功后立即释放监听器
            // 这确保了端口被及时释放，不会影响后续操作
            drop(listener);
        })
        .map_err(|err| format!("回环地址绑定不可用: {err}"))
}
#[cfg(test)]
#[path = "test_capabilities_tests.rs"]
mod test_capabilities_tests;
