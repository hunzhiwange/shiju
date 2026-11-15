//! 查询分类器模块
//!
//! 本模块提供用户消息的分类功能，根据预定义的规则将消息匹配到不同的提示类型。
//! 分类器支持基于关键词和模式的多条件匹配，并按优先级返回匹配结果。
//!
//! # 主要功能
//!
//! - **规则匹配**：根据配置的分类规则对用户消息进行匹配
//! - **优先级排序**：支持多规则按优先级顺序匹配，高优先级规则优先
//! - **多条件过滤**：支持关键词匹配（不区分大小写）和模式匹配（区分大小写）
//! - **长度约束**：支持消息长度范围过滤
//!
//! # 使用示例
//!
//! ```ignore
//! use config_routing::QueryClassificationConfig;
//! use model_selection::classifier::classify;
//!
//! let config = QueryClassificationConfig {
//!     enabled: true,
//!     rules: vec![/* 规则列表 */],
//! };
//!
//! if let Some(hint) = classify(&config, "用户消息内容") {
//!     println!("匹配到分类提示: {}", hint);
//! }
//! ```

use config_routing::QueryClassificationConfig;

/// 分类决策结果
///
/// 包含匹配成功后的分类提示和匹配规则的优先级信息，
/// 用于向调用方提供分类结果以及可观测性数据。
///
/// # 字段
///
/// - `hint`：分类提示字符串，用于标识消息所属的类别
/// - `priority`：匹配规则的优先级值，数值越大优先级越高
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationDecision {
    /// 分类提示字符串，用于标识消息所属的类别
    pub hint: String,
    /// 匹配规则的优先级值，数值越大表示优先级越高
    pub priority: i32,
}

/// 对用户消息进行分类并返回匹配的提示字符串
///
/// 根据配置的分类规则对用户消息进行匹配，返回第一个匹配成功的规则提示。
/// 该函数是 [`classify_with_decision`] 的简化版本，仅返回提示字符串。
///
/// # 参数
///
/// - `config`：查询分类配置，包含是否启用分类以及分类规则列表
/// - `message`：待分类的用户消息内容
///
/// # 返回值
///
/// 返回 `Some(String)` 表示匹配成功，包含分类提示字符串；
/// 返回 `None` 表示未匹配到任何规则，可能的原因包括：
/// - 分类功能已禁用（`config.enabled` 为 `false`）
/// - 规则列表为空
/// - 所有规则均未匹配成功
///
/// # 示例
///
/// ```ignore
/// let hint = classify(&config, "帮我分析这个错误日志");
/// if let Some(h) = hint {
///     println!("消息被分类为: {}", h);
/// }
/// ```
pub fn classify(config: &QueryClassificationConfig, message: &str) -> Option<String> {
    classify_with_decision(config, message).map(|decision| decision.hint)
}

/// 对用户消息进行分类并返回完整的分类决策信息
///
/// 根据配置的分类规则对用户消息进行完整匹配，返回包含提示和优先级的决策结果。
/// 该函数提供比 [`classify`] 更详细的信息，适用于需要可观测性数据的场景。
///
/// # 匹配逻辑
///
/// 1. **前置检查**：如果分类功能未启用或规则列表为空，直接返回 `None`
/// 2. **规则排序**：按优先级从高到低排序所有规则
/// 3. **遍历匹配**：依次检查每个规则是否满足条件
///    - 首先验证消息长度是否在约束范围内
///    - 然后检查关键词或模式是否匹配
/// 4. **返回结果**：返回第一个匹配成功的规则对应的分类决策
///
/// # 参数
///
/// - `config`：查询分类配置，包含以下关键属性：
///   - `enabled`：是否启用分类功能
///   - `rules`：分类规则列表，每条规则包含关键词、模式、长度约束等
/// - `message`：待分类的用户消息内容
///
/// # 返回值
///
/// 返回 `Some(ClassificationDecision)` 表示匹配成功，包含：
/// - `hint`：分类提示字符串
/// - `priority`：匹配规则的优先级
///
/// 返回 `None` 表示未匹配到任何规则。
///
/// # 示例
///
/// ```ignore
/// let decision = classify_with_decision(&config, "查询天气信息");
/// if let Some(d) = decision {
///     println!("分类提示: {}, 优先级: {}", d.hint, d.priority);
/// }
/// ```
pub fn classify_with_decision(
    config: &QueryClassificationConfig,
    message: &str,
) -> Option<ClassificationDecision> {
    // 前置检查：分类功能未启用或无规则配置时直接返回 None
    if !config.enabled || config.rules.is_empty() {
        return None;
    }

    // 预处理消息：转换为小写形式用于关键词不区分大小写匹配
    let lower = message.to_lowercase();
    // 缓存消息长度用于长度约束检查
    let len = message.len();

    // 收集所有规则并按优先级降序排序，确保高优先级规则优先匹配
    let mut rules: Vec<_> = config.rules.iter().collect();
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));

    // 遍历排序后的规则列表进行匹配
    for rule in rules {
        // 长度约束检查：消息长度必须满足最小长度要求
        if let Some(min) = rule.min_length {
            if len < min {
                // 长度不足，跳过当前规则继续检查下一条
                continue;
            }
        }
        // 长度约束检查：消息长度不能超过最大长度限制
        if let Some(max) = rule.max_length {
            if len > max {
                // 长度超限，跳过当前规则继续检查下一条
                continue;
            }
        }

        // 关键词匹配（不区分大小写）和模式匹配（区分大小写）
        // 任一条件满足即视为匹配成功
        let keyword_hit =
            rule.keywords.iter().any(|kw: &String| lower.contains(&kw.to_lowercase()));
        let pattern_hit = rule.patterns.iter().any(|pat: &String| message.contains(pat.as_str()));

        // 关键词或模式任一匹配成功即返回对应的分类决策
        if keyword_hit || pattern_hit {
            return Some(ClassificationDecision {
                hint: rule.hint.clone(),
                priority: rule.priority,
            });
        }
    }

    // 所有规则均未匹配，返回 None
    None
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
