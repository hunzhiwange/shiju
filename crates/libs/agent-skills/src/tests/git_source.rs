//! Git 源识别功能测试模块
//!
//! 本模块包含针对 `is_git_source` 函数的单元测试，用于验证 Git 源地址的识别逻辑。
//!
//! # 主要功能
//!
//! - 测试远程 Git 协议（HTTPS、HTTP、SSH、Git 协议）的识别
//! - 测试 SCP 风格 Git 地址的识别
//! - 验证本地路径和无效输入的拒绝逻辑
//!
//! # 测试覆盖
//!
//! 1. **合法 Git 源识别**：验证远程 URL 和 SCP 风格地址被正确识别
//! 2. **非法输入拒绝**：验证本地路径、不完整地址和无效字符串被正确拒绝

use crate::installer::is_git_source;

/// 测试 Git 源识别函数接受远程协议和 SCP 风格地址
///
/// # 测试目的
///
/// 验证 `is_git_source` 函数能够正确识别以下合法的 Git 源地址格式：
/// - HTTPS 协议 URL（`https://`）
/// - HTTP 协议 URL（`http://`）
/// - SSH 协议 URL（`ssh://`）
/// - Git 协议 URL（`git://`）
/// - SCP 风格地址（`user@host:path`）
///
/// # 测试数据
///
/// 包含六种不同格式的 Git 源地址，覆盖主流远程仓库访问方式
///
/// # 断言
///
/// 所有测试用例应被识别为有效的 Git 源（`is_git_source` 返回 `true`）
#[test]
fn git_source_detection_accepts_remote_protocols_and_scp_style() {
    // 定义合法的 Git 源地址集合，覆盖主要远程协议和 SCP 风格
    let sources = [
        "https://github.com/some-org/some-skill.git", // HTTPS 协议（标准格式）
        "http://github.com/some-org/some-skill.git",  // HTTP 协议（非加密）
        "ssh://git@github.com/some-org/some-skill.git", // SSH 协议（URL 格式）
        "git://github.com/some-org/some-skill.git",   // Git 协议（原生）
        "git@github.com:some-org/some-skill.git",     // SCP 风格（GitHub 常用）
        "git@localhost:skills/some-skill.git",        // SCP 风格（局域网）
    ];

    // 遍历所有合法源地址，验证识别函数返回 true
    for source in sources {
        assert!(is_git_source(source), "expected git source detection for '{source}'");
    }
}

/// 测试 Git 源识别函数拒绝本地路径和无效输入
///
/// # 测试目的
///
/// 验证 `is_git_source` 函数能够正确拒绝以下非法输入：
/// - 相对路径（`./path`）
/// - 绝对路径（`/path`、Windows 路径）
/// - 不完整的 SCP 地址（缺少路径部分）
/// - 不完整的协议 URL（缺少主机和路径）
/// - 非法字符串（既不是 URL 也不是 Git 地址）
/// - 混淆路径（包含 Git 地址片段的本地路径）
///
/// # 测试数据
///
/// 包含七种不应被识别为 Git 源的输入类型
///
/// # 断言
///
/// 所有测试用例应被识别为非 Git 源（`is_git_source` 返回 `false`）
#[test]
fn git_source_detection_rejects_local_paths_and_invalid_inputs() {
    // 定义应被拒绝的输入集合，包括本地路径、不完整地址和无效字符串
    let sources = [
        "./skills/local-skill",            // 相对路径
        "/tmp/skills/local-skill",         // Unix 绝对路径
        "C:\\skills\\local-skill",         // Windows 绝对路径
        "git@github.com",                  // 不完整的 SCP 地址（缺少路径）
        "ssh://",                          // 不完整的协议 URL
        "not-a-url",                       // 普通字符串（非 URL）
        "dir/git@github.com:org/repo.git", // 混淆路径（本地路径包含 Git 片段）
    ];

    // 遍历所有非法输入，验证识别函数返回 false
    for source in sources {
        assert!(!is_git_source(source), "expected local/invalid source detection for '{source}'");
    }
}
