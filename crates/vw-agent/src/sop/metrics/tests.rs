//! SOP 指标收集器单元测试模块
//!
//! 本模块提供对 `SopMetricsCollector` 的全面测试覆盖，验证以下核心功能：
//! - 计数器基准与算术运算
//! - 时间窗口过滤（7天/30天/90天）
//! - 协议遵循率计算
//! - 偏离率与完成率计算
//! - 按 SOP 名称的指标查询
//! - 快照诊断输出
//! - 从内存存储热启动恢复
//! - 环形缓冲区溢出处理
//! - 审批记录匹配与挂起状态管理

use super::*;

/// 测试模块封装
#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::sop::types::{SopEvent, SopStepResult, SopTriggerSource};
    use chrono::Utc;

    /// 创建测试用的事件对象
    ///
    /// # 返回值
    /// 返回一个手动触发、无主题、无载荷的 `SopEvent` 实例
    fn make_event() -> SopEvent {
        SopEvent {
            source: SopTriggerSource::Manual,
            topic: None,
            payload: None,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// 创建测试用的运行记录
    ///
    /// # 参数
    /// - `run_id`: 运行唯一标识符
    /// - `sop_name`: SOP 协议名称
    /// - `status`: 运行最终状态
    /// - `total_steps`: 总步骤数
    /// - `step_results`: 步骤执行结果列表
    ///
    /// # 返回值
    /// 返回一个配置完整的 `SopRun` 实例
    fn make_run(
        run_id: &str,
        sop_name: &str,
        status: SopRunStatus,
        total_steps: u32,
        step_results: Vec<SopStepResult>,
    ) -> SopRun {
        SopRun {
            run_id: run_id.into(),
            sop_name: sop_name.into(),
            trigger_event: make_event(),
            status,
            current_step: total_steps,
            total_steps,
            started_at: Utc::now().to_rfc3339(),
            completed_at: Some(Utc::now().to_rfc3339()),
            step_results,
            waiting_since: None,
        }
    }

    /// 创建测试用的步骤结果
    ///
    /// # 参数
    /// - `number`: 步骤编号
    /// - `status`: 步骤执行状态
    ///
    /// # 返回值
    /// 返回一个配置完整的 `SopStepResult` 实例
    fn make_step(number: u32, status: SopStepStatus) -> SopStepResult {
        SopStepResult {
            step_number: number,
            status,
            output: format!("Step {number}"),
            started_at: Utc::now().to_rfc3339(),
            completed_at: Some(Utc::now().to_rfc3339()),
        }
    }

    /// 测试零状态基准值
    ///
    /// 验证新建的指标收集器中所有指标都从零开始：
    /// - 完成数、失败数、取消数均为 0
    /// - 偏离率和完成率均为 0.0
    #[test]
    fn zero_state_baseline() {
        let c = SopMetricsCollector::new();
        assert_eq!(c.get_metric_value("sop.runs_completed"), Some(json!(0u64)));
        assert_eq!(c.get_metric_value("sop.runs_failed"), Some(json!(0u64)));
        assert_eq!(c.get_metric_value("sop.runs_cancelled"), Some(json!(0u64)));
        assert_eq!(c.get_metric_value("sop.deviation_rate"), Some(json!(0.0)));
        assert_eq!(c.get_metric_value("sop.completion_rate"), Some(json!(0.0)));
    }

    #[test]
    fn default_collector_has_zero_state() {
        let collector = SopMetricsCollector::default();

        assert_eq!(collector.get_metric_value("sop.runs_completed"), Some(json!(0)));
        assert_eq!(collector.snapshot()["pending_approvals"], json!(0));
    }

    #[test]
    fn poisoned_lock_fails_closed_for_writes_reads_and_snapshot() {
        let collector = std::sync::Arc::new(SopMetricsCollector::new());
        let poison_target = std::sync::Arc::clone(&collector);
        let poison_result = std::thread::spawn(move || {
            let _guard = poison_target.inner.write().unwrap();
            panic!("poison metrics lock for fail-closed test");
        })
        .join();
        assert!(poison_result.is_err());

        let run = make_run(
            "poisoned-run",
            "poisoned-sop",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        collector.record_approval("poisoned-sop", "poisoned-run");
        collector.record_timeout_auto_approve("poisoned-sop", "poisoned-run");
        collector.record_run_complete(&run);

        assert_eq!(collector.get_metric_value("sop.runs_completed"), None);
        assert_eq!(
            collector.get_metric_value_windowed(
                "sop.runs_completed",
                &std::time::Duration::from_secs(60),
            ),
            None
        );
        assert_eq!(collector.snapshot(), json!({"error": "lock poisoned"}));
    }

    /// 测试计数器算术运算
    ///
    /// 验证记录一个完成的运行后：
    /// - runs_completed 计数器递增为 1
    /// - runs_failed 保持为 0
    /// - 偏离率为 0.0（无失败步骤）
    /// - 完成率为 1.0（运行已成功完成）
    #[test]
    fn counter_arithmetic() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            3,
            vec![
                make_step(1, SopStepStatus::Completed),
                make_step(2, SopStepStatus::Completed),
                make_step(3, SopStepStatus::Completed),
            ],
        );
        c.record_run_complete(&run);

        assert_eq!(c.get_metric_value("sop.runs_completed"), Some(json!(1u64)));
        assert_eq!(c.get_metric_value("sop.runs_failed"), Some(json!(0u64)));
        assert_eq!(c.get_metric_value("sop.deviation_rate"), Some(json!(0.0)));
        assert_eq!(c.get_metric_value("sop.completion_rate"), Some(json!(1.0)));
    }

    /// 测试时间窗口过滤功能
    ///
    /// 验证窗口化指标能够正确统计：
    /// - 7 天窗口内的完成数
    /// - 30 天窗口内的完成数
    /// - 90 天窗口内的完成数
    #[test]
    fn windowed_filtering() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            2,
            vec![make_step(1, SopStepStatus::Completed), make_step(2, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        assert_eq!(c.get_metric_value("sop.runs_completed_7d"), Some(json!(1u64)));
        assert_eq!(c.get_metric_value("sop.runs_completed_30d"), Some(json!(1u64)));
        assert_eq!(c.get_metric_value("sop.runs_completed_90d"), Some(json!(1u64)));
    }

    /// 测试零步骤时的偏离率
    ///
    /// 验证当运行没有步骤时，偏离率应为 0.0（避免除零错误）
    #[test]
    fn deviation_rate_zero_steps() {
        let c = SopMetricsCollector::new();
        let run = make_run("r1", "test-sop", SopRunStatus::Completed, 0, vec![]);
        c.record_run_complete(&run);
        assert_eq!(c.get_metric_value("sop.deviation_rate"), Some(json!(0.0)));
    }

    #[test]
    fn protocol_adherence_rate_zero_defined_steps_is_zero() {
        let c = SopMetricsCollector::new();
        let run = make_run("r1", "test-sop", SopRunStatus::Completed, 0, vec![]);
        c.record_run_complete(&run);

        assert_eq!(c.get_metric_value("sop.protocol_adherence_rate"), Some(json!(0.0)));
    }

    #[test]
    fn parse_completed_at_accepts_naive_timestamp_and_rejects_invalid_timestamp() {
        let parsed = parse_completed_at("2026-07-20T06:00:00").expect("naive timestamp");
        assert_eq!(parsed.to_rfc3339(), "2026-07-20T06:00:00+00:00");

        assert_eq!(parse_completed_at("not-a-timestamp"), None);
    }

    /// 测试部分运行时的协议遵循率
    ///
    /// 验证当运行未完成所有步骤时：
    /// - 遵循率 = (执行步骤数 - 失败数 - 跳过数) / 总定义步骤数
    /// - 此处：adherence = (2 - 1 - 0) / 3 = 1/3
    #[test]
    fn protocol_adherence_rate_partial_run() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Failed,
            3,
            vec![make_step(1, SopStepStatus::Completed), make_step(2, SopStepStatus::Failed)],
        );
        c.record_run_complete(&run);

        // 遵循率 = (2 - 1 - 0) / 3 = 1/3
        let val = c.get_metric_value("sop.protocol_adherence_rate").unwrap().as_f64().unwrap();
        assert!((val - 1.0 / 3.0).abs() < 1e-10);
    }

    /// 测试完整运行时的协议遵循率
    ///
    /// 验证当所有步骤都成功完成时，遵循率为 1.0
    #[test]
    fn protocol_adherence_rate_full_run() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            2,
            vec![make_step(1, SopStepStatus::Completed), make_step(2, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        let val = c.get_metric_value("sop.protocol_adherence_rate").unwrap().as_f64().unwrap();
        assert!((val - 1.0).abs() < 1e-10);
    }

    /// 测试失败运行时的协议遵循率
    ///
    /// 验证包含失败和跳过步骤时的遵循率计算：
    /// - 遵循率 = (3 - 1 - 1) / 3 = 1/3
    #[test]
    fn protocol_adherence_rate_failed_run() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Failed,
            3,
            vec![
                make_step(1, SopStepStatus::Completed),
                make_step(2, SopStepStatus::Failed),
                make_step(3, SopStepStatus::Skipped),
            ],
        );
        c.record_run_complete(&run);

        // 遵循率 = (3 - 1 - 1) / 3 = 1/3
        let val = c.get_metric_value("sop.protocol_adherence_rate").unwrap().as_f64().unwrap();
        assert!((val - 1.0 / 3.0).abs() < 1e-10);
    }

    /// 测试派生比率指标
    ///
    /// 验证基于审批记录计算的派生指标：
    /// - 人工干预率 = 人工审批数 / 完成运行数 = 1 / 2 = 0.5
    /// - 超时审批率 = 超时自动审批数 / 完成运行数 = 1 / 2 = 0.5
    #[test]
    fn derived_rate_metrics() {
        let c = SopMetricsCollector::new();
        c.record_approval("test-sop", "r1");
        c.record_timeout_auto_approve("test-sop", "r2");

        let run1 = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        let run2 = make_run(
            "r2",
            "test-sop",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run1);
        c.record_run_complete(&run2);

        // 人工干预率 = 1 / 2 = 0.5
        let hir = c.get_metric_value("sop.human_intervention_rate").unwrap().as_f64().unwrap();
        assert!((hir - 0.5).abs() < 1e-10);

        // 超时审批率 = 1 / 2 = 0.5
        let tar = c.get_metric_value("sop.timeout_approval_rate").unwrap().as_f64().unwrap();
        assert!((tar - 0.5).abs() < 1e-10);

        assert_eq!(c.get_metric_value("sop.completion_rate"), Some(json!(1.0)));
    }

    /// 测试按 SOP 名称查询指标
    ///
    /// 验证可以通过 `sop.<sop_name>.<metric>` 格式查询特定 SOP 的指标
    #[test]
    fn per_sop_lookup() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "valve-shutdown",
            SopRunStatus::Completed,
            2,
            vec![make_step(1, SopStepStatus::Completed), make_step(2, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        assert_eq!(c.get_metric_value("sop.valve-shutdown.runs_completed"), Some(json!(1u64)));
        assert_eq!(c.get_metric_value("sop.valve-shutdown.completion_rate"), Some(json!(1.0)));
    }

    /// 测试最长匹配消歧
    ///
    /// 验证当存在名称前缀冲突时（如 "valve" 和 "valve-shutdown"），
    /// 能够正确区分不同 SOP 的指标
    #[test]
    fn longest_match_disambiguation() {
        let c = SopMetricsCollector::new();
        let r1 = make_run(
            "r1",
            "valve",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        let r2 = make_run(
            "r2",
            "valve-shutdown",
            SopRunStatus::Failed,
            2,
            vec![make_step(1, SopStepStatus::Completed), make_step(2, SopStepStatus::Failed)],
        );
        c.record_run_complete(&r1);
        c.record_run_complete(&r2);

        assert_eq!(c.get_metric_value("sop.valve-shutdown.runs_failed"), Some(json!(1u64)));
        assert_eq!(c.get_metric_value("sop.valve.runs_completed"), Some(json!(1u64)));
    }

    /// 测试未知指标返回 None
    ///
    /// 验证查询不存在的指标时返回 None，包括：
    /// - 不存在的全局指标
    /// - 非 sop 命名空间的指标
    /// - 不存在的 SOP 特定指标
    #[test]
    fn not_found_for_unknown_metric() {
        let c = SopMetricsCollector::new();
        assert_eq!(c.get_metric_value("sop.nonexistent"), None);
        assert_eq!(c.get_metric_value("other.runs_completed"), None);
        assert_eq!(c.get_metric_value("sop.no-sop.nonexistent"), None);
    }

    /// 测试审批标记传播
    ///
    /// 验证审批事件被正确记录并传播到：
    /// - 全局统计中
    /// - 7 天窗口化指标中
    #[test]
    fn approval_flag_propagation() {
        let c = SopMetricsCollector::new();
        c.record_approval("test-sop", "r1");

        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        let snap = c.snapshot();
        let global = &snap["global"];
        assert_eq!(global["human_approvals"], json!(1u64));
        assert_eq!(global["runs_completed"], json!(1u64));

        let hic = c.get_metric_value("sop.human_intervention_count_7d").unwrap().as_u64().unwrap();
        assert_eq!(hic, 1);
    }

    /// 测试挂起审批的过期清理
    ///
    /// 验证挂起的审批记录：
    /// - 在运行完成前会保留在挂起列表中
    /// - 不会立即被清理（未超过 1 小时过期阈值）
    #[test]
    fn pending_approval_stale_eviction() {
        let c = SopMetricsCollector::new();
        c.record_approval("test-sop", "orphan-run");

        {
            let state = c.inner.read().unwrap();
            assert_eq!(state.pending_approvals.len(), 1);
        }

        let run = make_run(
            "r2",
            "test-sop",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        // 孤儿条目仍然存在（尚未过期——创建时间小于 1 小时）
        {
            let state = c.inner.read().unwrap();
            assert_eq!(state.pending_approvals.len(), 1);
        }
    }

    /// 测试快照诊断输出
    ///
    /// 验证 snapshot() 方法返回的 JSON 结构包含：
    /// - global 对象（全局统计）
    /// - per_sop 对象（按 SOP 分组的统计）
    /// - 正确的字段值
    #[test]
    fn snapshot_diagnostic_output() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        let snap = c.snapshot();
        assert!(snap["global"].is_object());
        assert!(snap["per_sop"].is_object());
        assert_eq!(snap["global"]["runs_completed"], json!(1u64));
        assert_eq!(snap["global"]["recent_runs_depth"], json!(1));
        assert!(snap["per_sop"]["test-sop"].is_object());
    }

    /// 测试取消运行追踪
    ///
    /// 验证取消状态的运行：
    /// - runs_cancelled 计数器正确递增
    /// - completion_rate 为 0（取消不计入完成）
    #[test]
    fn runs_cancelled_tracking() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Cancelled,
            2,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        assert_eq!(c.get_metric_value("sop.runs_cancelled"), Some(json!(1u64)));
        let cr = c.get_metric_value("sop.completion_rate").unwrap().as_f64().unwrap();
        assert!((cr - 0.0).abs() < 1e-10);
    }

    // ── BUG 1 回归测试：单次运行多次审批 ──────────────────────────

    /// 测试单次运行多次审批的一致性
    ///
    /// 回归测试 BUG 1：验证同一运行有多个审批事件时的行为：
    /// - 全时间统计：记录 3 个审批事件
    /// - 窗口化统计：也记录 3 个事件（与全时间一致，而非 1 个运行）
    /// - 人工干预率：3 / 1 = 3.0（每个完成运行有 3 次审批事件）
    #[test]
    fn multiple_approvals_per_run_consistent() {
        let c = SopMetricsCollector::new();
        // 同一运行上的 3 个审批事件
        c.record_approval("test-sop", "r1");
        c.record_approval("test-sop", "r1");
        c.record_approval("test-sop", "r1");

        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            3,
            vec![
                make_step(1, SopStepStatus::Completed),
                make_step(2, SopStepStatus::Completed),
                make_step(3, SopStepStatus::Completed),
            ],
        );
        c.record_run_complete(&run);

        // 全时间统计：3 个事件
        assert_eq!(c.get_metric_value("sop.human_intervention_count"), Some(json!(3u64)));
        // 窗口化统计：也是 3 个事件（与全时间一致，而非按运行计数）
        assert_eq!(c.get_metric_value("sop.human_intervention_count_7d"), Some(json!(3u64)));
        // 干预率：3 / 1 = 3.0（每个完成运行有 3 次审批事件）
        let rate = c.get_metric_value("sop.human_intervention_rate").unwrap().as_f64().unwrap();
        assert!((rate - 3.0).abs() < 1e-10);
    }

    // ── 环形缓冲区溢出测试 ──────────────────────────────────────────

    /// 测试环形缓冲区溢出上限
    ///
    /// 验证当运行数超过 MAX_RECENT_RUNS 时：
    /// - 全时间计数正确统计所有 1001 次运行
    /// - 环形缓冲区被限制在 MAX_RECENT_RUNS
    /// - 窗口化指标返回最多 MAX_RECENT_RUNS 条记录
    #[test]
    fn ring_buffer_overflow_cap() {
        let c = SopMetricsCollector::new();
        for i in 0..1001u64 {
            let run = make_run(
                &format!("r{i}"),
                "test-sop",
                SopRunStatus::Completed,
                1,
                vec![make_step(1, SopStepStatus::Completed)],
            );
            c.record_run_complete(&run);
        }

        // 全时间统计：全部 1001 次运行
        assert_eq!(c.get_metric_value("sop.runs_completed"), Some(json!(1001u64)));
        // 环形缓冲区限制在 MAX_RECENT_RUNS
        let snap = c.snapshot();
        assert_eq!(snap["global"]["recent_runs_depth"], json!(MAX_RECENT_RUNS));
        // 窗口化返回最多上限值（所有近期运行都在 7 天内）
        let w = c.get_metric_value("sop.runs_completed_7d").unwrap().as_u64().unwrap();
        assert_eq!(w, MAX_RECENT_RUNS as u64);
    }

    // ── 窗口化指标排除旧运行测试 ────────────────────────────────────

    /// 测试窗口化指标排除旧运行
    ///
    /// 验证时间窗口正确过滤：
    /// - 10 天前的运行：全时间统计包含，7 天窗口排除，30 天窗口包含
    #[test]
    fn windowed_excludes_old_runs() {
        let c = SopMetricsCollector::new();
        // 直接注入一个旧运行快照（10 天前）
        {
            let mut state = c.inner.write().unwrap();
            let old_snap = RunSnapshot {
                completed_at: Utc::now() - chrono::Duration::days(10),
                terminal_status: SopRunStatus::Completed,
                steps_executed: 1,
                steps_defined: 1,
                steps_failed: 0,
                steps_skipped: 0,
                human_approval_count: 0,
                timeout_approval_count: 0,
            };
            state.global.counters.runs_completed += 1;
            state.global.counters.steps_executed += 1;
            state.global.counters.steps_defined += 1;
            state.global.recent_runs.push_back(old_snap);
        }

        // 全时间统计：1
        assert_eq!(c.get_metric_value("sop.runs_completed"), Some(json!(1u64)));
        // 7 天窗口：0（运行已超过 7 天）
        assert_eq!(c.get_metric_value("sop.runs_completed_7d"), Some(json!(0u64)));
        // 30 天窗口：1（运行在 30 天内）
        assert_eq!(c.get_metric_value("sop.runs_completed_30d"), Some(json!(1u64)));
    }

    // ── SOP 名称与指标后缀匹配（S3 边缘情况）──────────────────────

    /// 测试 SOP 名称与指标后缀匹配时的解析
    ///
    /// 边缘情况：SOP 名称为 "runs_completed"
    /// - "sop.runs_completed" 应解析为全局指标（值为 1）
    /// - 按 SOP 查询需要使用完整路径 "sop.runs_completed.runs_completed"
    #[test]
    fn sop_name_matching_metric_suffix_resolves_global() {
        let c = SopMetricsCollector::new();
        // SOP 名称为 "runs_completed" —— 一个边缘情况
        let run = make_run(
            "r1",
            "runs_completed",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        // "sop.runs_completed" 解析为全局指标（值为 1），而非按 SOP 指标
        assert_eq!(c.get_metric_value("sop.runs_completed"), Some(json!(1u64)));
        // 按 SOP 查询需要使用完整路径
        assert_eq!(c.get_metric_value("sop.runs_completed.runs_completed"), Some(json!(1u64)));
    }

    // ── 热启动测试 ──────────────────────────────────────────────────

    /// 测试热启动往返一致性
    ///
    /// 验证从内存存储重建指标收集器时：
    /// - 全局指标正确恢复
    /// - 审批计数正确恢复
    /// - 按 SOP 的指标正确恢复
    #[tokio::test]
    async fn warm_start_roundtrip() {
        let mem_cfg = crate::app::agent::config::MemoryConfig {
            backend: "sqlite".into(),
            ..crate::app::agent::config::MemoryConfig::default()
        };
        let tmp = tempfile::tempdir().unwrap();
        let memory: std::sync::Arc<dyn Memory> = std::sync::Arc::from(
            crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap(),
        );

        let audit = crate::app::agent::sop::SopAuditLogger::new(memory.clone());
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            2,
            vec![make_step(1, SopStepStatus::Completed), make_step(2, SopStepStatus::Completed)],
        );
        audit.log_run_start(&run).await.unwrap();
        audit.log_run_complete(&run).await.unwrap();
        audit.log_approval(&run, 1).await.unwrap();

        let collector = SopMetricsCollector::rebuild_from_memory(memory.as_ref()).await.unwrap();

        assert_eq!(collector.get_metric_value("sop.runs_completed"), Some(json!(1u64)));
        assert_eq!(collector.get_metric_value("sop.human_intervention_count"), Some(json!(1u64)));
        assert_eq!(collector.get_metric_value("sop.test-sop.runs_completed"), Some(json!(1u64)));
    }

    /// 测试热启动跳过运行中的运行
    ///
    /// 验证重建时只统计已终止的运行：
    /// - Running 状态的运行不计入完成数
    #[tokio::test]
    async fn warm_start_skips_running_runs() {
        let mem_cfg = crate::app::agent::config::MemoryConfig {
            backend: "sqlite".into(),
            ..crate::app::agent::config::MemoryConfig::default()
        };
        let tmp = tempfile::tempdir().unwrap();
        let memory: std::sync::Arc<dyn Memory> = std::sync::Arc::from(
            crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap(),
        );

        let audit = crate::app::agent::sop::SopAuditLogger::new(memory.clone());
        let run = SopRun {
            run_id: "r1".into(),
            sop_name: "test-sop".into(),
            trigger_event: make_event(),
            status: SopRunStatus::Running,
            current_step: 1,
            total_steps: 3,
            started_at: "2026-02-19T12:00:00Z".into(),
            completed_at: None,
            step_results: vec![],
            waiting_since: None,
        };
        audit.log_run_start(&run).await.unwrap();

        let collector = SopMetricsCollector::rebuild_from_memory(memory.as_ref()).await.unwrap();

        assert_eq!(collector.get_metric_value("sop.runs_completed"), Some(json!(0u64)));
    }

    /// 测试热启动空内存
    ///
    /// 验证从空内存存储重建时返回零状态
    #[tokio::test]
    async fn warm_start_empty_memory() {
        let mem_cfg = crate::app::agent::config::MemoryConfig {
            backend: "sqlite".into(),
            ..crate::app::agent::config::MemoryConfig::default()
        };
        let tmp = tempfile::tempdir().unwrap();
        let memory: std::sync::Arc<dyn Memory> = std::sync::Arc::from(
            crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap(),
        );

        let collector = SopMetricsCollector::rebuild_from_memory(memory.as_ref()).await.unwrap();

        assert_eq!(collector.get_metric_value("sop.runs_completed"), Some(json!(0u64)));
    }

    /// 测试热启动审批匹配
    ///
    /// 验证超时自动审批记录在热启动后正确恢复
    #[tokio::test]
    async fn warm_start_approval_matching() {
        let mem_cfg = crate::app::agent::config::MemoryConfig {
            backend: "sqlite".into(),
            ..crate::app::agent::config::MemoryConfig::default()
        };
        let tmp = tempfile::tempdir().unwrap();
        let memory: std::sync::Arc<dyn Memory> = std::sync::Arc::from(
            crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap(),
        );

        let audit = crate::app::agent::sop::SopAuditLogger::new(memory.clone());
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        audit.log_run_start(&run).await.unwrap();
        audit.log_timeout_auto_approve(&run, 1).await.unwrap();
        audit.log_run_complete(&run).await.unwrap();

        let collector = SopMetricsCollector::rebuild_from_memory(memory.as_ref()).await.unwrap();

        assert_eq!(collector.get_metric_value("sop.timeout_auto_approvals"), Some(json!(1u64)));
        let ta_7d =
            collector.get_metric_value("sop.timeout_auto_approvals_7d").unwrap().as_u64().unwrap();
        assert_eq!(ta_7d, 1);
    }

    // ── BUG 2 回归测试：热启动时非终止运行的处理 ────────────────────

    /// 测试热启动保留非终止运行的挂起审批
    ///
    /// 回归测试 BUG 2：验证非终止运行的审批在热启动后：
    /// - 全时间审批计数正确
    /// - 完成运行计数为 0（运行未终止）
    /// - 挂起审批在运行完成时被正确应用
    #[tokio::test]
    async fn warm_start_preserves_pending_for_nonterminal_runs() {
        let mem_cfg = crate::app::agent::config::MemoryConfig {
            backend: "sqlite".into(),
            ..crate::app::agent::config::MemoryConfig::default()
        };
        let tmp = tempfile::tempdir().unwrap();
        let memory: std::sync::Arc<dyn Memory> = std::sync::Arc::from(
            crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap(),
        );

        let audit = crate::app::agent::sop::SopAuditLogger::new(memory.clone());

        // 存储一个 Running（非终止）状态的运行，带有一个审批
        let running_run = SopRun {
            run_id: "r1".into(),
            sop_name: "test-sop".into(),
            trigger_event: make_event(),
            status: SopRunStatus::Running,
            current_step: 1,
            total_steps: 3,
            started_at: "2026-02-19T12:00:00Z".into(),
            completed_at: None,
            step_results: vec![],
            waiting_since: None,
        };
        audit.log_run_start(&running_run).await.unwrap();
        audit.log_approval(&running_run, 1).await.unwrap();

        // 热启动：运行未终止，审批应进入挂起状态
        let collector = SopMetricsCollector::rebuild_from_memory(memory.as_ref()).await.unwrap();

        // 全时间审批计数
        assert_eq!(collector.get_metric_value("sop.human_intervention_count"), Some(json!(1u64)));
        // 尚无完成的运行
        assert_eq!(collector.get_metric_value("sop.runs_completed"), Some(json!(0u64)));

        // 现在通过实时推送完成运行（模拟重启后的完成）
        let completed_run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            3,
            vec![
                make_step(1, SopStepStatus::Completed),
                make_step(2, SopStepStatus::Completed),
                make_step(3, SopStepStatus::Completed),
            ],
        );
        collector.record_run_complete(&completed_run);

        // 窗口化指标应反映重启前的审批
        let hic_7d = collector
            .get_metric_value("sop.human_intervention_count_7d")
            .unwrap()
            .as_u64()
            .unwrap();
        assert_eq!(hic_7d, 1);
    }

    // ── 窗口化 MetricsProvider 测试（ampersona-gates 特性）───────

    /// 测试 7 天窗口后缀与显式窗口调用一致性
    ///
    /// 验证 `get_metric_value("sop.xxx_7d")` 与
    /// `get_metric_value_windowed("sop.xxx", 7天)` 返回相同结果
    #[test]
    fn get_metric_windowed_7d_matches_suffix() {
        let c = SopMetricsCollector::new();
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            2,
            vec![make_step(1, SopStepStatus::Completed), make_step(2, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        let suffix_val = c.get_metric_value("sop.completion_rate_7d");
        let windowed_val = c.get_metric_value_windowed(
            "sop.completion_rate",
            &std::time::Duration::from_secs(7 * 86400),
        );
        assert_eq!(suffix_val, windowed_val);
    }

    /// 测试自定义时长的窗口化查询
    ///
    /// 验证可以查询任意时长窗口内的指标：
    /// - 14 天窗口：只包含 14 天内的运行
    /// - 30 天窗口：包含 30 天内的所有运行
    #[test]
    fn get_metric_windowed_custom_duration() {
        let c = SopMetricsCollector::new();
        // 记录一个近期运行
        let run = make_run(
            "r1",
            "test-sop",
            SopRunStatus::Completed,
            1,
            vec![make_step(1, SopStepStatus::Completed)],
        );
        c.record_run_complete(&run);

        // 注入一个旧运行（20 天前）
        {
            let mut state = c.inner.write().unwrap();
            let old_snap = RunSnapshot {
                completed_at: Utc::now() - chrono::Duration::days(20),
                terminal_status: SopRunStatus::Completed,
                steps_executed: 1,
                steps_defined: 1,
                steps_failed: 0,
                steps_skipped: 0,
                human_approval_count: 0,
                timeout_approval_count: 0,
            };
            state.global.recent_runs.push_back(old_snap);
        }

        // 14 天窗口：只包含近期运行
        let val = c
            .get_metric_value_windowed(
                "sop.runs_completed",
                &std::time::Duration::from_secs(14 * 86400),
            )
            .unwrap()
            .as_u64()
            .unwrap();
        assert_eq!(val, 1);

        // 30 天窗口：包含两个运行
        let val = c
            .get_metric_value_windowed(
                "sop.runs_completed",
                &std::time::Duration::from_secs(30 * 86400),
            )
            .unwrap()
            .as_u64()
            .unwrap();
        assert_eq!(val, 2);
    }
}
