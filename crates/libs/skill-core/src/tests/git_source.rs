//! Git 源检测功能的单元测试模块
//!
//! 本模块用于测试 `is_git_source` 函数的正确性，验证其对各种 Git 远程仓库地址格式的识别能力。
//! 主要测试场景包括：
//! - 接受标准远程协议（HTTPS、SSH、Git 协议）
//! - 接受 SCP 风格的 Git 地址
//! - 拒绝本地路径和格式无效的输入
//!
//! # 测试覆盖
//!
//! - 远程协议：`https://`、`http://`、`ssh://`、`git://`
//! - SCP 风格：`git@host:path` 格式
//! - 拒绝场景：相对路径、绝对路径、Windows 路径、不完整地址

use super::*;

/// 测试 Git 源检测函数接受所有有效的远程协议和 SCP 风格地址
///
/// # 测试目的
///
/// 验证 `is_git_source` 函数能够正确识别以下格式的 Git 远程仓库地址：
/// - HTTPS 协议（`https://`）
/// - HTTP 协议（`http://`）
/// - SSH 协议（`ssh://git@`）
/// - Git 协议（`git://`）
/// - SCP 风格地址（`git@host:path`）
///
/// # 测试数据
///
/// 包含来自不同主机（GitHub、localhost）的完整 Git 仓库地址
#[test]
fn git_source_detection_accepts_remote_protocols_and_scp_style() {
    // 定义一组应该被识别为有效 Git 源的测试用例
    // 包括：标准 URL 格式（https/http/ssh/git 协议）和 SCP 风格格式
    let sources = [
        "https://github.com/some-org/some-skill.git",
        "http://github.com/some-org/some-skill.git",
        "ssh://git@github.com/some-org/some-skill.git",
        "git://github.com/some-org/some-skill.git",
        "git@github.com:some-org/some-skill.git",
        "git@localhost:skills/some-skill.git",
    ];

    // 遍历所有测试用例，确保每个都能被正确识别为 Git 源
    for source in sources {
        assert!(is_git_source(source), "expected git source detection for '{source}'");
    }
}

/// 测试 Git 源检测函数拒绝本地路径和无效输入
///
/// # 测试目的
///
/// 验证 `is_git_source` 函数能够正确拒绝以下不应被识别为 Git 远程源的输入：
/// - 相对路径（`./` 开头）
/// - 绝对路径（`/` 开头）
/// - Windows 路径（`C:\` 开头）
/// - 不完整的 Git 地址（缺少路径部分）
/// - 纯协议字符串
/// - 普通字符串
/// - 以 Git 风格开头但包含路径分隔符的字符串
///
/// # 测试数据
///
/// 包含各种边界情况和无效格式，确保函数的健壮性
#[test]
fn git_source_detection_rejects_local_paths_and_invalid_inputs() {
    // 定义一组不应被识别为 Git 源的测试用例
    // 包括：本地路径（相对/绝对/Windows 格式）和无效/不完整的 Git 地址
    let sources = [
        "./skills/local-skill",            // 相对路径
        "/tmp/skills/local-skill",         // Unix 绝对路径
        "C:\\skills\\local-skill",         // Windows 路径
        "git@github.com",                  // 不完整的 SCP 地址（缺少路径）
        "ssh://",                          // 不完整的协议字符串
        "not-a-url",                       // 普通字符串
        "dir/git@github.com:org/repo.git", // 路径前缀的伪 Git 地址
    ];

    // 遍历所有测试用例，确保每个都不会被误识别为 Git 源
    for source in sources {
        assert!(!is_git_source(source), "expected local/invalid source detection for '{source}'");
    }
}
