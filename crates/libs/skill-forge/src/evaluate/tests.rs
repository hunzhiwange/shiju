//! 技能评估模块的单元测试
//!
//! 本模块包含对 `Evaluator` 及相关评分逻辑的单元测试，用于验证技能候选项的
//! 质量评估、安全检测和推荐决策等功能。
//!
//! # 测试覆盖范围
//!
//! - 高质量 Rust 仓库的自动推荐逻辑
//! - 低质量/无许可证仓库的手动审核决策
//! - 恶意模式检测对安全分数的影响
//! - 加权分数计算的准确性
//! - 误报防止（如 "hackathon" 不应被标记为 "hack"）

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::scout::{ScoutResult, ScoutSource};

    /// 创建测试用的技能候选项
    ///
    /// 该辅助函数用于快速构造具有指定属性的 `ScoutResult` 实例，
    /// 便于在各个测试用例中使用。
    ///
    /// # 参数
    ///
    /// - `stars`: 仓库的星标数量，用于质量评分
    /// - `lang`: 编程语言（可选），用于兼容性评估
    /// - `has_license`: 是否包含许可证，影响质量分数
    ///
    /// # 返回值
    ///
    /// 返回一个具有默认值的 `ScoutResult` 实例，其中：
    /// - 名称为 "test-skill"
    /// - URL 为 GitHub 测试地址
    /// - 描述为 "A test skill"
    /// - 更新时间设为当前时间
    /// - 来源为 GitHub
    /// - 所有者为 "test"
    fn make_candidate(stars: u64, lang: Option<&str>, has_license: bool) -> ScoutResult {
        ScoutResult {
            name: "test-skill".into(),
            url: "https://github.com/test/test-skill".into(),
            description: "A test skill".into(),
            stars,
            language: lang.map(String::from),
            updated_at: Some(chrono::Utc::now()),
            source: ScoutSource::GitHub,
            owner: "test".into(),
            has_license,
        }
    }

    /// 测试高质量 Rust 仓库应获得自动推荐
    ///
    /// 验证一个具有以下特征的仓库应获得 Auto 推荐级别：
    /// - 星标数较高（500）
    /// - 使用 Rust 语言（与项目技术栈兼容）
    /// - 包含许可证
    ///
    /// 预期结果：总分 >= 0.7，推荐级别为 Auto
    #[test]
    fn high_quality_rust_repo_gets_auto() {
        let eval = Evaluator::new(0.7);
        let c = make_candidate(500, Some("Rust"), true);
        let res = eval.evaluate(c);
        assert!(res.total_score >= 0.7, "score: {}", res.total_score);
        assert_eq!(res.recommendation, Recommendation::Auto);
    }

    /// 测试低星标且无许可证的仓库应获得手动审核或跳过推荐
    ///
    /// 验证一个具有以下特征的仓库不应获得 Auto 推荐级别：
    /// - 星标数极低（1）
    /// - 无编程语言信息
    /// - 无许可证
    ///
    /// 预期结果：总分 < 0.7，推荐级别不是 Auto
    #[test]
    fn low_star_no_license_gets_manual_or_skip() {
        let eval = Evaluator::new(0.7);
        let c = make_candidate(1, None, false);
        let res = eval.evaluate(c);
        assert!(res.total_score < 0.7, "score: {}", res.total_score);
        assert_ne!(res.recommendation, Recommendation::Auto);
    }

    /// 测试恶意模式检测会显著降低安全分数
    ///
    /// 验证名称中包含恶意关键词的仓库会被安全检测识别，
    /// 即使其他指标良好（高星标、有许可证），安全分数也会大幅下降。
    ///
    /// 测试场景：
    /// - 基础分数（高质量仓库）：约 0.5
    /// - 许可证加分：+0.3
    /// - 恶意模式惩罚：-0.5
    /// - 新近度加分：+0.2
    /// - 预期最终安全分数：<= 0.5
    #[test]
    fn bad_pattern_tanks_security() {
        let eval = Evaluator::new(0.7);
        let mut c = make_candidate(1000, Some("Rust"), true);
        c.name = "malware-skill".into();
        let res = eval.evaluate(c);
        // 分数计算：0.5（基础）+ 0.3（许可证）- 0.5（恶意模式）+ 0.2（新近度）= 0.5
        assert!(res.scores.security <= 0.5, "security: {}", res.scores.security);
    }

    /// 测试加权总分计算的准确性
    ///
    /// 验证 `Scores::total()` 方法正确计算各维度的加权总和：
    /// - 当所有分数为满分（1.0）时，总分应接近 1.0
    /// - 当所有分数为零分（0.0）时，总分应接近 0.0
    ///
    /// 使用浮点数精度比较（f64::EPSILON）来处理浮点运算误差
    #[test]
    fn scores_total_weighted() {
        let s = Scores { compatibility: 1.0, quality: 1.0, security: 1.0 };
        assert!((s.total() - 1.0).abs() < f64::EPSILON);

        let s2 = Scores { compatibility: 0.0, quality: 0.0, security: 0.0 };
        assert!((s2.total()).abs() < f64::EPSILON);
    }

    /// 测试 "hackathon" 不应被误报为恶意模式
    ///
    /// 验证安全检测的精确性：
    /// - 包含 "hack" 子串的合法词汇（如 "hackathon"、"lifehacks"）不应触发惩罚
    /// - 这确保了模式匹配不会产生过多误报
    ///
    /// 预期结果：安全分数 >= 0.5（未受惩罚）
    #[test]
    fn hackathon_not_flagged_as_bad() {
        let eval = Evaluator::new(0.7);
        let mut c = make_candidate(500, Some("Rust"), true);
        c.name = "hackathon-tools".into();
        c.description = "Tools for hackathons and lifehacks".into();
        let res = eval.evaluate(c);
        // "hack" 不应匹配 "hackathon" 或 "lifehacks" 中的子串
        assert!(res.scores.security >= 0.5, "security: {}", res.scores.security);
    }

    /// 测试精确的恶意模式 "hack" 应被正确标记
    ///
    /// 验证安全检测能正确识别真实的恶意模式：
    /// - 名称中包含独立的 "hack" 关键词（如 "hack-tool"）应触发惩罚
    /// - 同时测试无许可证和无更新时间的综合影响
    ///
    /// 测试场景：
    /// - 基础分数：0.5
    /// - 无许可证：+0.0
    /// - 恶意模式惩罚：-0.5
    /// - 无更新时间（不新鲜）：+0.0
    /// - 预期最终安全分数：< 0.5
    #[test]
    fn exact_hack_is_flagged() {
        let eval = Evaluator::new(0.7);
        let mut c = make_candidate(500, Some("Rust"), false);
        c.name = "hack-tool".into();
        c.updated_at = None;
        let res = eval.evaluate(c);
        // 分数计算：0.5（基础）+ 0.0（无许可证）- 0.5（恶意模式）+ 0.0（无新近度）= 0.0
        assert!(res.scores.security < 0.5, "security: {}", res.scores.security);
    }
}

fn candidate_for_boundaries(
    stars: u64,
    language: Option<&str>,
    has_license: bool,
) -> crate::scout::ScoutResult {
    crate::scout::ScoutResult {
        name: "boundary-skill".into(),
        url: "https://github.com/test/boundary-skill".into(),
        description: "A boundary test skill".into(),
        stars,
        language: language.map(String::from),
        updated_at: Some(chrono::Utc::now()),
        source: crate::scout::ScoutSource::GitHub,
        owner: "test".into(),
        has_license,
    }
}

#[test]
fn compatibility_scores_language_tiers_and_quality_caps() {
    let eval = Evaluator::new(0.7);

    assert_eq!(eval.score_compatibility(&candidate_for_boundaries(0, Some("Rust"), true)), 1.0);
    assert_eq!(
        eval.score_compatibility(&candidate_for_boundaries(0, Some("TypeScript"), true)),
        0.6
    );
    assert_eq!(eval.score_compatibility(&candidate_for_boundaries(0, Some("Go"), true)), 0.3);
    assert_eq!(eval.score_compatibility(&candidate_for_boundaries(0, None, true)), 0.2);

    assert_eq!(eval.score_quality(&candidate_for_boundaries(0, Some("Rust"), true)), 0.0);
    assert_eq!(eval.score_quality(&candidate_for_boundaries(100_000, Some("Rust"), true)), 1.0);
}

#[test]
fn recommendation_boundaries_cover_manual_and_skip() {
    let eval = Evaluator::new(0.9);

    let manual = eval.evaluate(candidate_for_boundaries(1, Some("Python"), true));
    assert_eq!(manual.recommendation, Recommendation::Manual);

    let mut weak = candidate_for_boundaries(0, None, false);
    weak.updated_at = None;
    let skipped = eval.evaluate(weak);
    assert_eq!(skipped.recommendation, Recommendation::Skip);
}

#[test]
fn security_does_not_reward_future_or_stale_updates() {
    let eval = Evaluator::new(0.7);

    let mut future = candidate_for_boundaries(10, Some("Rust"), true);
    future.updated_at = Some(chrono::Utc::now() + chrono::Duration::days(2));
    assert!((eval.score_security(&future) - 0.8).abs() < f64::EPSILON);

    let mut stale = candidate_for_boundaries(10, Some("Rust"), true);
    stale.updated_at = Some(chrono::Utc::now() - chrono::Duration::days(365));
    assert!((eval.score_security(&stale) - 0.8).abs() < f64::EPSILON);
}

#[test]
fn contains_word_requires_non_alphanumeric_boundaries() {
    assert!(contains_word("safe hack-tool", "hack"));
    assert!(contains_word("hack", "hack"));
    assert!(!contains_word("hackathon", "hack"));
    assert!(!contains_word("lifehacks", "hack"));
}
