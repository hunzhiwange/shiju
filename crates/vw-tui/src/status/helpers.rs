//! `/status` 展示所需的模型、账户、路径、套餐与时间格式化辅助函数。
//! 这些函数只整理已提供的数据，不读取认证信息或主动查询运行时。

use crate::exec_command::relativize_to_home;
use crate::legacy_core::config::Config;
use crate::status::StatusAccountDisplay;
use crate::text_formatting;
use crate::width::display_width;
use chrono::DateTime;
use chrono::Local;
use codex_protocol::account::PlanType;
use codex_utils_path_uri::PathConvention;
use codex_utils_path_uri::PathUri;
use std::path::Path;

fn normalize_agents_display_path(path: &Path) -> String {
    dunce::simplified(path).display().to_string()
}

/// 组合模型名称与推理强度、摘要设置等紧凑详情。
pub(crate) fn compose_model_display(
    model_name: &str,
    entries: &[(&str, String)],
) -> (String, Vec<String>) {
    let mut details: Vec<String> = Vec::new();
    if let Some((_, effort)) = entries.iter().find(|(k, _)| *k == "reasoning effort") {
        details.push(format!("reasoning {}", effort.to_ascii_lowercase()));
    }
    if let Some((_, summary)) = entries.iter().find(|(k, _)| *k == "reasoning summaries") {
        let summary = summary.trim();
        if summary.eq_ignore_ascii_case("none") || summary.eq_ignore_ascii_case("off") {
            details.push("summaries off".to_string());
        } else if !summary.is_empty() {
            details.push(format!("summaries {}", summary.to_ascii_lowercase()));
        }
    }

    (model_name.to_string(), details)
}

/// 把指令来源整理为相对当前目录或保留外来路径约定的摘要。
pub(crate) fn compose_agents_summary(config: &Config, paths: &[PathUri]) -> String {
    let mut rels: Vec<String> = Vec::new();

    for path in paths {
        // TODO(anp): Rationalize instruction-source summaries with the TUI's broader foreign-path
        // display strategy once other status surfaces can retain environment-native paths.
        // TODO(anp)：待其他状态界面能保留环境原生路径后，再统一指令来源摘要与外来路径展示策略。
        // 原生路径尽量相对工作目录展示；无法安全转换的外来路径保留其推断约定。
        if path.infer_path_convention() != Some(PathConvention::native()) {
            rels.push(path.inferred_native_path_string());
            continue;
        }
        let Ok(p) = path.to_abs_path() else {
            rels.push(path.inferred_native_path_string());
            continue;
        };
        let p = p.as_path();
        let file_name = p
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "<unknown>".to_string());
        let display = if let Some(parent) = p.parent() {
            if parent == config.cwd.as_path() {
                file_name.clone()
            } else {
                let mut cur = config.cwd.as_path();
                let mut ups = 0usize;
                let mut reached = false;
                while let Some(c) = cur.parent() {
                    if cur == parent {
                        reached = true;
                        break;
                    }
                    cur = c;
                    ups += 1;
                }
                if reached {
                    let up = format!("..{}", std::path::MAIN_SEPARATOR);
                    format!("{}{}", up.repeat(ups), file_name)
                } else if let Ok(stripped) = p.strip_prefix(&config.cwd) {
                    normalize_agents_display_path(stripped)
                } else {
                    normalize_agents_display_path(p)
                }
            }
        } else {
            normalize_agents_display_path(p)
        };
        rels.push(display);
    }

    if rels.is_empty() {
        "<none>".to_string()
    } else {
        rels.join(", ")
    }
}

/// 克隆已经脱敏的账户展示值，不解析账户或凭据。
pub(crate) fn compose_account_display(
    account_display: Option<&StatusAccountDisplay>,
) -> Option<StatusAccountDisplay> {
    account_display.cloned()
}

/// 把协议套餐枚举映射为产品界面使用的名称。
pub(crate) fn plan_type_display_name(plan_type: PlanType) -> String {
    if plan_type == PlanType::EnterpriseCbpAutomation {
        "Enterprise (Automation)".to_string()
    } else if plan_type.is_team_like() {
        "Business".to_string()
    } else if plan_type.is_business_like() {
        "Enterprise".to_string()
    } else if plan_type == PlanType::ProLite {
        "Pro Lite".to_string()
    } else if plan_type == PlanType::EduPlus {
        "Edu Plus".to_string()
    } else if plan_type == PlanType::EduPro {
        "Edu Pro".to_string()
    } else {
        title_case(format!("{plan_type:?}").as_str())
    }
}

/// 以最多两位小数把非负令牌数压缩为 K、M、B 或 T 单位。
pub(crate) fn format_tokens_compact(value: i64) -> String {
    let value = value.max(0);
    if value == 0 {
        return "0".to_string();
    }
    if value < 1_000 {
        return value.to_string();
    }

    let value_f64 = value as f64;
    let (scaled, suffix) = if value >= 1_000_000_000_000 {
        (value_f64 / 1_000_000_000_000.0, "T")
    } else if value >= 1_000_000_000 {
        (value_f64 / 1_000_000_000.0, "B")
    } else if value >= 1_000_000 {
        (value_f64 / 1_000_000.0, "M")
    } else {
        (value_f64 / 1_000.0, "K")
    };

    let decimals = if scaled < 10.0 {
        2
    } else if scaled < 100.0 {
        1
    } else {
        0
    };

    let mut formatted = format!("{scaled:.decimals$}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }

    format!("{formatted}{suffix}")
}

/// 优先相对主目录展示路径，并按可选终端宽度从中间安全截断。
pub(crate) fn format_directory_display(directory: &Path, max_width: Option<usize>) -> String {
    let formatted = if let Some(rel) = relativize_to_home(directory) {
        if rel.as_os_str().is_empty() {
            "~".to_string()
        } else {
            format!("~{}{}", std::path::MAIN_SEPARATOR, rel.display())
        }
    } else {
        directory.display().to_string()
    };

    if let Some(max_width) = max_width {
        if max_width == 0 {
            return String::new();
        }
        if display_width(&formatted) > max_width {
            return text_formatting::center_truncate_path(&formatted, max_width);
        }
    }

    formatted
}

/// 同日只显示本地时间，跨日时附加日与月份。
pub(crate) fn format_reset_timestamp(dt: DateTime<Local>, captured_at: DateTime<Local>) -> String {
    let time = dt.format("%H:%M").to_string();
    if dt.date_naive() == captured_at.date_naive() {
        time
    } else {
        format!("{time} on {}", dt.format("%-d %b"))
    }
}

fn title_case(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let rest = chars.as_str().to_ascii_lowercase();
    first.to_uppercase().collect::<String>() + &rest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_core::config::ConfigBuilder;
    use codex_utils_absolute_path::test_support::PathBufExt;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    async fn test_config(codex_home: &TempDir, cwd: &TempDir) -> Config {
        ConfigBuilder::default()
            .codex_home(codex_home.path().to_path_buf())
            .fallback_cwd(Some(cwd.path().to_path_buf()))
            .build()
            .await
            .expect("load config")
    }

    // 生成覆盖特殊套餐重命名和普通标题化回退的表驱动用例。
    #[test]
    fn plan_type_display_name_remaps_display_labels() {
        let cases = [
            (PlanType::Free, "Free"),
            (PlanType::Go, "Go"),
            (PlanType::Plus, "Plus"),
            (PlanType::Pro, "Pro"),
            (PlanType::ProLite, "Pro Lite"),
            (PlanType::Team, "Business"),
            (PlanType::SelfServeBusinessProLite, "Business"),
            (PlanType::SelfServeBusinessUsageBased, "Business"),
            (PlanType::Business, "Enterprise"),
            (PlanType::EnterpriseCbpAutomation, "Enterprise (Automation)"),
            (PlanType::EnterpriseCbpUsageBased, "Enterprise"),
            (PlanType::Enterprise, "Enterprise"),
            (PlanType::Edu, "Edu"),
            (PlanType::Unknown, "Unknown"),
        ];

        for (plan_type, expected) in cases {
            assert_eq!(plan_type_display_name(plan_type), expected);
        }
        insta::assert_snapshot!(
            "education_plan_display_names",
            [PlanType::Edu, PlanType::EduPlus, PlanType::EduPro]
                .map(plan_type_display_name)
                .join("\n")
        );
    }

    #[test]
    // 验证路径截断不会拆散半角字符与其浊音组合。
    fn format_directory_display_truncates_halfwidth_sound_marks() {
        let directory = Path::new("workspace").join("ｶﾞ").join("project");
        let max_width = display_width(directory.to_string_lossy().as_ref()) - 1;
        let formatted = format_directory_display(&directory, Some(max_width));

        insta::assert_snapshot!(formatted.replace('\\', "/"), @"workspace/…/project");
    }

    #[tokio::test]
    // 验证全局指令文件在工作目录外仍可读地展示。
    async fn compose_agents_summary_includes_global_agents_path() {
        let codex_home = TempDir::new().expect("temp codex home");
        let cwd = TempDir::new().expect("temp cwd");
        let global_agents_path = codex_home.path().join("global.md");
        let config = test_config(&codex_home, &cwd).await;

        assert_eq!(
            compose_agents_summary(
                &config,
                &[PathUri::from_abs_path(&global_agents_path.abs())]
            ),
            format_directory_display(&global_agents_path, /*max_width*/ None)
        );
    }

    #[tokio::test]
    // 验证全局指令覆盖文件沿用同一安全路径格式。
    async fn compose_agents_summary_names_global_agents_override() {
        let codex_home = TempDir::new().expect("temp codex home");
        let cwd = TempDir::new().expect("temp cwd");
        let override_path = codex_home.path().join("override.md");
        let config = test_config(&codex_home, &cwd).await;

        assert_eq!(
            compose_agents_summary(&config, &[PathUri::from_abs_path(&override_path.abs())]),
            format_directory_display(&override_path, /*max_width*/ None)
        );
    }

    #[tokio::test]
    // 验证原生来源相对显示，而外来路径保留其平台约定。
    async fn compose_agents_summary_shows_relative_native_and_full_foreign_paths() {
        let codex_home = TempDir::new().expect("temp codex home");
        let cwd = TempDir::new().expect("temp cwd");
        let config = test_config(&codex_home, &cwd).await;
        let native_source = PathUri::from_abs_path(&config.cwd.join("AGENTS.md"));
        let foreign_source = if cfg!(windows) {
            PathUri::parse("file:///remote%20workspace/AGENTS.md")
                .expect("POSIX instruction source")
        } else {
            PathUri::parse("file:///C:/remote%20workspace/AGENTS.md")
                .expect("Windows instruction source")
        };

        let summary = compose_agents_summary(&config, &[native_source, foreign_source]);
        if cfg!(windows) {
            insta::assert_snapshot!(summary, @r"AGENTS.md, /remote workspace/AGENTS.md");
        } else {
            insta::assert_snapshot!(summary, @r"AGENTS.md, C:\remote workspace\AGENTS.md");
        }
    }

    #[tokio::test]
    // 验证摘要保持调用方提供的全局到项目顺序。
    async fn compose_agents_summary_orders_global_before_project_agents() {
        let codex_home = TempDir::new().expect("temp codex home");
        let cwd = TempDir::new().expect("temp cwd");
        let global_agents_path = codex_home.path().join("global.md");
        let project_agents_path = cwd.path().join("project.md");
        let config = test_config(&codex_home, &cwd).await;

        let summary = compose_agents_summary(
            &config,
            &[
                PathUri::from_abs_path(&global_agents_path.clone().abs()),
                PathUri::from_abs_path(&project_agents_path.clone().abs()),
            ],
        );
        let mut paths = summary.split(", ");
        assert_eq!(
            paths.next(),
            Some(format_directory_display(&global_agents_path, /*max_width*/ None).as_str())
        );
        let project_path = paths.next().expect("project agents path");
        assert!(project_path.ends_with("project.md"));
        assert_eq!(paths.next(), None);
    }
}
