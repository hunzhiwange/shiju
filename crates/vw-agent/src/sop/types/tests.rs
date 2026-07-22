//! # SOP 类型单元测试模块
//!
//! 本模块包含对 `sop/types` 模块中定义的各种 SOP（标准操作程序）相关类型的单元测试。
//! 主要测试内容包括：
//!
//! - 枚举类型的 `Display` trait 实现（字符串表示）
//! - 序列化/反序列化（JSON、TOML）的正确性和往返一致性
//! - 结构体的默认值行为
//! - 清单文件的解析能力
//!
//! 这些测试确保 SOP 类型的核心契约在不同使用场景下保持稳定和可预测。

use super::*;

/// SOP 类型测试套件
///
/// 包含所有针对 SOP 类型的单元测试，覆盖优先级、执行模式、触发器、
/// 运行状态、步骤状态以及完整的数据结构序列化行为。
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试 `SopPriority` 枚举的 `Display` trait 实现
    ///
    /// 验证不同优先级级别转换为字符串时的正确性：
    /// - `Critical` 应显示为 `"critical"`
    /// - `Low` 应显示为 `"low"`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// assert_eq!(SopPriority::Critical.to_string(), "critical");
    /// ```
    #[test]
    fn priority_display() {
        assert_eq!(SopPriority::Critical.to_string(), "critical");
        assert_eq!(SopPriority::Low.to_string(), "low");
    }

    /// 测试 `SopExecutionMode` 枚举的 `Display` trait 实现
    ///
    /// 验证不同执行模式转换为字符串时的正确性：
    /// - `Auto` 应显示为 `"auto"`
    /// - `PriorityBased` 应显示为 `"priority_based"`
    ///
    /// 字符串表示采用 snake_case 格式，与配置文件格式保持一致。
    #[test]
    fn execution_mode_display() {
        assert_eq!(SopExecutionMode::Auto.to_string(), "auto");
        assert_eq!(SopExecutionMode::PriorityBased.to_string(), "priority_based");
    }

    /// 测试 `SopTrigger` 枚举的 `Display` trait 实现
    ///
    /// 验证不同触发器类型转换为字符串时的格式：
    /// - MQTT 触发器：格式为 `"mqtt:<topic>"`，不包含条件表达式
    /// - 手动触发器：显示为 `"manual"`
    ///
    /// 注意：条件表达式（condition 字段）不包含在显示字符串中，
    /// 仅主题（topic）用于标识触发器来源。
    #[test]
    fn trigger_display() {
        // 构造一个带条件的 MQTT 触发器
        let mqtt = SopTrigger::Mqtt {
            topic: "sensors/temp".into(),
            condition: Some("$.value > 85".into()),
        };
        // 验证显示格式为 "mqtt:topic"，条件不参与显示
        assert_eq!(mqtt.to_string(), "mqtt:sensors/temp");

        // 验证手动触发器的显示
        let manual = SopTrigger::Manual;
        assert_eq!(manual.to_string(), "manual");
    }

    /// 测试 `SopPriority` 枚举的 JSON 序列化/反序列化往返一致性
    ///
    /// 验证：
    /// 1. 序列化后的 JSON 字符串格式正确（如 `"critical"`）
    /// 2. 反序列化能正确还原原始枚举值
    ///
    /// 这是 serde 序列化框架的基本契约测试，确保配置文件
    /// 中的优先级设置能被正确解析和持久化。
    #[test]
    fn priority_serde_roundtrip() {
        // 序列化为 JSON 字符串
        let json = serde_json::to_string(&SopPriority::Critical).unwrap();
        assert_eq!(json, "\"critical\"");

        // 反序列化回枚举值，验证一致性
        let parsed: SopPriority = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SopPriority::Critical);
    }

    /// 测试 `SopExecutionMode` 枚举的 JSON 序列化/反序列化往返一致性
    ///
    /// 验证执行模式在 JSON 格式下的序列化和反序列化行为，
    /// 确保配置系统能正确读写执行模式设置。
    #[test]
    fn execution_mode_serde_roundtrip() {
        let json = serde_json::to_string(&SopExecutionMode::PriorityBased).unwrap();
        assert_eq!(json, "\"priority_based\"");

        let parsed: SopExecutionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SopExecutionMode::PriorityBased);
    }

    /// 测试 `SopTrigger` 的 TOML 解析能力
    ///
    /// 验证从 TOML 配置字符串解析 MQTT 触发器的正确性，
    /// 包括主题（topic）和条件（condition）字段的解析。
    ///
    /// 这是 SOP 清单文件加载场景的核心测试，确保用户
    /// 定义的触发器配置能被正确识别。
    #[test]
    fn trigger_toml_roundtrip() {
        // 定义 MQTT 触发器的 TOML 配置
        let toml_str = r#"
    type = "mqtt"
    topic = "facility/pump/pressure"
    condition = "$.value > 85"
    "#;

        // 解析 TOML 并验证字段映射正确
        let trigger: SopTrigger = toml::from_str(toml_str).unwrap();
        assert!(
            matches!(trigger, SopTrigger::Mqtt { ref topic, .. } if topic == "facility/pump/pressure")
        );
    }

    /// 测试手动触发器的 TOML 解析
    ///
    /// 验证最简单的触发器类型——手动触发器能从 TOML 正确解析。
    /// 手动触发器仅需要 `type = "manual"` 即可定义。
    #[test]
    fn trigger_manual_toml() {
        let toml_str = r#"type = "manual""#;
        let trigger: SopTrigger = toml::from_str(toml_str).unwrap();
        assert_eq!(trigger, SopTrigger::Manual);
    }

    /// 测试 `SopRunStatus` 枚举的 `Display` trait 实现
    ///
    /// 验证运行状态转换为字符串时的格式正确性。
    /// 例如 `WaitingApproval` 应显示为 `"waiting_approval"`。
    ///
    /// 这些字符串表示常用于日志输出、API 响应和状态监控。
    #[test]
    fn run_status_display() {
        assert_eq!(SopRunStatus::WaitingApproval.to_string(), "waiting_approval");
    }

    /// 测试 `SopStep` 结构体的默认值行为
    ///
    /// 验证当 JSON 中省略可选字段时，结构体能正确采用默认值：
    /// - `suggested_tools` 默认为空向量
    /// - `requires_confirmation` 默认为 `false`
    ///
    /// 这确保了配置文件的向后兼容性——新增可选字段不会
    /// 破坏旧配置的解析。
    #[test]
    fn step_defaults() {
        // 解析仅包含必填字段的 JSON
        let step: SopStep =
            serde_json::from_str(r#"{"number": 1, "title": "Check", "body": "Verify readings"}"#)
                .unwrap();

        // 验证可选字段采用正确的默认值
        assert!(step.suggested_tools.is_empty());
        assert!(!step.requires_confirmation);
    }

    /// 测试 `SopManifest` 结构体的完整 TOML 解析能力
    ///
    /// 验证 SOP 清单文件的核心解析功能，包括：
    /// - 基本元数据（name、description）
    /// - 多触发器配置（手动触发器和 Webhook 触发器）
    /// - 默认值的应用（priority 默认为 Normal，execution_mode 默认为 None）
    ///
    /// 这是 SOP 加载流程的关键测试，确保清单文件格式符合预期。
    #[test]
    fn manifest_parse() {
        // 定义包含多个触发器的 SOP 清单
        let toml_str = r#"
    [sop]
    name = "test-sop"
    description = "A test SOP"

    [[triggers]]
    type = "manual"

    [[triggers]]
    type = "webhook"
    path = "/sop/test"
    "#;

        // 解析并验证各字段
        let manifest: SopManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.sop.name, "test-sop");
        assert_eq!(manifest.triggers.len(), 2);
        // 验证默认值
        assert_eq!(manifest.sop.priority, SopPriority::Normal);
        assert_eq!(manifest.sop.execution_mode, None);
    }

    /// 测试 `SopTriggerSource` 枚举的 `Display` trait 实现
    ///
    /// 验证触发源类型转换为字符串时的正确性。
    /// 触发源用于标识 SOP 执行是由哪种类型的触发器发起的。
    #[test]
    fn trigger_source_display() {
        assert_eq!(SopTriggerSource::Mqtt.to_string(), "mqtt");
        assert_eq!(SopTriggerSource::Manual.to_string(), "manual");
    }

    /// 测试 `SopStepStatus` 枚举的 `Display` trait 实现
    ///
    /// 验证步骤执行状态转换为字符串时的格式正确性：
    /// - `Completed` → `"completed"`
    /// - `Failed` → `"failed"`
    /// - `Skipped` → `"skipped"`
    #[test]
    fn step_status_display() {
        assert_eq!(SopStepStatus::Completed.to_string(), "completed");
        assert_eq!(SopStepStatus::Failed.to_string(), "failed");
        assert_eq!(SopStepStatus::Skipped.to_string(), "skipped");
    }

    /// 测试 `SopEvent` 结构体的 JSON 序列化/反序列化往返一致性
    ///
    /// 验证触发事件记录在 JSON 格式下的完整序列化能力，
    /// 包括：
    /// - 触发源（source）
    /// - MQTT 主题（topic，可选）
    /// - 事件载荷（payload，可选）
    /// - 时间戳（timestamp）
    ///
    /// 这确保事件数据能被正确存储到日志系统或数据库中。
    #[test]
    fn sop_event_serde_roundtrip() {
        // 构造一个完整的 MQTT 触发事件
        let event = SopEvent {
            source: SopTriggerSource::Mqtt,
            topic: Some("sensors/pressure".into()),
            payload: Some(r#"{"value": 87.3}"#.into()),
            timestamp: "2026-02-19T12:00:00Z".into(),
        };

        // 执行序列化/反序列化往返
        let json = serde_json::to_string(&event).unwrap();
        let parsed: SopEvent = serde_json::from_str(&json).unwrap();

        // 验证所有字段正确还原
        assert_eq!(parsed.source, SopTriggerSource::Mqtt);
        assert_eq!(parsed.topic.as_deref(), Some("sensors/pressure"));
    }

    /// 测试 `SopRun` 结构体的 JSON 序列化/反序列化往返一致性
    ///
    /// 这是最复杂的数据结构测试，验证完整的 SOP 运行记录
    /// 能被正确序列化和还原，包括：
    /// - 运行标识（run_id、sop_name）
    /// - 触发事件详情（trigger_event）
    /// - 运行状态（status）
    /// - 执行进度（current_step、total_steps）
    /// - 时间信息（started_at、completed_at）
    /// - 步骤结果列表（step_results）
    /// - 等待状态（waiting_since，用于审批等待场景）
    ///
    /// 此测试覆盖了 SOP 运行时状态持久化的核心路径。
    #[test]
    fn sop_run_serde_roundtrip() {
        // 构造一个包含完整状态信息的 SOP 运行记录
        let run = SopRun {
            run_id: "run-001".into(),
            sop_name: "test-sop".into(),
            trigger_event: SopEvent {
                source: SopTriggerSource::Manual,
                topic: None,
                payload: None,
                timestamp: "2026-02-19T12:00:00Z".into(),
            },
            status: SopRunStatus::Running,
            current_step: 2,
            total_steps: 5,
            started_at: "2026-02-19T12:00:00Z".into(),
            completed_at: None,
            // 包含一个已完成的步骤结果
            step_results: vec![SopStepResult {
                step_number: 1,
                status: SopStepStatus::Completed,
                output: "Step 1 done".into(),
                started_at: "2026-02-19T12:00:00Z".into(),
                completed_at: Some("2026-02-19T12:00:05Z".into()),
            }],
            waiting_since: None,
        };

        // 执行序列化/反序列化往返
        let json = serde_json::to_string(&run).unwrap();
        let parsed: SopRun = serde_json::from_str(&json).unwrap();

        // 验证关键字段
        assert_eq!(parsed.run_id, "run-001");
        assert_eq!(parsed.status, SopRunStatus::Running);
        assert_eq!(parsed.step_results.len(), 1);
        assert_eq!(parsed.step_results[0].status, SopStepStatus::Completed);
    }

    #[test]
    fn display_contracts_cover_all_remaining_variants() {
        assert_eq!(SopPriority::Normal.to_string(), "normal");
        assert_eq!(SopPriority::High.to_string(), "high");

        assert_eq!(
            SopTrigger::Webhook { path: "/incoming".into() }.to_string(),
            "webhook:/incoming"
        );
        assert_eq!(
            SopTrigger::Cron { expression: "0 * * * *".into() }.to_string(),
            "cron:0 * * * *"
        );

        assert_eq!(SopTriggerSource::Webhook.to_string(), "webhook");
        assert_eq!(SopTriggerSource::Cron.to_string(), "cron");

        let run_statuses = [
            (SopRunStatus::Pending, "pending"),
            (SopRunStatus::Running, "running"),
            (SopRunStatus::Completed, "completed"),
            (SopRunStatus::Failed, "failed"),
            (SopRunStatus::Cancelled, "cancelled"),
        ];
        for (status, expected) in run_statuses {
            assert_eq!(status.to_string(), expected);
        }
    }

    #[test]
    fn manifest_defaults_include_version_and_execution_limits() {
        let manifest: SopManifest = toml::from_str(
            r#"
            [sop]
            name = "defaults"
            description = "Defaulted metadata"
            "#,
        )
        .unwrap();

        assert_eq!(manifest.sop.version, "0.1.0");
        assert_eq!(manifest.sop.cooldown_secs, 0);
        assert_eq!(manifest.sop.max_concurrent, 1);
        assert!(manifest.triggers.is_empty());
    }
}
