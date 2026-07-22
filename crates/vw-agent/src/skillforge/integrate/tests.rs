//! 技能集成模块测试套件
//!
//! 本模块为 `skillforge::integrate` 模块提供全面的单元测试覆盖，验证技能集成流程
//! 的核心功能，包括文件创建、路径安全处理和 TOML 格式转义等关键行为。
//!
//! # 测试范围
//!
//! - **集成功能测试**：验证技能集成本身能否正确创建必要的文件结构
//! - **安全防护测试**：确保路径处理不受目录遍历攻击影响
//! - **格式化测试**：验证 TOML 内容的转义和格式化正确性

use super::*;

/// 集成模块测试用例集合
///
/// 封装所有集成相关的测试函数，确保测试代码的组织性和可维护性。
/// 使用 `#[allow(dead_code)]` 标记，因为测试模块仅在测试构建中被引用。
#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::skillforge::scout::{ScoutResult, ScoutSource};
    use std::fs;

    /// 创建用于测试的示例候选技能数据
    ///
    /// 生成一个包含完整字段的 `ScoutResult` 实例，用于各个测试用例的输入数据。
    /// 该候选技能模拟一个来自 GitHub 的 Rust 语言项目，包含典型的元数据字段。
    ///
    /// # 返回值
    ///
    /// 返回一个预设好所有字段的 `ScoutResult` 实例，包含以下特征：
    /// - 名称：`test-skill`
    /// - URL：`https://github.com/user/test-skill`
    /// - 描述：`A test skill for unit tests`
    /// - 星标数：42
    /// - 语言：Rust
    /// - 来源：GitHub
    /// - 许可证：存在
    fn sample_candidate() -> ScoutResult {
        ScoutResult {
            name: "test-skill".into(),
            url: "https://github.com/user/test-skill".into(),
            description: "A test skill for unit tests".into(),
            stars: 42,
            language: Some("Rust".into()),
            updated_at: Some(Utc::now()),
            source: ScoutSource::GitHub,
            owner: "user".into(),
            has_license: true,
        }
    }

    /// 测试集成器是否正确创建必要的文件
    ///
    /// 验证当集成一个候选技能时，`Integrator` 能够在指定目录下创建预期的文件结构，
    /// 包括 `SKILL.toml` 配置文件和 `SKILL.md` 说明文档，并确保文件内容包含正确的元数据。
    ///
    /// # 测试步骤
    ///
    /// 1. 在系统临时目录下创建测试专用目录
    /// 2. 使用示例候选技能数据执行集成操作
    /// 3. 验证 `SKILL.toml` 和 `SKILL.md` 文件已创建
    /// 4. 验证 TOML 文件包含正确的名称和星标数
    /// 5. 验证 Markdown 文件包含正确的标题和描述
    /// 6. 清理测试目录
    ///
    /// # 异步说明
    ///
    /// 使用 `#[tokio::test]` 标记，因为文件读取操作采用异步 API 执行。
    #[tokio::test]
    async fn integrate_creates_files() {
        // 准备测试目录路径
        let tmp = std::env::temp_dir().join("vibewindow-test-integrate");
        // 清理可能存在的旧测试数据
        let _ = fs::remove_dir_all(&tmp);

        // 创建集成器实例并执行集成操作
        let integrator = Integrator::new(tmp.to_string_lossy().into_owned());
        let c = sample_candidate();
        let path = integrator.integrate(&c).unwrap();

        // 验证必需文件是否存在
        assert!(path.join("SKILL.toml").exists());
        assert!(path.join("SKILL.md").exists());

        // 验证 TOML 文件内容是否正确
        let toml = tokio::fs::read_to_string(path.join("SKILL.toml")).await.unwrap();
        assert!(toml.contains("name = \"test-skill\""));
        assert!(toml.contains("stars = 42"));

        // 验证 Markdown 文件内容是否正确
        let md = tokio::fs::read_to_string(path.join("SKILL.md")).await.unwrap();
        assert!(md.contains("# test-skill"));
        assert!(md.contains("A test skill for unit tests"));

        // 清理测试数据
        let _ = fs::remove_dir_all(&tmp);
    }

    /// 测试 TOML 转义函数对引号和控制字符的处理
    ///
    /// 验证 `escape_toml` 函数能够正确处理 TOML 格式中需要转义的特殊字符，
    /// 包括双引号、反斜杠和各种控制字符（换行、制表符、回车等）。
    ///
    /// # 测试用例
    ///
    /// - 双引号应被转义为 `\"`
    /// - 反斜杠应被转义为 `\\`
    /// - 换行符应被转义为 `\n`
    /// - 制表符应被转义为 `\t`
    /// - 回车符应被转义为 `\r`
    #[test]
    fn escape_toml_handles_quotes_and_control_chars() {
        assert_eq!(escape_toml(r#"say "hello""#), r#"say \"hello\""#);
        assert_eq!(escape_toml(r"back\slash"), r"back\\slash");
        assert_eq!(escape_toml("line\nbreak"), "line\\nbreak");
        assert_eq!(escape_toml("tab\there"), "tab\\there");
        assert_eq!(escape_toml("cr\rhere"), "cr\\rhere");
    }

    /// 测试路径清理函数对目录遍历攻击的拒绝
    ///
    /// 验证 `sanitize_path_component` 函数能够正确拒绝可能导致目录遍历攻击的
    /// 危险路径组件，包括相对路径标记、空字符串和空白字符串。
    ///
    /// # 安全考虑
    ///
    /// 路径遍历防护是防止恶意输入访问预期目录外文件的关键安全措施。
    /// 以下输入应被拒绝：
    /// - `..`：标准的父目录引用
    /// - `...`：变体形式的父目录引用
    /// - 空字符串：无效的路径组件
    /// - 仅包含空白的字符串：无效的路径组件
    #[test]
    fn sanitize_rejects_traversal() {
        assert!(sanitize_path_component("..").is_err());
        assert!(sanitize_path_component("...").is_err());
        assert!(sanitize_path_component("").is_err());
        assert!(sanitize_path_component("  ").is_err());
    }

    /// 测试路径清理函数对路径分隔符的替换
    ///
    /// 验证 `sanitize_path_component` 函数能够将各种路径分隔符和安全字符
    /// 替换为安全的下划线字符，防止路径注入攻击。
    ///
    /// # 处理的分隔符
    ///
    /// - 正斜杠 `/`：Unix 风格路径分隔符
    /// - 反斜杠 `\`：Windows 风格路径分隔符
    /// - 空字符 `\0`：C 风格字符串终止符
    ///
    /// 所有这些字符都会被统一替换为下划线 `_`，确保路径组件的跨平台安全性。
    #[test]
    fn sanitize_replaces_separators() {
        let s = sanitize_path_component("foo/bar\\baz\0qux").unwrap();
        assert!(!s.contains('/'));
        assert!(!s.contains('\\'));
        assert!(!s.contains('\0'));
        assert_eq!(s, "foo_bar_baz_qux");
    }

    /// 测试路径清理函数对首尾点号的修剪
    ///
    /// 验证 `sanitize_path_component` 函数能够正确移除路径组件首尾的点号，
    /// 防止创建隐藏文件（Unix 系统中以点号开头）或产生不规范的文件名。
    ///
    /// # 处理逻辑
    ///
    /// - 开头的点号会被移除（避免创建隐藏文件）
    /// - 结尾的点号会被移除（符合跨平台文件命名规范）
    /// - 中间的点号会被保留（允许合法的文件扩展名）
    ///
    /// # 示例
    ///
    /// 输入 `.hidden.` 将被处理为 `hidden`
    #[test]
    fn sanitize_trims_dots() {
        let s = sanitize_path_component(".hidden.").unwrap();
        assert_eq!(s, "hidden");
    }
}

fn candidate_with_missing_optional_fields() -> crate::skillforge::scout::ScoutResult {
    crate::skillforge::scout::ScoutResult {
        name: "quoted\"skill".into(),
        url: "https://github.com/user/quoted".into(),
        description: "Line one\nLine two".into(),
        stars: 7,
        language: None,
        updated_at: None,
        source: crate::skillforge::scout::ScoutSource::GitHub,
        owner: "user".into(),
        has_license: false,
    }
}

#[test]
fn generate_toml_uses_defaults_and_escapes_strings() {
    let integrator = Integrator::new("unused".to_string());
    let toml = integrator.generate_toml(&candidate_with_missing_optional_fields());

    assert!(toml.contains("name = \"quoted\\\"skill\""));
    assert!(toml.contains("description = \"Line one\\nLine two\""));
    assert!(toml.contains("language = \"unknown\""));
    assert!(toml.contains("license = false"));
    assert!(toml.contains("updated_at = \"unknown\""));
}

#[test]
fn generate_md_uses_unknowns_for_missing_metadata() {
    let integrator = Integrator::new("unused".to_string());
    let md = integrator.generate_md(&candidate_with_missing_optional_fields());

    assert!(md.contains("# quoted\"skill"));
    assert!(md.contains("**Language**: unknown"));
    assert!(md.contains("**License**: unknown"));
    assert!(md.contains("Line one\nLine two"));
}

#[test]
fn integrate_replaces_separators_in_skill_directory_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let integrator = Integrator::new(dir.path().to_string_lossy().to_string());
    let mut candidate = candidate_with_missing_optional_fields();
    candidate.name = "one/two\\three".to_string();

    let path = integrator.integrate(&candidate).unwrap();

    assert_eq!(path.file_name().and_then(|name| name.to_str()), Some("one_two_three"));
    assert!(path.join("SKILL.toml").is_file());
    assert!(path.join("SKILL.md").is_file());
}

#[test]
fn integrate_reports_skill_directory_creation_failure_with_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output_file = dir.path().join("output-file");
    std::fs::write(&output_file, "occupied").unwrap();
    let integrator = Integrator::new(output_file.to_string_lossy().to_string());

    let error = integrator
        .integrate(&candidate_with_missing_optional_fields())
        .expect_err("a file cannot contain a skill directory");

    assert!(error.to_string().contains("Failed to create dir"));
    assert!(error.to_string().contains("quoted\"skill"));
}

#[test]
fn integrate_reports_toml_write_failure_with_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skill_dir = dir.path().join("quoted\"skill");
    std::fs::create_dir_all(skill_dir.join("SKILL.toml")).unwrap();
    let integrator = Integrator::new(dir.path().to_string_lossy().to_string());

    let error = integrator
        .integrate(&candidate_with_missing_optional_fields())
        .expect_err("a directory cannot be overwritten by TOML content");

    assert!(error.to_string().contains("Failed to write"));
    assert!(error.to_string().contains("SKILL.toml"));
}

#[test]
fn integrate_reports_markdown_write_failure_with_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skill_dir = dir.path().join("quoted\"skill");
    std::fs::create_dir_all(skill_dir.join("SKILL.md")).unwrap();
    let integrator = Integrator::new(dir.path().to_string_lossy().to_string());

    let error = integrator
        .integrate(&candidate_with_missing_optional_fields())
        .expect_err("a directory cannot be overwritten by Markdown content");

    assert!(error.to_string().contains("Failed to write"));
    assert!(error.to_string().contains("SKILL.md"));
    assert!(skill_dir.join("SKILL.toml").is_file());
}
