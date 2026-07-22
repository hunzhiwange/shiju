//! SkillForge — 技能自动发现、评估和集成引擎
//!
//! # 模块概述
//!
//! SkillForge 是 VibeWindow 代理系统的技能自动化管理引擎,负责从外部源自动发现、
//! 评估并集成高质量的技能包。该模块实现了完整的技能生命周期管理流水线。
//!
//! # 核心流水线
//!
//! 流水线包含三个主要阶段:
//!
//! 1. **发现 (Scout)**: 从配置的外部源(如 GitHub、ClawHub、HuggingFace)搜索和发现潜在的技能候选项
//! 2. **评估 (Evaluate)**: 对发现的候选项进行多维度的质量评分和安全性检查
//! 3. **集成 (Integrate)**: 根据评估结果,将符合条件的技能自动或手动集成到 VibeWindow 系统中
//!
//! # 架构设计
//!
//! - **模块化**: 每个流水线阶段都由独立的子模块实现,便于扩展和维护
//! - **可配置**: 通过 `SkillForgeConfig` 支持灵活的配置选项
//! - **安全第一**: 自动集成前进行严格的安全评估和质量把关
//!
//! # 子模块
//!
//! - [`scout`]: 负责从各种外部源发现技能候选项
//! - [`evaluate`]: 负责评估技能质量和安全性
//! - [`integrate`]: 负责将合格的技能集成到系统中
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use vibe_agent::skillforge::{SkillForge, SkillForgeConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 创建配置
//! let config = SkillForgeConfig {
//!     enabled: true,
//!     auto_integrate: true,
//!     sources: vec!["github".to_string()],
//!     scan_interval_hours: 24,
//!     min_score: 0.7,
//!     github_token: Some("your_token".to_string()),
//!     output_dir: "./skills".to_string(),
//! };
//!
//! // 创建 SkillForge 实例并运行流水线
//! let forge = SkillForge::new(config);
//! let report = forge.forge().await?;
//!
//! println!("发现: {}, 集成: {}", report.discovered, report.auto_integrated);
//! # Ok(())
//! # }
//! ```

pub mod evaluate;
pub mod integrate;
pub mod scout;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use self::evaluate::{EvalResult, Evaluator, Recommendation};
use self::integrate::Integrator;
use self::scout::{GitHubScout, Scout, ScoutResult, ScoutSource};

// ---------------------------------------------------------------------------
// 配置 (Configuration)
// ---------------------------------------------------------------------------

/// SkillForge 配置结构体
///
/// 定义了技能发现、评估和集成流水线的所有配置选项。
/// 该结构体支持序列化和反序列化,可以从配置文件或环境变量加载。
///
/// # 配置字段说明
///
/// - `enabled`: 是否启用 SkillForge 引擎
/// - `auto_integrate`: 是否自动集成高分技能(无需人工审核)
/// - `sources`: 技能发现源列表(如 "github", "clawhub")
/// - `scan_interval_hours`: 定期扫描的时间间隔(小时)
/// - `min_score`: 自动集成的最低质量分数阈值
/// - `github_token`: GitHub 个人访问令牌(可选,用于提高 API 速率限制)
/// - `output_dir`: 集成技能的输出目录路径
///
/// # 安全注意事项
///
/// - `github_token` 字段在 Debug 输出时会被自动脱敏为 `"***"`
/// - 敏感信息不应记录到日志中
///
/// # 示例
///
/// ```rust
/// use vibe_agent::skillforge::SkillForgeConfig;
///
/// let config = SkillForgeConfig::default();
/// assert_eq!(config.enabled, false);
/// assert_eq!(config.auto_integrate, true);
/// assert_eq!(config.min_score, 0.7);
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct SkillForgeConfig {
    /// 是否启用 SkillForge 引擎
    ///
    /// 当设置为 `false` 时,`forge()` 方法将直接返回空报告而不执行任何操作。
    /// 默认值: `false`
    #[serde(default)]
    pub enabled: bool,

    /// 是否自动集成符合条件的技能
    ///
    /// 当设置为 `true` 且技能评估推荐为 `Auto` 时,将自动集成该技能。
    /// 当设置为 `false` 时,即使技能评分达标也需要人工审核。
    /// 默认值: `true`
    #[serde(default = "default_auto_integrate")]
    pub auto_integrate: bool,

    /// 技能发现源列表
    ///
    /// 指定要从哪些外部源发现技能。支持的源包括:
    /// - `"github"`: 从 GitHub 仓库发现技能
    /// - `"clawhub"`: 从 ClawHub 发现技能(尚未实现)
    /// - `"huggingface"`: 从 HuggingFace 发现技能(尚未实现)
    ///
    /// 默认值: `["github", "clawhub"]`
    #[serde(default = "default_sources")]
    pub sources: Vec<String>,

    /// 定期扫描的时间间隔(小时)
    ///
    /// 用于定时任务调度,指定两次技能发现扫描之间的时间间隔。
    /// 默认值: `24` (每天扫描一次)
    #[serde(default = "default_scan_interval")]
    pub scan_interval_hours: u64,

    /// 自动集成的最低质量分数阈值
    ///
    /// 评分范围: 0.0 - 1.0
    /// 只有评分达到或超过此阈值的技能才会被考虑自动集成。
    /// 建议值: 0.7 - 0.9 (越高越严格)
    /// 默认值: `0.7`
    #[serde(default = "default_min_score")]
    pub min_score: f64,

    /// GitHub 个人访问令牌(可选)
    ///
    /// 用于提高 GitHub API 的速率限制。如果不提供令牌,将使用匿名访问,
    /// 但会受到更严格的速率限制。
    ///
    /// **安全提示**: 该字段在 Debug 输出时会被自动脱敏。
    /// 默认值: `None`
    #[serde(default)]
    pub github_token: Option<String>,

    /// 集成技能的输出目录路径
    ///
    /// 指定将合格的技能清单文件写入到哪个目录。
    /// 如果目录不存在,将在集成时自动创建。
    /// 默认值: `"./skills"`
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

/// 返回 `auto_integrate` 字段的默认值
///
/// 默认启用自动集成功能。
fn default_auto_integrate() -> bool {
    true
}

/// 返回 `sources` 字段的默认值
///
/// 默认从 GitHub 和 ClawHub 两个源发现技能。
fn default_sources() -> Vec<String> {
    vec!["github".into(), "clawhub".into()]
}

/// 返回 `scan_interval_hours` 字段的默认值
///
/// 默认每 24 小时扫描一次。
fn default_scan_interval() -> u64 {
    24
}

/// 返回 `min_score` 字段的默认值
///
/// 默认最低分数阈值为 0.7 (满分 1.0)。
fn default_min_score() -> f64 {
    0.7
}

/// 返回 `output_dir` 字段的默认值
///
/// 默认输出目录为当前目录下的 `skills` 文件夹。
fn default_output_dir() -> String {
    "./skills".into()
}

impl Default for SkillForgeConfig {
    /// 为 SkillForgeConfig 提供默认实现
    ///
    /// 返回一个包含所有字段默认值的配置实例。
    /// 该实现确保所有字段都有合理的默认值,避免使用 `Option` 包装。
    fn default() -> Self {
        Self {
            enabled: false,
            auto_integrate: default_auto_integrate(),
            sources: default_sources(),
            scan_interval_hours: default_scan_interval(),
            min_score: default_min_score(),
            github_token: None,
            output_dir: default_output_dir(),
        }
    }
}

impl std::fmt::Debug for SkillForgeConfig {
    /// 自定义 Debug 格式化实现
    ///
    /// **安全特性**: 对 `github_token` 字段进行脱敏处理,
    /// 防止敏感信息在日志或调试输出中泄露。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillForgeConfig")
            .field("enabled", &self.enabled)
            .field("auto_integrate", &self.auto_integrate)
            .field("sources", &self.sources)
            .field("scan_interval_hours", &self.scan_interval_hours)
            .field("min_score", &self.min_score)
            // 安全脱敏: 将令牌替换为 "***" 以防止泄露
            .field("github_token", &self.github_token.as_ref().map(|_| "***"))
            .field("output_dir", &self.output_dir)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// 锻造报告 (ForgeReport) — 单次流水线执行的汇总报告
// ---------------------------------------------------------------------------

/// SkillForge 流水线执行报告
///
/// 该结构体记录了一次完整流水线执行(发现→评估→集成)的统计信息和详细结果。
/// 用于向用户或监控系统报告流水线的执行情况。
///
/// # 字段说明
///
/// - `discovered`: 从所有源发现的去重后候选项总数
/// - `evaluated`: 完成评估的候选项数量(通常等于 discovered)
/// - `auto_integrated`: 成功自动集成的技能数量
/// - `manual_review`: 需要人工审核的技能数量
/// - `skipped`: 被跳过的技能数量(评分不达标或其他原因)
/// - `results`: 所有候选项的详细评估结果列表
///
/// # 示例输出
///
/// ```text
/// 发现: 50, 评估: 50, 自动集成: 12, 人工审核: 8, 跳过: 30
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeReport {
    /// 从所有源发现的去重后候选项总数
    pub discovered: usize,

    /// 完成评估的候选项数量
    pub evaluated: usize,

    /// 成功自动集成的技能数量
    pub auto_integrated: usize,

    /// 需要人工审核的技能数量
    pub manual_review: usize,

    /// 被跳过的技能数量(评分不达标或其他原因)
    pub skipped: usize,

    /// 所有候选项的详细评估结果列表
    ///
    /// 包含每个候选项的评分、推荐操作和元数据,
    /// 可用于后续的人工审核或分析。
    pub results: Vec<EvalResult>,
}

// ---------------------------------------------------------------------------
// SkillForge 核心引擎
// ---------------------------------------------------------------------------

/// SkillForge 技能发现和集成引擎
///
/// SkillForge 是技能自动化管理的主入口,协调发现、评估和集成三个阶段。
/// 它维护配置信息和内部组件(评估器和集成器),提供统一的流水线执行接口。
///
/// # 架构
///
/// ```text
/// SkillForge
///   ├── config: SkillForgeConfig (配置信息)
///   ├── evaluator: Evaluator (评估组件)
///   └── integrator: Integrator (集成组件)
/// ```
///
/// # 生命周期
///
/// 1. 使用 [`SkillForge::new`] 创建实例
/// 2. 调用 [`forge`](SkillForge::forge) 方法执行完整流水线
/// 3. 获取 `ForgeReport` 查看执行结果
///
/// # 线程安全
///
/// 该结构体的设计为单次使用,不保证多线程并发安全。
/// 如需并发执行,应为每个线程创建独立的实例。
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::skillforge::{SkillForge, SkillForgeConfig};
///
/// # async fn run() -> anyhow::Result<()> {
/// let config = SkillForgeConfig {
///     enabled: true,
///     ..Default::default()
/// };
///
/// let forge = SkillForge::new(config);
/// let report = forge.forge().await?;
///
/// println!("集成完成: {} 个技能", report.auto_integrated);
/// # Ok(())
/// # }
/// ```
pub struct SkillForge {
    /// SkillForge 配置信息
    config: SkillForgeConfig,

    /// 技能评估器
    ///
    /// 负责对发现的候选项进行质量评分和安全性检查。
    evaluator: Evaluator,

    /// 技能集成器
    ///
    /// 负责将合格的技能写入到输出目录并生成清单文件。
    integrator: Integrator,
}

impl SkillForge {
    /// 创建新的 SkillForge 实例
    ///
    /// 根据提供的配置初始化评估器和集成器组件。
    ///
    /// # 参数
    ///
    /// - `config`: SkillForge 的配置信息,包含发现源、评分阈值等设置
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 `SkillForge` 实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use vibe_agent::skillforge::{SkillForge, SkillForgeConfig};
    ///
    /// let config = SkillForgeConfig {
    ///     enabled: true,
    ///     min_score: 0.8,
    ///     ..Default::default()
    /// };
    ///
    /// let forge = SkillForge::new(config);
    /// ```
    pub fn new(config: SkillForgeConfig) -> Self {
        // 创建评估器,使用配置中的最低分数阈值
        let evaluator = Evaluator::new(config.min_score);
        // 创建集成器,指定输出目录
        let integrator = Integrator::new(config.output_dir.clone());
        Self { config, evaluator, integrator }
    }

    /// 执行完整的技能发现、评估和集成流水线
    ///
    /// 该方法是 SkillForge 的核心入口,执行三阶段流水线:
    /// 1. **发现阶段**: 从配置的外部源搜索技能候选项
    /// 2. **评估阶段**: 对每个候选项进行质量评分和安全检查
    /// 3. **集成阶段**: 根据评估结果自动或标记为人工审核
    ///
    /// # 返回值
    ///
    /// - `Ok(ForgeReport)`: 流水线执行成功,返回包含统计信息的报告
    /// - `Err(...)`: 流水线执行过程中发生严重错误
    ///
    /// # 禁用行为
    ///
    /// 如果 `config.enabled` 为 `false`,该方法将立即返回空报告而不执行任何操作。
    /// 这允许在配置中全局禁用 SkillForge 而无需修改代码。
    ///
    /// # 错误处理
    ///
    /// - 单个源的发现失败不会中断整个流水线,会继续尝试其他源
    /// - 单个技能的集成失败会被记录但不会中断流水线
    /// - 致命错误(如配置无效)会导致整个方法返回错误
    ///
    /// # 异步说明
    ///
    /// 该方法是异步的,因为它需要与外部源(如 GitHub API)进行网络通信。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use vibe_agent::skillforge::{SkillForge, SkillForgeConfig};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = SkillForgeConfig {
    ///     enabled: true,
    ///     sources: vec!["github".to_string()],
    ///     ..Default::default()
    /// };
    ///
    /// let forge = SkillForge::new(config);
    /// let report = forge.forge().await?;
    ///
    /// // 查看执行结果
    /// println!("发现: {} 个技能", report.discovered);
    /// println!("自动集成: {} 个", report.auto_integrated);
    /// println!("需人工审核: {} 个", report.manual_review);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn forge(&self) -> Result<ForgeReport> {
        // 检查是否启用 SkillForge
        if !self.config.enabled {
            warn!("SkillForge is disabled — skipping");
            // 禁用状态下返回空报告
            return Ok(ForgeReport {
                discovered: 0,
                evaluated: 0,
                auto_integrated: 0,
                manual_review: 0,
                skipped: 0,
                results: vec![],
            });
        }

        // --- 阶段 1: 发现 (Scout) ----------------------------------------------------------
        // 从配置的所有源中搜索技能候选项
        let mut candidates: Vec<ScoutResult> = Vec::new();

        // 遍历配置的每个源,依次执行发现操作
        for src in &self.config.sources {
            // 解析源类型(解析是安全的,因为源字符串来自配置)
            let source: ScoutSource = src.parse().unwrap(); // Infallible
            match source {
                ScoutSource::GitHub => {
                    // 创建 GitHub 发现器,使用配置的令牌(如果有)
                    let scout = GitHubScout::new(self.config.github_token.clone());
                    // 执行发现操作并处理结果
                    match scout.discover().await {
                        Ok(mut found) => {
                            info!(count = found.len(), "GitHub scout returned candidates");
                            // 将发现的候选项添加到总列表中
                            candidates.append(&mut found);
                        }
                        Err(e) => {
                            // 单个源失败不中断流水线,记录警告后继续
                            warn!(error = %e, "GitHub scout failed, continuing with other sources");
                        }
                    }
                }
                // 其他源尚未实现,记录信息后跳过
                ScoutSource::ClawHub | ScoutSource::HuggingFace => {
                    info!(source = src.as_str(), "Source not yet implemented — skipping");
                }
            }
        }

        self.report_from_candidates(candidates)
    }

    fn report_from_candidates(&self, mut candidates: Vec<ScoutResult>) -> Result<ForgeReport> {
        // 对候选项进行去重(基于 URL)
        // 避免不同源返回相同技能时重复处理
        scout::dedup(&mut candidates);
        let discovered = candidates.len();
        info!(discovered, "Total unique candidates after dedup");

        // --- 阶段 2: 评估 (Evaluate) -------------------------------------------------------
        // 对每个候选项进行质量评分和安全检查
        let results: Vec<EvalResult> =
            candidates.into_iter().map(|c| self.evaluator.evaluate(c)).collect();
        let evaluated = results.len();

        // --- 阶段 3: 集成 (Integrate) ------------------------------------------------------
        // 根据评估推荐执行相应的集成操作
        let mut auto_integrated = 0usize;
        let mut manual_review = 0usize;
        let mut skipped = 0usize;

        // 遍历所有评估结果,根据推荐执行相应操作
        for res in &results {
            match res.recommendation {
                Recommendation::Auto => {
                    // 推荐自动集成
                    if self.config.auto_integrate {
                        // 配置允许自动集成,执行集成操作
                        match self.integrator.integrate(&res.candidate) {
                            Ok(_) => {
                                auto_integrated += 1;
                            }
                            Err(e) => {
                                // 集成失败,记录警告但不中断流水线
                                warn!(
                                    skill = res.candidate.name.as_str(),
                                    error = %e,
                                    "Integration failed for candidate, continuing"
                                );
                            }
                        }
                    } else {
                        // 配置不允许自动集成,标记为需要人工审核
                        manual_review += 1;
                    }
                }
                Recommendation::Manual => {
                    // 推荐人工审核,不计入自动集成
                    manual_review += 1;
                }
                Recommendation::Skip => {
                    // 推荐跳过(评分不达标或有安全隐患)
                    skipped += 1;
                }
            }
        }

        // 记录流水线完成信息
        info!(auto_integrated, manual_review, skipped, "Forge pipeline complete");

        // 返回完整的执行报告
        Ok(ForgeReport { discovered, evaluated, auto_integrated, manual_review, skipped, results })
    }
}

// ---------------------------------------------------------------------------
// 测试模块 (Tests)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
