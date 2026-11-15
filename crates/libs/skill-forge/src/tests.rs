//! SkillForge 模块单元测试
//!
//! 本模块包含对 SkillForge 技能锻造系统的测试用例，主要验证：
//! - 禁用状态下的锻造行为
//! - 默认配置值的正确性
//!
//! 测试覆盖核心配置和运行时行为的边界条件

use super::*;

/// SkillForge 测试套件
///
/// 包含针对 SkillForge 核心功能的单元测试，验证配置加载、
/// 锻造流程和边界条件处理的正确性
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试禁用状态下的锻造行为
    ///
    /// 当 SkillForge 被禁用时（enabled = false），调用 forge() 方法
    /// 应返回空的锻造报告，不执行任何实际的发现或集成操作。
    ///
    /// # 验证点
    /// - 发现的技能数量应为 0
    /// - 自动集成的技能数量应为 0
    /// - 不应发生错误
    #[tokio::test]
    async fn disabled_forge_returns_empty_report() {
        // 创建禁用状态的配置
        let cfg = SkillForgeConfig { enabled: false, ..Default::default() };

        // 实例化 SkillForge
        let forge = SkillForge::new(cfg);

        // 执行锻造操作
        let report = forge.forge().await.unwrap();

        // 验证返回空报告
        assert_eq!(report.discovered, 0);
        assert_eq!(report.auto_integrated, 0);
    }

    /// 测试默认配置值
    ///
    /// 验证 SkillForgeConfig 的默认值符合预期设计：
    /// - 默认禁用（enabled = false）
    /// - 默认启用自动集成（auto_integrate = true）
    /// - 默认扫描间隔为 24 小时
    /// - 默认最低质量分数为 0.7
    /// - 默认技能源为 ["github", "clawhub"]
    #[test]
    fn default_config_values() {
        // 获取默认配置
        let cfg = SkillForgeConfig::default();

        // 验证默认禁用状态
        assert!(!cfg.enabled);

        // 验证默认启用自动集成
        assert!(cfg.auto_integrate);

        // 验证默认扫描间隔（24 小时）
        assert_eq!(cfg.scan_interval_hours, 24);

        // 验证默认最低质量分数（0.7，使用浮点数容差比较）
        assert!((cfg.min_score - 0.7).abs() < f64::EPSILON);

        // 验证默认技能源列表
        assert_eq!(cfg.sources, vec!["github", "clawhub"]);
    }
}

#[test]
fn debug_redacts_github_token() {
    let cfg =
        SkillForgeConfig { github_token: Some("ghp_secret".to_string()), ..Default::default() };
    let rendered = format!("{cfg:?}");

    assert!(rendered.contains("***"));
    assert!(!rendered.contains("ghp_secret"));
}

#[test]
fn config_deserialization_fills_defaults() {
    let cfg: SkillForgeConfig =
        serde_json::from_str(r#"{"enabled":true,"github_token":"token"}"#).unwrap();

    assert!(cfg.enabled);
    assert!(cfg.auto_integrate);
    assert_eq!(cfg.sources, vec!["github", "clawhub"]);
    assert_eq!(cfg.scan_interval_hours, 24);
    assert!((cfg.min_score - 0.7).abs() < f64::EPSILON);
    assert_eq!(cfg.output_dir, "./skills");
    assert_eq!(cfg.github_token.as_deref(), Some("token"));
}

#[test]
fn config_deserialization_rejects_invalid_field_types() {
    let error = serde_json::from_str::<SkillForgeConfig>(
        r#"{"enabled":"yes","scan_interval_hours":"often","min_score":"high"}"#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("invalid type"), "{error}");
}

#[tokio::test]
async fn enabled_unimplemented_sources_return_empty_report_without_network() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cfg = SkillForgeConfig {
        enabled: true,
        sources: vec!["clawhub".to_string(), "huggingface".to_string()],
        output_dir: dir.path().to_string_lossy().to_string(),
        ..Default::default()
    };

    let report = SkillForge::new(cfg).forge().await.unwrap();

    assert_eq!(report.discovered, 0);
    assert_eq!(report.evaluated, 0);
    assert_eq!(report.auto_integrated, 0);
    assert_eq!(report.manual_review, 0);
    assert_eq!(report.skipped, 0);
}

#[test]
fn empty_config_deserialization_matches_default() {
    let cfg: SkillForgeConfig = serde_json::from_str("{}").unwrap();

    assert!(!cfg.enabled);
    assert!(cfg.auto_integrate);
    assert_eq!(cfg.sources, vec!["github", "clawhub"]);
    assert_eq!(cfg.scan_interval_hours, 24);
    assert!((cfg.min_score - 0.7).abs() < f64::EPSILON);
    assert_eq!(cfg.github_token, None);
    assert_eq!(cfg.output_dir, "./skills");
}

#[test]
fn explicit_config_json_round_trip_preserves_all_fields() {
    let cfg = SkillForgeConfig {
        enabled: true,
        auto_integrate: false,
        sources: vec!["huggingface".to_string()],
        scan_interval_hours: 6,
        min_score: 0.85,
        github_token: Some("placeholder-token".to_string()),
        output_dir: "custom-skills".to_string(),
    };

    let encoded = serde_json::to_string(&cfg).unwrap();
    let decoded: SkillForgeConfig = serde_json::from_str(&encoded).unwrap();

    assert!(decoded.enabled);
    assert!(!decoded.auto_integrate);
    assert_eq!(decoded.sources, vec!["huggingface"]);
    assert_eq!(decoded.scan_interval_hours, 6);
    assert!((decoded.min_score - 0.85).abs() < f64::EPSILON);
    assert_eq!(decoded.github_token.as_deref(), Some("placeholder-token"));
    assert_eq!(decoded.output_dir, "custom-skills");
}

#[test]
fn debug_without_github_token_reports_none() {
    let rendered = format!("{:?}", SkillForgeConfig::default());

    assert!(rendered.contains("github_token: None"));
    assert!(rendered.contains("output_dir: \"./skills\""));
}

#[tokio::test]
async fn disabled_forge_ignores_external_configuration_and_returns_fully_empty_report() {
    let cfg = SkillForgeConfig {
        enabled: false,
        sources: vec!["unknown-network-source".to_string()],
        output_dir: "unused-output-path".to_string(),
        ..Default::default()
    };

    let report = SkillForge::new(cfg).forge().await.unwrap();

    assert_eq!(report.discovered, 0);
    assert_eq!(report.evaluated, 0);
    assert_eq!(report.auto_integrated, 0);
    assert_eq!(report.manual_review, 0);
    assert_eq!(report.skipped, 0);
    assert!(report.results.is_empty());
}

#[tokio::test]
async fn enabled_forge_with_no_sources_returns_fully_empty_report() {
    let cfg = SkillForgeConfig { enabled: true, sources: Vec::new(), ..Default::default() };

    let report = SkillForge::new(cfg).forge().await.unwrap();

    assert_eq!(report.discovered, 0);
    assert_eq!(report.evaluated, 0);
    assert_eq!(report.auto_integrated, 0);
    assert_eq!(report.manual_review, 0);
    assert_eq!(report.skipped, 0);
    assert!(report.results.is_empty());
}

#[test]
fn forge_report_json_round_trip_preserves_summary_and_detailed_result() {
    use super::evaluate::Scores;

    let report = ForgeReport {
        discovered: 1,
        evaluated: 1,
        auto_integrated: 0,
        manual_review: 1,
        skipped: 0,
        results: vec![EvalResult {
            candidate: ScoutResult {
                name: "review-me".to_string(),
                url: "https://example.invalid/review-me".to_string(),
                description: "fixture candidate".to_string(),
                stars: 12,
                language: Some("Rust".to_string()),
                updated_at: None,
                source: ScoutSource::ClawHub,
                owner: "fixture-owner".to_string(),
                has_license: true,
            },
            scores: Scores { compatibility: 0.8, quality: 0.6, security: 0.9 },
            total_score: 0.765,
            recommendation: Recommendation::Manual,
        }],
    };

    let encoded = serde_json::to_string(&report).unwrap();
    let decoded: ForgeReport = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.discovered, 1);
    assert_eq!(decoded.evaluated, 1);
    assert_eq!(decoded.auto_integrated, 0);
    assert_eq!(decoded.manual_review, 1);
    assert_eq!(decoded.skipped, 0);
    assert_eq!(decoded.results.len(), 1);
    let result = &decoded.results[0];
    assert_eq!(result.candidate.name, "review-me");
    assert_eq!(result.candidate.source, ScoutSource::ClawHub);
    assert!((result.total_score - 0.765).abs() < f64::EPSILON);
    assert_eq!(result.recommendation, Recommendation::Manual);
}

#[test]
fn report_from_candidates_dedups_and_counts_auto_manual_and_skip_results() {
    let dir = tempfile::tempdir().expect("temp dir");
    let forge = SkillForge::new(SkillForgeConfig {
        enabled: true,
        output_dir: dir.path().to_string_lossy().to_string(),
        ..Default::default()
    });

    let auto = candidate("auto-skill", "https://example.invalid/auto", "Rust", 2_000, true);
    let mut duplicate_auto = auto.clone();
    duplicate_auto.name = "duplicate-auto-skill".to_string();
    let manual = candidate("manual-skill", "https://example.invalid/manual", "Python", 10, false);
    let skip = ScoutResult {
        name: "skip-skill".to_string(),
        url: "https://example.invalid/skip".to_string(),
        description: "contains malware marker".to_string(),
        stars: 0,
        language: None,
        updated_at: None,
        source: ScoutSource::ClawHub,
        owner: "fixture-owner".to_string(),
        has_license: false,
    };

    let report =
        forge.report_from_candidates(vec![auto, duplicate_auto, manual, skip]).expect("report");

    assert_eq!(report.discovered, 3);
    assert_eq!(report.evaluated, 3);
    assert_eq!(report.auto_integrated, 1);
    assert_eq!(report.manual_review, 1);
    assert_eq!(report.skipped, 1);
    assert!(dir.path().join("auto-skill").join("SKILL.toml").exists());
    assert!(report.results.iter().any(|result| result.recommendation == Recommendation::Manual));
    assert!(report.results.iter().any(|result| result.recommendation == Recommendation::Skip));
}

#[test]
fn report_from_candidates_marks_auto_candidates_for_review_when_auto_integrate_is_disabled() {
    let dir = tempfile::tempdir().expect("temp dir");
    let forge = SkillForge::new(SkillForgeConfig {
        enabled: true,
        auto_integrate: false,
        output_dir: dir.path().to_string_lossy().to_string(),
        ..Default::default()
    });

    let report = forge
        .report_from_candidates(vec![candidate(
            "manual-auto-skill",
            "https://example.invalid/manual-auto",
            "Rust",
            2_000,
            true,
        )])
        .expect("report");

    assert_eq!(report.discovered, 1);
    assert_eq!(report.auto_integrated, 0);
    assert_eq!(report.manual_review, 1);
    assert!(!dir.path().join("manual-auto-skill").exists());
}

#[test]
fn report_from_candidates_continues_when_auto_integration_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let forge = SkillForge::new(SkillForgeConfig {
        enabled: true,
        output_dir: dir.path().to_string_lossy().to_string(),
        ..Default::default()
    });

    let report = forge
        .report_from_candidates(vec![candidate(
            "..",
            "https://example.invalid/bad-name",
            "Rust",
            2_000,
            true,
        )])
        .expect("report");

    assert_eq!(report.discovered, 1);
    assert_eq!(report.evaluated, 1);
    assert_eq!(report.auto_integrated, 0);
    assert_eq!(report.manual_review, 0);
    assert_eq!(report.skipped, 0);
    assert_eq!(report.results[0].recommendation, Recommendation::Auto);
}

fn candidate(name: &str, url: &str, language: &str, stars: u64, has_license: bool) -> ScoutResult {
    ScoutResult {
        name: name.to_string(),
        url: url.to_string(),
        description: "fixture candidate".to_string(),
        stars,
        language: Some(language.to_string()),
        updated_at: Some(chrono::Utc::now()),
        source: ScoutSource::GitHub,
        owner: "fixture-owner".to_string(),
        has_license,
    }
}
