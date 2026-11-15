//! # 查询分类器单元测试模块
//!
//! 本模块提供 [`classify`] 和 [`classify_with_decision`] 函数的全面测试覆盖，
//! 验证查询分类系统的核心行为和边界条件。
//!
//! ## 测试覆盖范围
//!
//! - **配置状态**：启用/禁用分类功能
//! - **规则匹配**：关键词匹配（大小写不敏感）、正则模式匹配（大小写敏感）
//! - **约束条件**：查询长度约束（最小/最大长度）
//! - **优先级排序**：多规则匹配时的优先级排序与选择
//! - **边界情况**：空规则集、无匹配等
//!
//! ## 架构说明
//!
//! 分类器遵循配置驱动的 trait 架构，通过 [`QueryClassificationConfig`] 定义
//! 分类规则，实现对查询的智能路由和优化提示选择。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use config_routing::{ClassificationRule, QueryClassificationConfig};

    /// 构建测试用 [`QueryClassificationConfig`] 实例。
    ///
    /// # 参数
    ///
    /// - `enabled`: 是否启用分类功能；`false` 时 [`classify`] 将直接返回 `None`
    /// - `rules`: 分类规则列表，按优先级排序后依次匹配
    ///
    /// # 返回值
    ///
    /// 返回一个配置好的 [`QueryClassificationConfig`] 实例，可直接用于测试。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let config = make_config(true, vec![
    ///     ClassificationRule {
    ///         hint: "fast".into(),
    ///         keywords: vec!["hello".into()],
    ///         ..Default::default()
    ///     },
    /// ]);
    /// ```
    fn make_config(enabled: bool, rules: Vec<ClassificationRule>) -> QueryClassificationConfig {
        QueryClassificationConfig { enabled, rules }
    }

    /// 测试：分类功能禁用时应返回 `None`。
    ///
    /// # 验证点
    ///
    /// - 即使规则完全匹配查询内容，`enabled = false` 时分类器应短路返回 `None`
    /// - 确保禁用开关正确生效，不执行任何规则匹配逻辑
    #[test]
    fn disabled_returns_none() {
        let config = make_config(
            false,
            vec![ClassificationRule {
                hint: "fast".into(),
                keywords: vec!["hello".into()],
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "hello"), None);
    }

    /// 测试：空规则集应返回 `None`。
    ///
    /// # 验证点
    ///
    /// - 当 `rules` 为空数组时，即使分类功能启用，也应返回 `None`
    /// - 确保空规则集边界条件正确处理
    #[test]
    fn empty_rules_returns_none() {
        let config = make_config(true, vec![]);
        assert_eq!(classify(&config, "hello"), None);
    }

    /// 测试：关键词匹配应大小写不敏感。
    ///
    /// # 验证点
    ///
    /// - 关键词 "hello" 应匹配 "HELLO world"（全大写）
    /// - 确保用户输入的大小写变体不会影响分类结果
    /// - 验证返回正确的 `hint` 值
    #[test]
    fn keyword_match_case_insensitive() {
        let config = make_config(
            true,
            vec![ClassificationRule {
                hint: "fast".into(),
                keywords: vec!["hello".into()],
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "HELLO world"), Some("fast".into()));
    }

    /// 测试：正则模式匹配应大小写敏感。
    ///
    /// # 验证点
    ///
    /// - 正则模式 `"fn "` 应匹配 `"fn main()"`（小写）
    /// - 正则模式 `"fn "` 不应匹配 `"FN MAIN()"`（全大写）
    /// - 验证正则匹配与关键词匹配的大小写行为差异
    ///
    /// # 设计说明
    ///
    /// 正则模式保持大小写敏感以支持精确的代码模式匹配（如函数定义），
    /// 而关键词匹配使用大小写不敏感以提升用户体验。
    #[test]
    fn pattern_match_case_sensitive() {
        let config = make_config(
            true,
            vec![ClassificationRule {
                hint: "code".into(),
                patterns: vec!["fn ".into()],
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "fn main()"), Some("code".into()));
        assert_eq!(classify(&config, "FN MAIN()"), None);
    }

    /// 测试：长度约束应正确过滤匹配结果。
    ///
    /// # 验证点（最大长度约束）
    ///
    /// - 查询 "hi" 长度 ≤ 10，应匹配 `max_length: Some(10)` 规则
    /// - 查询 "hi there, how are you doing today?" 长度 > 10，不应匹配
    ///
    /// # 验证点（最小长度约束）
    ///
    /// - 查询 "explain" 长度 < 20，不应匹配 `min_length: Some(20)` 规则
    /// - 查询 "explain how this works in detail" 长度 ≥ 20，应匹配
    ///
    /// # 设计说明
    ///
    /// 长度约束允许根据查询复杂度选择不同的处理策略：
    /// - 短查询 → 快速响应模式（如 "fast" hint）
    /// - 长查询 → 深度推理模式（如 "reasoning" hint）
    #[test]
    fn length_constraints() {
        let config = make_config(
            true,
            vec![ClassificationRule {
                hint: "fast".into(),
                keywords: vec!["hi".into()],
                max_length: Some(10),
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "hi"), Some("fast".into()));
        assert_eq!(classify(&config, "hi there, how are you doing today?"), None);

        let config2 = make_config(
            true,
            vec![ClassificationRule {
                hint: "reasoning".into(),
                keywords: vec!["explain".into()],
                min_length: Some(20),
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config2, "explain"), None);
        assert_eq!(
            classify(&config2, "explain how this works in detail"),
            Some("reasoning".into())
        );
    }

    /// 测试：多规则匹配时应按优先级选择最高优先级规则。
    ///
    /// # 验证点
    ///
    /// - 两个规则都匹配关键词 "code"：
    ///   - 规则 1: `hint = "fast"`, `priority = 1`
    ///   - 规则 2: `hint = "code"`, `priority = 10`
    /// - 应选择优先级更高的规则 2（`priority = 10`）
    /// - 返回的 `hint` 应为 `"code"`
    ///
    /// # 设计说明
    ///
    /// 优先级数值越大表示优先级越高，允许更具体的规则覆盖通用规则。
    /// 例如：通用 "fast" 规则可能 priority = 1，而特定 "code" 规则 priority = 10。
    #[test]
    fn priority_ordering() {
        let config = make_config(
            true,
            vec![
                ClassificationRule {
                    hint: "fast".into(),
                    keywords: vec!["code".into()],
                    priority: 1,
                    ..Default::default()
                },
                ClassificationRule {
                    hint: "code".into(),
                    keywords: vec!["code".into()],
                    priority: 10,
                    ..Default::default()
                },
            ],
        );
        assert_eq!(classify(&config, "write some code"), Some("code".into()));
    }

    /// 测试：无匹配时应返回 `None`。
    ///
    /// # 验证点
    ///
    /// - 查询内容与所有规则的关键词和模式均不匹配
    /// - 应返回 `None` 而非错误或默认值
    /// - 确保分类器的"快速失败"行为正确
    #[test]
    fn no_match_returns_none() {
        let config = make_config(
            true,
            vec![ClassificationRule {
                hint: "fast".into(),
                keywords: vec!["hello".into()],
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "something completely different"), None);
    }

    /// 测试：[`classify_with_decision`] 应暴露匹配规则的完整信息。
    ///
    /// # 验证点
    ///
    /// - 多规则匹配时应选择最高优先级规则（`priority = 10`）
    /// - 返回的 [`ClassificationDecision`] 应包含：
    ///   - `hint`: 正确的提示值（`"code"`）
    ///   - `priority`: 匹配规则的实际优先级（`10`）
    /// - 验证决策详情的完整性和准确性
    ///
    /// # 设计说明
    ///
    /// [`classify_with_decision`] 与 [`classify`] 的区别：
    /// - [`classify`]: 仅返回 `hint` 字符串（轻量级 API）
    /// - [`classify_with_decision`]: 返回完整 [`ClassificationDecision`]（包含元数据）
    ///
    /// 完整决策信息用于调试、日志记录和高级路由逻辑。
    #[test]
    fn classify_with_decision_exposes_priority_of_matched_rule() {
        let config = make_config(
            true,
            vec![
                ClassificationRule {
                    hint: "fast".into(),
                    keywords: vec!["code".into()],
                    priority: 3,
                    ..Default::default()
                },
                ClassificationRule {
                    hint: "code".into(),
                    keywords: vec!["code".into()],
                    priority: 10,
                    ..Default::default()
                },
            ],
        );

        let decision = classify_with_decision(&config, "write code now")
            .expect("classification decision expected");
        assert_eq!(decision.hint, "code");
        assert_eq!(decision.priority, 10);
    }
}
