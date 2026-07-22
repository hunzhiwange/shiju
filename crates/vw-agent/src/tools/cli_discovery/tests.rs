//! CLI 工具发现功能的测试模块
//!
//! 本模块包含对 CLI 工具自动发现功能的单元测试，主要测试：
//! - 工具发现功能的基本行为
//! - 工具排除列表的过滤功能
//! - CLI 工具分类的字符串表示
//!
//! 这些测试确保系统能够正确识别和过滤系统中可用的 CLI 工具。

use super::{CliCategory, discover_cli_tools};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// 测试工具发现功能返回非空结果
///
/// 验证 `discover_cli_tools` 函数能够返回一个包含已发现 CLI 工具的向量，
/// 且每个工具的名称字段都不为空。
///
/// # 测试场景
/// - 不提供任何包含列表和排除列表
/// - 检查返回结果中每个工具的名称字段有效性
#[test]
fn discover_returns_vec() {
    // 使用空的包含列表和排除列表调用发现函数
    let results = discover_cli_tools(&[], &[]);

    // 验证每个发现的 CLI 工具都有非空的名称
    for cli in &results {
        assert!(!cli.name.is_empty());
    }
}

/// 测试排除列表能够正确过滤工具
///
/// 验证当工具被添加到排除列表时，该工具不会出现在发现结果中。
///
/// # 测试场景
/// - 将 "git" 添加到排除列表
/// - 验证结果中不包含 "git" 工具
#[test]
fn excluded_tools_are_skipped() {
    // 调用发现函数，排除 git 工具
    let results = discover_cli_tools(&[], &["git".to_string()]);

    // 断言结果中不存在 git 工具
    assert!(!results.iter().any(|r| r.name == "git"));
}

/// 测试 CLI 工具分类的字符串显示
///
/// 验证各个 CLI 工具分类枚举值能够正确转换为人类可读的字符串表示。
///
/// # 测试场景
/// - 测试所有预定义的 CLI 工具分类
/// - 验证每个分类的字符串表示符合预期格式
#[test]
fn category_display() {
    // 验证版本控制分类的显示字符串
    assert_eq!(CliCategory::VersionControl.to_string(), "Version Control");

    // 验证编程语言分类的显示字符串
    assert_eq!(CliCategory::Language.to_string(), "Language");

    // 验证包管理器分类的显示字符串
    assert_eq!(CliCategory::PackageManager.to_string(), "Package Manager");

    // 验证容器分类的显示字符串
    assert_eq!(CliCategory::Container.to_string(), "Container");

    // 验证构建工具分类的显示字符串
    assert_eq!(CliCategory::Build.to_string(), "Build");

    // 验证云服务分类的显示字符串
    assert_eq!(CliCategory::Cloud.to_string(), "Cloud");
}

#[cfg(unix)]
#[test]
fn probe_cli_captures_the_first_stdout_version_line_from_an_isolated_stub() {
    let directory = tempfile::tempdir().expect("temporary CLI directory should be created");
    let executable = directory.path().join("version-tool");
    std::fs::write(&executable, b"#!/bin/sh\nprintf 'version-tool 1.2.3\\nignored\\n'\n")
        .expect("CLI stub should be written");
    let mut permissions =
        std::fs::metadata(&executable).expect("CLI stub metadata should be readable").permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&executable, permissions).expect("CLI stub should be executable");
    let executable_name = executable.to_string_lossy().into_owned();

    let discovered = super::probe_cli(&executable_name, &["--version"], CliCategory::Build)
        .expect("absolute executable stub should be discovered");

    assert_eq!(discovered.name, executable_name);
    assert_eq!(discovered.path, executable);
    assert_eq!(discovered.version.as_deref(), Some("version-tool 1.2.3"));
    assert_eq!(discovered.category, CliCategory::Build);
}

#[cfg(unix)]
#[test]
fn get_version_uses_stderr_when_stdout_is_empty() {
    let directory = tempfile::tempdir().expect("temporary CLI directory should be created");
    let executable = executable_stub(
        directory.path(),
        "stderr-version-tool",
        "#!/bin/sh\nprintf 'stderr-tool 2.0\\nignored\\n' >&2\n",
    );
    let executable_name = executable.to_string_lossy().into_owned();

    let version = super::get_version(&executable_name, &["--version"]);

    assert_eq!(version.as_deref(), Some("stderr-tool 2.0"));
}

#[cfg(unix)]
#[test]
fn get_version_returns_none_for_missing_or_blank_commands() {
    let directory = tempfile::tempdir().expect("temporary CLI directory should be created");
    let blank = executable_stub(directory.path(), "blank-version-tool", "#!/bin/sh\nexit 0\n");
    let blank_name = blank.to_string_lossy().into_owned();

    assert_eq!(
        super::get_version("definitely-missing-vw-cli-discovery-tool", &["--version"]),
        None
    );
    assert_eq!(super::get_version(&blank_name, &["--version"]), None);
}

#[cfg(unix)]
#[test]
fn discover_additional_tools_respects_exclusion_and_deduplicates_existing_results() {
    let directory = tempfile::tempdir().expect("temporary CLI directory should be created");
    let included = executable_stub(
        directory.path(),
        "additional-tool",
        "#!/bin/sh\nprintf 'additional-tool 3.0\\n'\n",
    );
    let excluded = executable_stub(
        directory.path(),
        "excluded-tool",
        "#!/bin/sh\nprintf 'excluded-tool 4.0\\n'\n",
    );
    let included_name = included.to_string_lossy().into_owned();
    let excluded_name = excluded.to_string_lossy().into_owned();
    let additional = vec![included_name.clone(), included_name.clone(), excluded_name.clone()];
    let excluded = vec![excluded_name.clone()];

    let results = discover_cli_tools(&additional, &excluded);

    let matches = results.iter().filter(|cli| cli.name == included_name).collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].path, included);
    assert_eq!(matches[0].version.as_deref(), Some("additional-tool 3.0"));
    assert_eq!(matches[0].category, CliCategory::Build);
    assert!(!results.iter().any(|cli| cli.name == excluded_name));
}

#[cfg(unix)]
fn executable_stub(directory: &std::path::Path, name: &str, script: &str) -> std::path::PathBuf {
    let executable = directory.join(name);
    std::fs::write(&executable, script).expect("CLI stub should be written");
    let mut permissions =
        std::fs::metadata(&executable).expect("CLI stub metadata should be readable").permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&executable, permissions).expect("CLI stub should be executable");
    executable
}
