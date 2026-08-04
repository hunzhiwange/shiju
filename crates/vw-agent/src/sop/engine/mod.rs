//! SOP 引擎模块
//!
//! 本模块提供标准操作流程（SOP）的核心编排引擎，负责 SOP 的加载、触发器匹配、
//! 运行生命周期管理等核心功能。
//!
//! # 主要功能
//!
//! - **SOP 加载与重载**：从配置目录加载 SOP 定义，支持运行时热重载
//! - **触发器匹配**：支持 MQTT、Webhook、Cron、手动等多种触发源的匹配
//! - **运行生命周期管理**：管理 SOP 实例的启动、推进、取消、完成等状态转换
//! - **并发控制**：支持全局和单 SOP 级别的并发限制
//! - **冷却时间**：支持 SOP 执行后的冷却期设置
//! - **审批流程**：支持手动审批、超时自动审批（针对高优先级 SOP）
//!
//! # 执行模式
//!
//! - **Auto**：自动执行，无需人工干预
//! - **Supervised**：监督模式，首步需要审批
//! - **StepByStep**：逐步模式，每步都需要审批
//! - **PriorityBased**：优先级模式，根据 SOP 优先级决定审批策略

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Result, bail};
use tracing::{debug, info, warn};

use super::condition::evaluate_condition;
use super::load_sops;
use super::types::{
    Sop, SopEvent, SopPriority, SopRun, SopRunAction, SopRunStatus, SopStep, SopStepResult,
    SopStepStatus, SopTrigger, SopTriggerSource,
};
use crate::app::agent::config::SopConfig;

/// SOP 引擎
///
/// 中央 SOP 编排器，负责加载 SOP 定义、匹配触发事件、管理运行实例的完整生命周期。
/// 引擎维护活跃运行和已完成运行的状态，并提供并发控制和冷却时间检查。
///
/// # 字段说明
///
/// - `sops`：已加载的 SOP 定义列表
/// - `active_runs`：当前正在执行的运行实例（按运行 ID 索引）
/// - `finished_runs`：已完成/失败/取消的运行实例（保留用于状态查询）
/// - `config`：引擎配置
/// - `run_counter`：运行 ID 计数器，用于生成唯一标识符
pub struct SopEngine {
    /// 已加载的 SOP 定义列表
    sops: Vec<Sop>,
    /// 当前活跃的运行实例映射（run_id -> SopRun）
    active_runs: HashMap<String, SopRun>,
    /// 已完成的运行实例列表（保留用于状态查询和历史记录）
    finished_runs: Vec<SopRun>,
    /// 引擎配置
    config: SopConfig,
    /// 运行 ID 计数器
    run_counter: u64,
}

impl SopEngine {
    /// 创建新的 SOP 引擎实例
    ///
    /// 使用给定的配置创建引擎。创建后需要调用 [`reload`](Self::reload) 方法加载 SOP 定义。
    ///
    /// # 参数
    ///
    /// - `config`：引擎配置，包含并发限制、冷却时间、SOP 目录等设置
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use vibe_agent::app::agent::config::SopConfig;
    /// use vibe_agent::app::agent::sop::engine::SopEngine;
    ///
    /// let config = SopConfig::default();
    /// let mut engine = SopEngine::new(config);
    /// engine.reload(std::path::Path::new("./workspace"));
    /// ```
    pub fn new(config: SopConfig) -> Self {
        Self {
            sops: Vec::new(),
            active_runs: HashMap::new(),
            finished_runs: Vec::new(),
            config,
            run_counter: 0,
        }
    }

    /// 从配置目录加载或重新加载 SOP 定义
    ///
    /// 清空当前加载的 SOP 并从配置的工作空间目录重新加载。
    /// 支持运行时热重载，无需重启服务。
    ///
    /// # 参数
    ///
    /// - `workspace_dir`：工作空间根目录路径
    ///
    /// # 说明
    ///
    /// 加载完成后会记录加载的 SOP 数量到日志。
    pub fn reload(&mut self, workspace_dir: &Path) {
        self.sops = load_sops(
            workspace_dir,
            self.config.sops_dir.as_deref(),
            self.config.default_execution_mode,
        );
        debug!("SOP engine loaded {} SOPs", self.sops.len());
    }

    /// 获取所有已加载的 SOP 定义
    ///
    /// # 返回值
    ///
    /// 返回 SOP 定义切片的引用
    pub fn sops(&self) -> &[Sop] {
        &self.sops
    }

    /// 获取所有活跃（进行中）的运行实例
    ///
    /// # 返回值
    ///
    /// 返回活跃运行映射的引用，键为运行 ID，值为运行实例
    pub fn active_runs(&self) -> &HashMap<String, SopRun> {
        &self.active_runs
    }

    /// 根据运行 ID 查找运行实例
    ///
    /// 同时搜索活跃运行和已完成的运行。
    ///
    /// # 参数
    ///
    /// - `run_id`：运行实例的唯一标识符
    ///
    /// # 返回值
    ///
    /// 找到时返回运行实例的引用，否则返回 `None`
    pub fn get_run(&self, run_id: &str) -> Option<&SopRun> {
        self.active_runs
            .get(run_id)
            .or_else(|| self.finished_runs.iter().find(|r| r.run_id == run_id))
    }

    /// 根据名称查找 SOP 定义
    ///
    /// # 参数
    ///
    /// - `name`：SOP 名称
    ///
    /// # 返回值
    ///
    /// 找到时返回 SOP 定义的引用，否则返回 `None`
    pub fn get_sop(&self, name: &str) -> Option<&Sop> {
        self.sops.iter().find(|s| s.name == name)
    }

    // ═══════════════════════════════════════════════════════════════
    // 触发器匹配
    // ═══════════════════════════════════════════════════════════════

    /// 匹配传入事件与所有已加载的 SOP 触发器
    ///
    /// 遍历所有已加载的 SOP，检查其触发器是否与给定事件匹配。
    /// 一个 SOP 可以有多个触发器，只要有一个匹配即返回该 SOP。
    ///
    /// # 参数
    ///
    /// - `event`：待匹配的触发事件
    ///
    /// # 返回值
    ///
    /// 返回所有触发器匹配的 SOP 定义引用列表
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let event = SopEvent {
    ///     source: SopTriggerSource::Webhook,
    ///     topic: Some("/api/alert".to_string()),
    ///     payload: Some(r#"{"level": "critical"}"#.to_string()),
    /// };
    /// let matching_sops = engine.match_trigger(&event);
    /// ```
    pub fn match_trigger(&self, event: &SopEvent) -> Vec<&Sop> {
        self.sops
            .iter()
            .filter(|sop| sop.triggers.iter().any(|t| trigger_matches(t, event)))
            .collect()
    }

    // ═══════════════════════════════════════════════════════════════
    // 运行生命周期管理
    // ═══════════════════════════════════════════════════════════════

    /// 检查是否可以为指定的 SOP 启动新的运行
    ///
    /// 检查以下限制条件：
    /// 1. SOP 是否存在
    /// 2. 该 SOP 的活跃运行数是否已达到最大并发限制
    /// 3. 全局活跃运行数是否已达到最大并发限制
    /// 4. 是否处于冷却期（如果 SOP 配置了冷却时间）
    ///
    /// # 参数
    ///
    /// - `sop_name`：SOP 名称
    ///
    /// # 返回值
    ///
    /// 可以启动时返回 `true`，否则返回 `false`
    pub fn can_start(&self, sop_name: &str) -> bool {
        let sop = match self.get_sop(sop_name) {
            Some(s) => s,
            None => return false,
        };

        // 检查单个 SOP 的并发限制
        let active_for_sop = self.active_runs.values().filter(|r| r.sop_name == sop_name).count();
        if active_for_sop >= sop.max_concurrent as usize {
            return false;
        }

        // 检查全局并发限制
        if self.active_runs.len() >= self.config.max_concurrent_total as usize {
            return false;
        }

        // 冷却期检查：查找该 SOP 最近一次完成的运行
        if sop.cooldown_secs > 0 {
            if let Some(last) = self.last_finished_run(sop_name) {
                if let Some(ref completed_at) = last.completed_at {
                    if !cooldown_elapsed(completed_at, sop.cooldown_secs) {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// 启动新的 SOP 运行实例
    ///
    /// 创建并初始化一个新的 SOP 运行实例，返回第一个需要执行的动作。
    /// 运行 ID 基于当前时间戳和计数器生成，确保唯一性。
    ///
    /// # 参数
    ///
    /// - `sop_name`：要执行的 SOP 名称
    /// - `event`：触发此运行的事件
    ///
    /// # 返回值
    ///
    /// 成功时返回第一个要执行的动作（`SopRunAction`），可能包括：
    /// - `ExecuteStep`：直接执行步骤
    /// - `WaitApproval`：等待审批后再执行
    ///
    /// # 错误
    ///
    /// - SOP 不存在
    /// - 并发限制或冷却期限制
    /// - SOP 没有定义任何步骤
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let event = SopEvent {
    ///     source: SopTriggerSource::Webhook,
    ///     topic: Some("/api/alert".to_string()),
    ///     payload: None,
    /// };
    /// match engine.start_run("incident-response", event)? {
    ///     SopRunAction::ExecuteStep { run_id, step, context } => {
    ///         // 立即执行步骤
    ///     }
    ///     SopRunAction::WaitApproval { run_id, step, context } => {
    ///         // 等待人工审批
    ///     }
    ///     _ => {}
    /// }
    /// ```
    pub fn start_run(&mut self, sop_name: &str, event: SopEvent) -> Result<SopRunAction> {
        let sop = self
            .get_sop(sop_name)
            .ok_or_else(|| anyhow::anyhow!("SOP not found: {sop_name}"))?
            .clone();

        if !self.can_start(sop_name) {
            bail!("Cannot start SOP '{}': cooldown or concurrency limit reached", sop_name);
        }

        if sop.steps.is_empty() {
            bail!("SOP '{}' has no steps defined", sop_name);
        }

        // 生成唯一的运行 ID：run-{epoch_ms}-{counter}
        self.run_counter += 1;
        let dur =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let epoch_ms = dur.as_secs() * 1000 + u64::from(dur.subsec_millis());
        let run_id = format!("run-{epoch_ms}-{:04}", self.run_counter);
        let now = now_iso8601();

        // 创建运行实例
        let run = SopRun {
            run_id: run_id.clone(),
            sop_name: sop_name.to_string(),
            trigger_event: event,
            status: SopRunStatus::Running,
            current_step: 1,
            total_steps: u32::try_from(sop.steps.len()).unwrap_or(u32::MAX),
            started_at: now,
            completed_at: None,
            step_results: Vec::new(),
            waiting_since: None,
        };

        self.active_runs.insert(run_id.clone(), run);

        info!("SOP run {} started for '{}'", run_id, sop_name);

        // 根据执行模式确定第一个动作
        let step = sop.steps[0].clone();
        let context = format_step_context(&sop, &self.active_runs[&run_id], &step);
        let action = resolve_step_action(&sop, &step, run_id.clone(), context);

        // 如果动作是等待审批，更新运行状态并记录时间戳
        if matches!(action, SopRunAction::WaitApproval { .. }) {
            if let Some(run) = self.active_runs.get_mut(&run_id) {
                run.status = SopRunStatus::WaitingApproval;
                run.waiting_since = Some(now_iso8601());
            }
        }

        Ok(action)
    }

    /// 报告当前步骤的执行结果并推进运行
    ///
    /// 记录步骤结果，检查是否失败，如果成功则推进到下一步骤。
    /// 如果所有步骤都已完成，则标记运行为完成状态。
    ///
    /// # 参数
    ///
    /// - `run_id`：运行实例的唯一标识符
    /// - `result`：当前步骤的执行结果
    ///
    /// # 返回值
    ///
    /// 返回下一个要执行的动作，可能是：
    /// - `ExecuteStep`：执行下一步骤
    /// - `WaitApproval`：下一步骤需要审批
    /// - `Completed`：所有步骤已完成
    /// - `Failed`：步骤失败导致运行失败
    ///
    /// # 错误
    ///
    /// - 运行 ID 不存在或不处于活跃状态
    /// - 对应的 SOP 不再加载
    pub fn advance_step(&mut self, run_id: &str, result: SopStepResult) -> Result<SopRunAction> {
        let run = self
            .active_runs
            .get_mut(run_id)
            .ok_or_else(|| anyhow::anyhow!("Active run not found: {run_id}"))?;

        let sop = self
            .sops
            .iter()
            .find(|s| s.name == run.sop_name)
            .ok_or_else(|| anyhow::anyhow!("SOP '{}' no longer loaded", run.sop_name))?
            .clone();

        // 记录步骤结果
        run.step_results.push(result.clone());

        // 检查步骤是否失败
        if result.status == SopStepStatus::Failed {
            let reason = format!("Step {} failed: {}", result.step_number, result.output);
            warn!("SOP run {run_id}: {reason}");
            return Ok(self.finish_run(run_id, SopRunStatus::Failed, Some(reason)));
        }

        // 推进到下一步骤
        let next_step_num = run.current_step + 1;
        if next_step_num > run.total_steps {
            // 所有步骤已完成
            info!("SOP run {run_id} completed successfully");
            return Ok(self.finish_run(run_id, SopRunStatus::Completed, None));
        }

        // 更新运行状态
        let run = self.active_runs.get_mut(run_id).unwrap();
        run.current_step = next_step_num;

        let step_idx = (next_step_num - 1) as usize;
        let step = sop.steps[step_idx].clone();
        let context = format_step_context(&sop, run, &step);
        let run_id_str = run_id.to_string();
        let action = resolve_step_action(&sop, &step, run_id_str.clone(), context);

        // 如果动作是等待审批，更新运行状态并记录时间戳
        if matches!(action, SopRunAction::WaitApproval { .. }) {
            if let Some(run) = self.active_runs.get_mut(&run_id_str) {
                run.status = SopRunStatus::WaitingApproval;
                run.waiting_since = Some(now_iso8601());
            }
        }

        Ok(action)
    }

    /// 取消活跃的运行实例
    ///
    /// # 参数
    ///
    /// - `run_id`：运行实例的唯一标识符
    ///
    /// # 错误
    ///
    /// - 运行 ID 不存在或不处于活跃状态
    pub fn cancel_run(&mut self, run_id: &str) -> Result<()> {
        if !self.active_runs.contains_key(run_id) {
            bail!("Active run not found: {run_id}");
        }
        self.finish_run(run_id, SopRunStatus::Cancelled, None);
        info!("SOP run {run_id} cancelled");
        Ok(())
    }

    /// 批准正在等待审批的步骤
    ///
    /// 将运行状态从 `WaitingApproval` 转换回 `Running`，
    /// 并返回需要执行的动作。
    ///
    /// # 参数
    ///
    /// - `run_id`：运行实例的唯一标识符
    ///
    /// # 返回值
    ///
    /// 返回需要执行的步骤动作
    ///
    /// # 错误
    ///
    /// - 运行 ID 不存在
    /// - 运行不在等待审批状态
    /// - 对应的 SOP 不再加载
    pub fn approve_step(&mut self, run_id: &str) -> Result<SopRunAction> {
        let run = self
            .active_runs
            .get_mut(run_id)
            .ok_or_else(|| anyhow::anyhow!("Active run not found: {run_id}"))?;

        if run.status != SopRunStatus::WaitingApproval {
            bail!("Run {run_id} is not waiting for approval (status: {})", run.status);
        }

        // 更新状态为运行中
        run.status = SopRunStatus::Running;
        run.waiting_since = None;

        let sop = self
            .sops
            .iter()
            .find(|s| s.name == run.sop_name)
            .ok_or_else(|| anyhow::anyhow!("SOP '{}' no longer loaded", run.sop_name))?
            .clone();

        let step_idx = (run.current_step - 1) as usize;
        let step = sop.steps[step_idx].clone();
        let context = format_step_context(&sop, run, &step);

        Ok(SopRunAction::ExecuteStep { run_id: run_id.to_string(), step, context })
    }

    /// 列出已完成的运行实例
    ///
    /// # 参数
    ///
    /// - `sop_name`：可选的 SOP 名称过滤条件
    ///
    /// # 返回值
    ///
    /// 返回已完成的运行实例引用列表，如果指定了 SOP 名称则只返回该 SOP 的运行
    pub fn finished_runs(&self, sop_name: Option<&str>) -> Vec<&SopRun> {
        self.finished_runs
            .iter()
            .filter(|r| sop_name.map_or(true, |name| r.sop_name == name))
            .collect()
    }

    // ═══════════════════════════════════════════════════════════════
    // 审批超时处理
    // ═══════════════════════════════════════════════════════════════

    /// 检查所有等待审批的运行是否超时
    ///
    /// 对于 Critical/High 优先级的 SOP，超时后自动审批并返回相应的动作。
    /// 对于非关键优先级的 SOP，超时后仍保持等待审批状态（除非手动审批或取消）。
    ///
    /// # 返回值
    ///
    /// 返回因超时而被自动审批的步骤动作列表
    ///
    /// # 说明
    ///
    /// - 如果 `approval_timeout_secs` 配置为 0，则禁用超时检查
    /// - 超时判断基于 `waiting_since` 时间戳
    pub fn check_approval_timeouts(&mut self) -> Vec<SopRunAction> {
        let timeout_secs = self.config.approval_timeout_secs;
        if timeout_secs == 0 {
            return Vec::new();
        }

        // 收集超时的运行及其优先级分类
        // cooldown_elapsed(ts, secs) 当 (now - ts) >= secs 时返回 true
        let timed_out: Vec<(String, bool)> = self
            .active_runs
            .values()
            .filter(|r| r.status == SopRunStatus::WaitingApproval)
            .filter(|r| {
                r.waiting_since
                    .as_deref()
                    .map_or(false, |ts| cooldown_elapsed(ts, timeout_secs as u64))
            })
            .map(|r| {
                // 判断 SOP 是否为关键/高优先级
                let is_critical =
                    self.sops.iter().find(|s| s.name == r.sop_name).map_or(false, |s| {
                        matches!(s.priority, SopPriority::Critical | SopPriority::High)
                    });
                (r.run_id.clone(), is_critical)
            })
            .collect();

        let mut actions = Vec::new();
        for (run_id, is_critical) in timed_out {
            if is_critical {
                // 自动审批：Critical/High 优先级 SOP 超时后回退到 Auto 模式
                info!(
                    "SOP run {run_id}: approval timeout — auto-approving (critical/high priority)"
                );
                match self.approve_step(&run_id) {
                    Ok(action) => actions.push(action),
                    Err(e) => warn!("SOP run {run_id}: auto-approve failed: {e}"),
                }
            } else {
                info!("SOP run {run_id}: approval timeout — waiting indefinitely (non-critical)");
            }
        }

        actions
    }

    // ═══════════════════════════════════════════════════════════════
    // 测试辅助方法
    // ═══════════════════════════════════════════════════════════════

    /// 替换已加载的 SOP 列表（仅供测试使用）
    ///
    /// 允许测试代码从其他模块直接设置 SOP 列表，而无需从文件系统加载。
    #[cfg(test)]
    pub(crate) fn set_sops_for_test(&mut self, sops: Vec<Sop>) {
        self.sops = sops;
    }

    // ═══════════════════════════════════════════════════════════════
    // 内部辅助方法
    // ═══════════════════════════════════════════════════════════════

    /// 获取指定 SOP 最近一次完成的运行
    ///
    /// # 参数
    ///
    /// - `sop_name`：SOP 名称
    ///
    /// # 返回值
    ///
    /// 返回最近一次完成的运行实例引用，如果没有则返回 `None`
    fn last_finished_run(&self, sop_name: &str) -> Option<&SopRun> {
        self.finished_runs.iter().rev().find(|r| r.sop_name == sop_name)
    }

    /// 完成运行实例并返回相应的动作
    ///
    /// 将运行从活跃列表移动到已完成列表，并处理容量限制。
    ///
    /// # 参数
    ///
    /// - `run_id`：运行实例的唯一标识符
    /// - `status`：最终状态（Completed/Failed/Cancelled）
    /// - `reason`：可选的失败原因
    ///
    /// # 返回值
    ///
    /// 返回表示运行结束的动作
    fn finish_run(
        &mut self,
        run_id: &str,
        status: SopRunStatus,
        reason: Option<String>,
    ) -> SopRunAction {
        let mut run = self.active_runs.remove(run_id).unwrap();
        run.status = status;
        run.completed_at = Some(now_iso8601());
        let sop_name = run.sop_name.clone();
        let run_id_owned = run.run_id.clone();
        self.finished_runs.push(run);

        // 当超过容量限制时，驱逐最旧的已完成运行
        let max = self.config.max_finished_runs as usize;
        if max > 0 && self.finished_runs.len() > max {
            let excess = self.finished_runs.len() - max;
            self.finished_runs.drain(..excess);
        }

        match status {
            SopRunStatus::Failed => SopRunAction::Failed {
                run_id: run_id_owned,
                sop_name,
                reason: reason.unwrap_or_default(),
            },
            _ => SopRunAction::Completed { run_id: run_id_owned, sop_name },
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 触发器匹配
// ═══════════════════════════════════════════════════════════════════

/// 检查单个触发器定义是否与传入事件匹配
///
/// 根据触发器类型和事件源进行模式匹配：
/// - **MQTT**：匹配主题模式，可选择性地评估条件表达式
/// - **Webhook**：匹配路径
/// - **Cron**：匹配 cron 表达式
/// - **Manual**：匹配手动触发
///
/// # 参数
///
/// - `trigger`：触发器定义
/// - `event`：触发事件
///
/// # 返回值
///
/// 匹配时返回 `true`，否则返回 `false`
fn trigger_matches(trigger: &SopTrigger, event: &SopEvent) -> bool {
    match (trigger, event.source) {
        (SopTrigger::Mqtt { topic, condition }, SopTriggerSource::Mqtt) => {
            // 检查 MQTT 主题是否匹配
            let topic_match =
                event.topic.as_deref().map_or(false, |t| mqtt_topic_matches(topic, t));
            if !topic_match {
                return false;
            }
            // 评估条件表达式（None 表示无条件匹配）
            match condition {
                Some(cond) => evaluate_condition(cond, event.payload.as_deref()),
                None => true,
            }
        }

        (SopTrigger::Webhook { path }, SopTriggerSource::Webhook) => {
            event.topic.as_deref().map_or(false, |t| t == path)
        }

        (SopTrigger::Cron { expression }, SopTriggerSource::Cron) => {
            event.topic.as_deref().map_or(false, |t| t == expression)
        }

        (SopTrigger::Manual, SopTriggerSource::Manual) => true,

        _ => false,
    }
}

/// MQTT 主题匹配实现
///
/// 支持 MQTT 标准通配符：
/// - `+`：单级通配符，匹配单个主题层级
/// - `#`：多级通配符，匹配零个或多个主题层级
///
/// # 参数
///
/// - `pattern`：主题模式（可包含通配符）
/// - `topic`：实际主题
///
/// # 返回值
///
/// 匹配时返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// assert!(mqtt_topic_matches("sensors/+/temperature", "sensors/room1/temperature"));
/// assert!(mqtt_topic_matches("sensors/#", "sensors/room1/temperature/current"));
/// assert!(!mqtt_topic_matches("sensors/+/temperature", "sensors/room1/humidity"));
/// ```
fn mqtt_topic_matches(pattern: &str, topic: &str) -> bool {
    let pat_parts: Vec<&str> = pattern.split('/').collect();
    let top_parts: Vec<&str> = topic.split('/').collect();

    let mut pi = 0;
    let mut ti = 0;

    while pi < pat_parts.len() && ti < top_parts.len() {
        match pat_parts[pi] {
            "#" => return true, // 多级通配符匹配剩余所有层级
            "+" => {
                // 单级通配符匹配一个层级
                pi += 1;
                ti += 1;
            }
            seg => {
                // 精确匹配
                if seg != top_parts[ti] {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
        }
    }

    // 两者都必须完全消费（除非模式以 # 结尾）
    pi == pat_parts.len() && ti == top_parts.len()
}

// ═══════════════════════════════════════════════════════════════════
// 执行模式解析
// ═══════════════════════════════════════════════════════════════════

/// 根据 SOP 执行模式确定步骤动作
///
/// 根据以下规则决定步骤是否需要审批：
///
/// | 执行模式 | 审批规则 |
/// |---------|---------|
/// | Auto | 无需审批 |
/// | Supervised | 仅首步需要审批 |
/// | StepByStep | 每步都需要审批 |
/// | PriorityBased | Critical/High 无需审批；Normal/Low 仅首步需要审批 |
///
/// # 参数
///
/// - `sop`：SOP 定义
/// - `step`：当前步骤
/// - `run_id`：运行实例 ID
/// - `context`：步骤上下文信息
///
/// # 返回值
///
/// 返回需要执行的动作：
/// - `WaitApproval`：需要等待人工审批
/// - `ExecuteStep`：可以直接执行
fn resolve_step_action(sop: &Sop, step: &SopStep, run_id: String, context: String) -> SopRunAction {
    // 标记为需要确认的步骤始终需要审批
    if step.requires_confirmation {
        return SopRunAction::WaitApproval { run_id, step: step.clone(), context };
    }

    let needs_approval = match sop.execution_mode {
        crate::app::agent::sop::SopExecutionMode::Auto => false,
        crate::app::agent::sop::SopExecutionMode::Supervised => {
            // 监督模式：仅首步需要审批
            step.number == 1
        }
        crate::app::agent::sop::SopExecutionMode::StepByStep => true,
        crate::app::agent::sop::SopExecutionMode::PriorityBased => {
            match sop.priority {
                SopPriority::Critical | SopPriority::High => false,
                SopPriority::Normal | SopPriority::Low => {
                    // 普通/低优先级采用监督模式行为
                    step.number == 1
                }
            }
        }
    };

    if needs_approval {
        SopRunAction::WaitApproval { run_id, step: step.clone(), context }
    } else {
        SopRunAction::ExecuteStep { run_id, step: step.clone(), context }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 步骤上下文格式化
// ═══════════════════════════════════════════════════════════════════

/// 构建注入到代理的结构化上下文消息
///
/// 生成包含以下信息的上下文字符串：
/// - SOP 名称和运行 ID
/// - 当前步骤进度
/// - 触发事件详情
/// - 上一步骤结果摘要
/// - 当前步骤标题和内容
/// - 建议使用的工具
///
/// # 参数
///
/// - `sop`：SOP 定义
/// - `run`：运行实例
/// - `step`：当前步骤
///
/// # 返回值
///
/// 返回格式化的上下文字符串，供代理在执行步骤时参考
fn format_step_context(sop: &Sop, run: &SopRun, step: &SopStep) -> String {
    let mut ctx = format!(
        "[SOP: {} (run {}) — Step {} of {}]\n\n",
        sop.name, run.run_id, step.number, run.total_steps
    );

    let _ = writeln!(
        ctx,
        "Trigger: {} {}",
        run.trigger_event.source,
        run.trigger_event.topic.as_deref().unwrap_or("(no topic)")
    );

    if let Some(ref payload) = run.trigger_event.payload {
        let _ = writeln!(ctx, "Payload: {payload}");
    }

    // 上一步骤摘要
    if let Some(prev) = run.step_results.last() {
        let _ =
            writeln!(ctx, "Previous: Step {} {} — {}", prev.step_number, prev.status, prev.output);
    }

    let _ = write!(ctx, "\nCurrent step: **{}**\n{}\n", step.title, step.body);

    if !step.suggested_tools.is_empty() {
        let _ = write!(ctx, "\nSuggested tools: {}\n", step.suggested_tools.join(", "));
    }

    ctx.push_str("\nWhen done, report your result.\n");

    ctx
}

// ═══════════════════════════════════════════════════════════════════
// 工具函数
// ═══════════════════════════════════════════════════════════════════

/// 生成当前时间的 ISO-8601 格式时间戳（UTC）
///
/// 不依赖 chrono 库，使用标准库实现简单的时间戳生成。
/// 格式：`YYYY-MM-DDTHH:MM:SSZ`
///
/// # 返回值
///
/// 返回 ISO-8601 格式的 UTC 时间字符串
pub(crate) fn now_iso8601() -> String {
    // 使用 SystemTime 而非 chrono 依赖
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    // 简单的 UTC 时间戳生成，无需 chrono 依赖
    let secs = now.as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // 将 epoch 以来的天数转换为 Y-M-D（简化算法，用于运行 ID 足够精确）
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// 将 Unix epoch 以来的天数转换为年月日
///
/// 使用 Howard Hinnant 的日期算法实现。
/// 参考：https://howardhinnant.github.io/date_algorithms.html
///
/// # 参数
///
/// - `days`：Unix epoch（1970-01-01）以来的天数
///
/// # 返回值
///
/// 返回元组 `(year, month, day)`
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // 算法来源：https://howardhinnant.github.io/date_algorithms.html
    days += 719_468;
    let era = days / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// 检查自指定时间戳以来是否已过去足够的冷却时间
///
/// # 参数
///
/// - `completed_at`：ISO-8601 格式的完成时间戳
/// - `cooldown_secs`：冷却秒数
///
/// # 返回值
///
/// - 已超过冷却时间：返回 `true`
/// - 未超过冷却时间：返回 `false`
/// - 时间戳解析失败：返回 `true`（允许启动，避免阻塞）
fn cooldown_elapsed(completed_at: &str, cooldown_secs: u64) -> bool {
    // 解析我们生成的 ISO-8601 时间戳
    let completed = parse_iso8601_secs(completed_at);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match completed {
        Some(ts) => now.saturating_sub(ts) >= cooldown_secs,
        None => true, // 无法解析时间戳时允许启动
    }
}

/// 最小化的 ISO-8601 时间戳解析器
///
/// 解析格式为 `YYYY-MM-DDTHH:MM:SSZ` 的时间戳，返回 epoch 以来的秒数。
///
/// # 参数
///
/// - `input`：ISO-8601 格式的时间字符串
///
/// # 返回值
///
/// 解析成功时返回 epoch 秒数，格式不匹配时返回 `None`
fn parse_iso8601_secs(input: &str) -> Option<u64> {
    // 期望格式：YYYY-MM-DDTHH:MM:SSZ
    let input = input.trim_end_matches('Z');
    let parts: Vec<&str> = input.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<u64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let time_parts: Vec<u64> = parts[1].split(':').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return None;
    }
    let (year, month, day) = (date_parts[0], date_parts[1], date_parts[2]);
    let (hour, min, sec) = (time_parts[0], time_parts[1], time_parts[2]);

    // days_to_ymd 的逆运算：计算 epoch 以来的天数
    let year_adj = if month <= 2 { year - 1 } else { year };
    let month_adj = if month > 2 { month - 3 } else { month + 9 };
    let era = year_adj / 400;
    let yoe = year_adj - era * 400;
    let doy = (153 * month_adj + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;

    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
