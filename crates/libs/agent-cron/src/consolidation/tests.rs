//! consolidation 模块的单元测试
//!
//! 本模块包含整合任务创建和配置的测试用例，验证以下功能：
//! - 整合任务的创建逻辑
//! - 任务调度表达式的正确性
//! - 提示词内容的完整性
//! - 自定义调度参数的应用

use super::*;

/// consolidation 模块的测试套件
///
/// 包含整合任务工厂函数的各类测试用例，确保任务配置符合预期。
#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::{JobType, Schedule, SessionTarget};
    use tempfile::TempDir;

    /// 创建用于测试的配置实例
    ///
    /// # 参数
    ///
    /// - `tmp`: 临时目录引用，用于创建测试所需的文件路径
    ///
    /// # 返回值
    ///
    /// 返回一个配置实例，其中：
    /// - `workspace_dir` 指向临时目录下的 workspace 子目录
    /// - `config_path` 指向临时目录下的 vibewindow.json 文件
    /// - 其他配置项使用默认值
    ///
    /// # 副作用
    ///
    /// 会在临时目录下创建 workspace 目录
    fn test_config(tmp: &TempDir) -> Config {
        let config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("vibewindow.json"),
            ..Config::default()
        };
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        config
    }

    /// 测试创建整合任务的基本属性是否正确
    ///
    /// 验证以下属性：
    /// - 任务名称应为 `CONSOLIDATION_JOB_NAME`
    /// - 任务类型应为 `JobType::Agent`
    /// - 会话目标应为 `SessionTarget::Isolated`
    /// - `delete_after_run` 应为 false（整合任务不应在运行后删除）
    /// - 任务应处于启用状态
    #[test]
    fn create_consolidation_job_produces_valid_job() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        let job = create_consolidation_job(&config).unwrap();

        // 验证任务名称为预定义的整合任务名称
        assert_eq!(job.name.as_deref(), Some(CONSOLIDATION_JOB_NAME));
        // 验证任务类型为 Agent 类型
        assert_eq!(job.job_type, JobType::Agent);
        // 验证会话目标为隔离模式
        assert_eq!(job.session_target, SessionTarget::Isolated);
        // 验证任务不会被自动删除
        assert!(!job.delete_after_run);
        // 验证任务默认启用
        assert!(job.enabled);
    }

    /// 测试创建整合任务时使用正确的默认调度表达式
    ///
    /// 验证任务的调度配置：
    /// - 调度表达式应为 `DEFAULT_SCHEDULE_EXPR`
    /// - 时区应为 None（使用系统默认时区）
    #[test]
    fn create_consolidation_job_uses_correct_schedule() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        let job = create_consolidation_job(&config).unwrap();

        match &job.schedule {
            Schedule::Cron { expr, tz } => {
                // 验证使用默认的 cron 表达式
                assert_eq!(expr, DEFAULT_SCHEDULE_EXPR);
                // 验证未指定时区（使用系统默认）
                assert!(tz.is_none());
            }
            other => panic!("Expected Cron schedule, got {other:?}"),
        }
    }

    /// 测试整合任务的提示词包含必要的指令和关键字
    ///
    /// 验证提示词中包含以下关键元素：
    /// - `memory_recall`: 应指导代理使用记忆召回工具
    /// - `memory_store`: 应指导代理使用记忆存储工具
    /// - `cron_runs`: 应指导代理访问 cron 运行记录
    /// - `consolidation_YYYY-MM-DD`: 应指定整合结果的键格式
    /// - `core`: 应指定使用 core 类别
    /// - `MEMORY.md`: 应提及 MEMORY.md 文件
    #[test]
    fn create_consolidation_job_prompt_contains_key_instructions() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        let job = create_consolidation_job(&config).unwrap();
        let prompt = job.prompt.expect("consolidation job must have a prompt");

        // 验证提示词包含记忆召回工具的指导
        assert!(prompt.contains("memory_recall"), "prompt should instruct use of memory_recall");
        // 验证提示词包含记忆存储工具的指导
        assert!(prompt.contains("memory_store"), "prompt should instruct use of memory_store");
        // 验证提示词包含 cron 运行记录的指导
        assert!(prompt.contains("cron_runs"), "prompt should instruct use of cron_runs");
        // 验证提示词指定了整合结果的键格式
        assert!(prompt.contains("consolidation_YYYY-MM-DD"), "prompt should specify key format");
        // 验证提示词指定了 core 类别
        assert!(prompt.contains("core"), "prompt should specify core category");
        // 验证提示词提及了 MEMORY.md 文件
        assert!(prompt.contains("MEMORY.md"), "prompt should mention MEMORY.md");
    }

    /// 测试使用自定义调度参数创建整合任务
    ///
    /// 验证：
    /// - 自定义的 cron 表达式能够正确应用
    /// - 自定义时区能够正确应用
    #[test]
    fn create_consolidation_job_with_custom_schedule_applies_tz() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        // 使用自定义的 cron 表达式和时区创建整合任务
        let job = create_consolidation_job_with_schedule(
            &config,
            "0 4 * * *",
            Some("America/New_York".into()),
        )
        .unwrap();

        match &job.schedule {
            Schedule::Cron { expr, tz } => {
                // 验证自定义的 cron 表达式已应用
                assert_eq!(expr, "0 4 * * *");
                // 验证自定义的时区已应用
                assert_eq!(tz.as_deref(), Some("America/New_York"));
            }
            other => panic!("Expected Cron schedule, got {other:?}"),
        }
    }
}
