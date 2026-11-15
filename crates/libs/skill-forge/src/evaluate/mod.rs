//! # 评估器模块 (Evaluator)
//!
//! 本模块负责对发现的技术候选项进行多维度评分和推荐决策。
//!
//! ## 核心功能
//!
//! - **多维度评分**：从兼容性、质量、安全性三个维度对技术候选项进行量化评估
//! - **加权综合**：使用预定义权重计算综合得分
//! - **推荐决策**：根据综合得分自动生成集成建议（自动/人工/跳过）
//!
//! ## 评分维度与权重
//!
//! | 维度       | 权重  | 说明                                   |
//! |------------|-------|----------------------------------------|
//! | 兼容性     | 30%   | 操作系统、架构、运行时兼容程度         |
//! | 质量       | 35%   | 代码质量信号：星标数、测试、文档等     |
//! | 安全性     | 35%   | 安全态势：许可证、已知恶意模式检测等   |
//!
//! ## 使用示例
//!
//! ```ignore
//! use skill_forge::evaluate::Evaluator;
//! use skill_forge::scout::ScoutResult;
//!
//! // 创建评估器，设置自动集成的最低分数阈值为 0.7
//! let evaluator = Evaluator::new(0.7);
//!
//! // 评估候选项
//! let result = evaluator.evaluate(scout_result);
//!
//! // 根据推荐决策执行后续操作
//! match result.recommendation {
//!     Recommendation::Auto => { /* 自动集成 */ }
//!     Recommendation::Manual => { /* 需要人工审核 */ }
//!     Recommendation::Skip => { /* 跳过 */ }
//! }
//! ```

use serde::{Deserialize, Serialize};

use super::scout::ScoutResult;

// ---------------------------------------------------------------------------
// 评分维度
// ---------------------------------------------------------------------------

/// 多维度评分结果
///
/// 包含候选项在各个评估维度上的得分，每个维度的分数范围为 [0.0, 1.0]。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scores {
    /// 兼容性得分 (0.0–1.0)
    ///
    /// 评估候选项与目标环境的兼容程度，包括：
    /// - 操作系统兼容性
    /// - 系统架构兼容性
    /// - 运行时环境兼容性
    pub compatibility: f64,

    /// 质量得分 (0.0–1.0)
    ///
    /// 评估候选项的代码质量信号，包括：
    /// - GitHub 星标数
    /// - 测试覆盖率
    /// - 文档完整性
    pub quality: f64,

    /// 安全性得分 (0.0–1.0)
    ///
    /// 评估候选项的安全态势，包括：
    /// - 开源许可证是否存在
    /// - 是否包含已知恶意模式
    /// - 代码更新活跃度
    pub security: f64,
}

impl Scores {
    /// 计算加权综合得分
    ///
    /// 使用预定义权重计算各维度的加权总和：
    /// - 兼容性权重：0.30 (30%)
    /// - 质量权重：0.35 (35%)
    /// - 安全性权重：0.35 (35%)
    ///
    /// # 返回值
    ///
    /// 返回加权后的综合得分，范围通常在 [0.0, 1.0] 之间。
    ///
    /// # 计算公式
    ///
    /// ```text
    /// total = compatibility * 0.30 + quality * 0.35 + security * 0.35
    /// ```
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let scores = Scores {
    ///     compatibility: 1.0,
    ///     quality: 0.8,
    ///     security: 0.9,
    /// };
    /// let total = scores.total(); // 0.895
    /// ```
    pub fn total(&self) -> f64 {
        self.compatibility * 0.30 + self.quality * 0.35 + self.security * 0.35
    }
}

// ---------------------------------------------------------------------------
// 推荐类型
// ---------------------------------------------------------------------------

/// 集成推荐类型
///
/// 根据候选项的综合得分，自动生成的集成建议。
/// 用于指导后续的技能集成流程。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recommendation {
    /// 自动集成
    ///
    /// 综合得分 >= 阈值，可安全地自动集成到系统中。
    /// 此类候选项通过了所有关键检查，风险较低。
    Auto,

    /// 人工审核
    ///
    /// 综合得分在 [0.4, 阈值) 范围内，需要人工审核后决定是否集成。
    /// 此类候选项存在一定风险，需要进一步评估。
    Manual,

    /// 跳过
    ///
    /// 综合得分 < 0.4，建议跳过不集成。
    /// 此类候选项质量或安全性存在明显问题。
    Skip,
}

// ---------------------------------------------------------------------------
// 评估结果
// ---------------------------------------------------------------------------

/// 完整的评估结果
///
/// 包含候选项的完整评估信息，包括原始数据、各维度得分、
/// 综合得分以及集成推荐。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// 被评估的候选项原始数据
    pub candidate: ScoutResult,

    /// 各维度的详细得分
    pub scores: Scores,

    /// 加权后的综合得分
    pub total_score: f64,

    /// 集成推荐决策
    pub recommendation: Recommendation,
}

// ---------------------------------------------------------------------------
// 评估器
// ---------------------------------------------------------------------------

/// 技能候选项评估器
///
/// 负责对 Scout 发现的技术候选项进行多维度评估，
/// 并生成集成推荐决策。
///
/// # 字段
///
/// - `min_score`: 自动集成的最低综合得分阈值
///
/// # 示例
///
/// ```ignore
/// // 创建评估器，阈值为 0.7
/// let evaluator = Evaluator::new(0.7);
///
/// // 评估候选项
/// let result = evaluator.evaluate(candidate);
/// ```
pub struct Evaluator {
    /// 自动集成的最低综合得分阈值
    ///
    /// 当候选项的综合得分达到或超过此阈值时，
    /// 将被推荐为自动集成（Recommendation::Auto）。
    min_score: f64,
}

/// 已知的恶意模式列表
///
/// 这些模式用于检测仓库名称或描述中可能存在的恶意内容。
/// 采用全词匹配方式（word boundary matching），避免误判。
///
/// # 匹配规则
///
/// - 匹配时忽略大小写
/// - 要求匹配词的前后字符为非字母数字字符（词边界）
/// - 任意一个模式匹配即触发惩罚
const BAD_PATTERNS: &[&str] =
    &["malware", "exploit", "hack", "crack", "keygen", "ransomware", "trojan"];

/// 检查字符串中是否包含指定单词（全词匹配）
///
/// 执行全词匹配（whole-word matching），确保匹配的词被非字母数字字符包围，
/// 避免部分匹配导致的误判。
///
/// # 参数
///
/// - `haystack`: 待搜索的字符串
/// - `word`: 要查找的单词
///
/// # 返回值
///
/// 如果找到全词匹配则返回 `true`，否则返回 `false`。
///
/// # 匹配逻辑
///
/// 对于每个匹配位置，检查：
/// 1. 匹配位置是字符串开头，或前一个字符不是字母数字
/// 2. 匹配结束位置是字符串末尾，或后一个字符不是字母数字
///
/// 两个条件同时满足才认为是全词匹配。
///
/// # 示例
///
/// ```ignore
/// assert!(contains_word("this is a test", "test"));    // true
/// assert!(!contains_word("testing", "test"));          // false（部分匹配）
/// assert!(contains_word("test-case", "test"));         // true
/// assert!(!contains_word("attest", "test"));           // false（部分匹配）
/// ```
fn contains_word(haystack: &str, word: &str) -> bool {
    // 遍历所有匹配位置
    for (i, _) in haystack.match_indices(word) {
        // 检查匹配位置之前是否为词边界：
        // - 位于字符串开头，或
        // - 前一个字符不是字母数字
        let before_ok = i == 0 || !haystack.as_bytes()[i - 1].is_ascii_alphanumeric();

        // 检查匹配位置之后是否为词边界
        let after = i + word.len();
        let after_ok =
            after >= haystack.len() || !haystack.as_bytes()[after].is_ascii_alphanumeric();

        // 前后都是词边界，确认为全词匹配
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

impl Evaluator {
    /// 创建新的评估器实例
    ///
    /// # 参数
    ///
    /// - `min_score`: 自动集成的最低综合得分阈值（通常为 0.7）
    ///
    /// # 返回值
    ///
    /// 返回配置好的评估器实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let evaluator = Evaluator::new(0.7);
    /// ```
    pub fn new(min_score: f64) -> Self {
        Self { min_score }
    }

    /// 评估候选项并生成评估结果
    ///
    /// 对候选项进行多维度评估，计算综合得分，并生成集成推荐。
    ///
    /// # 参数
    ///
    /// - `candidate`: 待评估的候选项数据（来自 Scout 模块）
    ///
    /// # 返回值
    ///
    /// 返回包含完整评估信息的 `EvalResult`，包括：
    /// - 原始候选项数据
    /// - 各维度详细得分
    /// - 综合得分
    /// - 集成推荐决策
    ///
    /// # 评估流程
    ///
    /// 1. 分别计算兼容性、质量、安全性三个维度的得分
    /// 2. 使用加权公式计算综合得分
    /// 3. 根据综合得分确定推荐决策：
    ///    - 得分 >= min_score → Auto（自动集成）
    ///    - 得分 >= 0.4 → Manual（人工审核）
    ///    - 得分 < 0.4 → Skip（跳过）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let evaluator = Evaluator::new(0.7);
    /// let result = evaluator.evaluate(candidate);
    ///
    /// println!("Total score: {}", result.total_score);
    /// println!("Recommendation: {:?}", result.recommendation);
    /// ```
    pub fn evaluate(&self, candidate: ScoutResult) -> EvalResult {
        // 分别计算各维度得分
        let compatibility = self.score_compatibility(&candidate);
        let quality = self.score_quality(&candidate);
        let security = self.score_security(&candidate);

        // 组合各维度得分并计算综合得分
        let scores = Scores { compatibility, quality, security };
        let total_score = scores.total();

        // 根据综合得分确定推荐决策
        let recommendation = if total_score >= self.min_score {
            // 达到阈值，可自动集成
            Recommendation::Auto
        } else if total_score >= 0.4 {
            // 未达阈值但尚可，需人工审核
            Recommendation::Manual
        } else {
            // 得分过低，跳过
            Recommendation::Skip
        };

        EvalResult { candidate, scores, total_score, recommendation }
    }

    // -- 维度评分器 ---------------------------------------------------------

    /// 计算兼容性得分
    ///
    /// 根据候选项的编程语言评估与 VibeWindow 系统的兼容程度。
    ///
    /// # 评分规则
    ///
    /// | 语言                    | 得分 | 说明                     |
    /// |-------------------------|------|--------------------------|
    /// | Rust                    | 1.0  | 原生支持，完全兼容       |
    /// | Python / TypeScript / JavaScript | 0.6  | 通过桥接层支持           |
    /// | 其他已知语言            | 0.3  | 兼容性有限               |
    /// | 未知语言                | 0.2  | 兼容性最差               |
    ///
    /// # 参数
    ///
    /// - `c`: 候选项数据
    ///
    /// # 返回值
    ///
    /// 返回兼容性得分，范围 [0.0, 1.0]
    fn score_compatibility(&self, c: &ScoutResult) -> f64 {
        match c.language.as_deref() {
            Some("Rust") => 1.0,
            Some("Python" | "TypeScript" | "JavaScript") => 0.6,
            Some(_) => 0.3,
            None => 0.2,
        }
    }

    /// 计算质量得分
    ///
    /// 基于 GitHub 星标数评估候选项的质量，使用对数尺度归一化。
    ///
    /// # 评分公式
    ///
    /// ```text
    /// score = log2(stars + 1) / 10
    /// ```
    ///
    /// # 算法说明
    ///
    /// - 使用对数函数压缩星标数范围，避免热门项目过度主导
    /// - 星标数为 0 时得分为 0，星标数约 1000 时得分为 1.0
    /// - 结果上限为 1.0
    ///
    /// # 参数
    ///
    /// - `c`: 候选项数据
    ///
    /// # 返回值
    ///
    /// 返回质量得分，范围 [0.0, 1.0]
    fn score_quality(&self, c: &ScoutResult) -> f64 {
        // 使用 log2(stars + 1) / 10 计算，上限为 1.0
        // 星标约 1023 时达到上限
        let raw = ((c.stars as f64) + 1.0).log2() / 10.0;
        raw.min(1.0)
    }

    /// 计算安全性得分
    ///
    /// 综合评估候选项的安全态势，包括许可证、恶意模式、更新活跃度。
    ///
    /// # 评分逻辑
    ///
    /// 1. **基础分**: 0.5
    /// 2. **许可证加成**: 如果有许可证，+0.3
    /// 3. **恶意模式惩罚**: 如果检测到恶意模式，-0.5
    /// 4. **活跃度加成**: 如果 180 天内有更新，+0.2
    /// 5. **结果范围**: 限制在 [0.0, 1.0]
    ///
    /// # 恶意模式检测
    ///
    /// 检查仓库名称和描述是否包含已知的恶意模式（如 malware、exploit 等）。
    /// 使用全词匹配避免误判。
    ///
    /// # 参数
    ///
    /// - `c`: 候选项数据
    ///
    /// # 返回值
    ///
    /// 返回安全性得分，范围 [0.0, 1.0]
    fn score_security(&self, c: &ScoutResult) -> f64 {
        let mut score: f64 = 0.5;

        // 许可证加成：有许可证的项目更可信
        if c.has_license {
            score += 0.3;
        }

        // 恶意模式惩罚：检查名称和描述中的恶意关键词
        // 使用小写进行不区分大小写的匹配
        let lower_name = c.name.to_lowercase();
        let lower_desc = c.description.to_lowercase();
        for pat in BAD_PATTERNS {
            // 全词匹配检测恶意模式
            if contains_word(&lower_name, pat) || contains_word(&lower_desc, pat) {
                score -= 0.5;
                break; // 检测到一个恶意模式即停止
            }
        }

        // 活跃度加成：180 天内有更新的项目更可信
        // 同时防止未来时间戳导致的异常
        if let Some(updated) = c.updated_at {
            let age_days = (chrono::Utc::now() - updated).num_days();
            // 只奖励 0-180 天内的更新（排除负数，即未来时间戳）
            if (0..180).contains(&age_days) {
                score += 0.2;
            }
        }

        // 确保得分在有效范围内
        score.clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
