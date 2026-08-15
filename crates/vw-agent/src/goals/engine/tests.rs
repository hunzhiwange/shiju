//! 目标引擎单元测试模块
//!
//! 本模块包含针对 `GoalEngine` 及其相关数据结构的完整测试套件，涵盖以下功能：
//!
//! - **目标状态持久化**：验证 `GoalState` 的 JSON 序列化/反序列化和文件 I/O
//! - **步骤选择算法**：验证 `select_next_actionable` 的优先级排序和过滤逻辑
//! - **提示词生成**：验证 `build_step_prompt` 和 `build_reflection_prompt` 的内容组装
//! - **结果解释**：验证 `interpret_result` 对成功/失败关键词的识别
//! - **停滞检测**：验证 `find_stalled_goals` 的边界条件处理
//! - **自愈反序列化**：验证未知枚举变体回退到默认值的容错机制
//!
//! ## 测试数据结构
//!
//! 测试使用 `sample_goal_state()` 函数生成标准测试数据，包含：
//! - 两个目标（g1 高优先级，g2 中优先级）
//! - 每个目标包含多个步骤，覆盖不同状态（已完成、待执行、阻塞等）
//!
//! ## 运行方式
//!
//! ```bash
//! cargo test --package vibe-agent --lib app::agent::goals::engine::tests
//! ```

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    /// 生成标准测试用目标状态数据
    ///
    /// 返回一个包含两个目标的 `GoalState` 实例：
    ///
    /// # 目标 g1（高优先级）
    /// - 描述：构建自动化平台
    /// - 状态：进行中
    /// - 步骤：
    ///   - s1：研究工具（已完成）
    ///   - s2：配置环境（待执行）
    ///   - s3：编写代码（待执行）
    /// - 上下文：使用 Python + Selenium
    ///
    /// # 目标 g2（中优先级）
    /// - 描述：学习 Rust
    /// - 状态：进行中
    /// - 步骤：
    ///   - s1：阅读书籍（待执行）
    fn sample_goal_state() -> GoalState {
        GoalState {
            goals: vec![
                Goal {
                    id: "g1".into(),
                    description: "Build automation platform".into(),
                    status: GoalStatus::InProgress,
                    priority: GoalPriority::High,
                    created_at: "2026-01-01T00:00:00Z".into(),
                    updated_at: "2026-01-01T00:00:00Z".into(),
                    steps: vec![
                        Step {
                            id: "s1".into(),
                            description: "Research tools".into(),
                            status: StepStatus::Completed,
                            result: Some("Found 3 tools".into()),
                            attempts: 1,
                        },
                        Step {
                            id: "s2".into(),
                            description: "Setup environment".into(),
                            status: StepStatus::Pending,
                            result: None,
                            attempts: 0,
                        },
                        Step {
                            id: "s3".into(),
                            description: "Write code".into(),
                            status: StepStatus::Pending,
                            result: None,
                            attempts: 0,
                        },
                    ],
                    context: "Using Python + Selenium".into(),
                    last_error: None,
                },
                Goal {
                    id: "g2".into(),
                    description: "Learn Rust".into(),
                    status: GoalStatus::InProgress,
                    priority: GoalPriority::Medium,
                    created_at: "2026-01-02T00:00:00Z".into(),
                    updated_at: "2026-01-02T00:00:00Z".into(),
                    steps: vec![Step {
                        id: "s1".into(),
                        description: "Read the book".into(),
                        status: StepStatus::Pending,
                        result: None,
                        attempts: 0,
                    }],
                    context: String::new(),
                    last_error: None,
                },
            ],
        }
    }

    /// 测试 GoalState 的 JSON 序列化/反序列化完整流程
    ///
    /// 验证：
    /// - 目标列表能够正确序列化为 JSON
    /// - JSON 能够完整还原为原始结构
    /// - 嵌套的步骤状态（如 StepStatus::Completed）正确保留
    #[test]
    fn goal_state_serde_roundtrip() {
        let state = sample_goal_state();
        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: GoalState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.goals.len(), 2);
        assert_eq!(parsed.goals[0].steps.len(), 3);
        assert_eq!(parsed.goals[0].steps[0].status, StepStatus::Completed);
    }

    /// 测试 select_next_actionable 选择最高优先级的可执行步骤
    ///
    /// 场景：存在多个进行中的目标时，应优先选择高优先级目标的下一个待执行步骤。
    ///
    /// 预期结果：g1（高优先级）的 s2 步骤应被选中，而非 g2（中优先级）的 s1。
    #[test]
    fn select_next_actionable_picks_highest_priority() {
        let state = sample_goal_state();
        let result = GoalEngine::select_next_actionable(&state);
        // g1（高优先级）的步骤 s2 应被选中，而非 g2（中优先级）
        assert_eq!(result, Some((0, 1)));
    }

    /// 测试 select_next_actionable 跳过已耗尽重试次数的步骤
    ///
    /// 场景：某个步骤的尝试次数已达到 MAX_STEP_ATTEMPTS 上限。
    ///
    /// 预期结果：该步骤被跳过，算法返回下一个可执行的步骤。
    #[test]
    fn select_next_actionable_skips_exhausted_steps() {
        let mut state = sample_goal_state();
        // 将 s2 的尝试次数设为最大值，模拟已耗尽状态
        state.goals[0].steps[1].attempts = MAX_STEP_ATTEMPTS;
        let result = GoalEngine::select_next_actionable(&state);
        // 应跳过 s2，选择 s3
        assert_eq!(result, Some((0, 2)));
    }

    /// 测试 select_next_actionable 跳过非进行中状态的目标
    ///
    /// 场景：高优先级目标已标记为完成状态。
    ///
    /// 预期结果：算法应跳过已完成目标，返回下一个进行中目标的可执行步骤。
    #[test]
    fn select_next_actionable_skips_non_in_progress_goals() {
        let mut state = sample_goal_state();
        state.goals[0].status = GoalStatus::Completed;
        let result = GoalEngine::select_next_actionable(&state);
        // g1 已完成，应选择 g2 的 s1
        assert_eq!(result, Some((1, 0)));
    }

    /// 测试 select_next_actionable 在无可执行步骤时返回 None
    ///
    /// 场景：空的目标状态（无任何目标）。
    ///
    /// 预期结果：返回 None 表示没有可执行的动作。
    #[test]
    fn select_next_actionable_returns_none_when_nothing_actionable() {
        let state = GoalState::default();
        assert!(GoalEngine::select_next_actionable(&state).is_none());
    }

    /// 测试 build_step_prompt 包含目标和步骤的关键信息
    ///
    /// 验证生成的提示词包含：
    /// - 目标描述
    /// - 当前步骤描述
    /// - 已完成步骤的历史信息
    /// - 上下文信息
    ///
    /// 同时验证：当无重试时，不应包含警告信息。
    #[test]
    fn build_step_prompt_includes_goal_and_step() {
        let state = sample_goal_state();
        let prompt = GoalEngine::build_step_prompt(&state.goals[0], &state.goals[0].steps[1]);
        assert!(prompt.contains("Build automation platform"));
        assert!(prompt.contains("Setup environment"));
        assert!(prompt.contains("Research tools"));
        assert!(prompt.contains("Using Python + Selenium"));
        // 尚未重试，不应包含警告
        assert!(!prompt.contains("WARNING"));
    }

    /// 测试 build_step_prompt 在重试场景下包含警告信息
    ///
    /// 场景：步骤已有多次尝试失败，且存在最后的错误信息。
    ///
    /// 验证生成的提示词包含：
    /// - 重试警告（WARNING）
    /// - 重试次数
    /// - 最后的错误信息
    #[test]
    fn build_step_prompt_includes_retry_warning() {
        let mut state = sample_goal_state();
        state.goals[0].steps[1].attempts = 2;
        state.goals[0].last_error = Some("connection refused".into());
        let prompt = GoalEngine::build_step_prompt(&state.goals[0], &state.goals[0].steps[1]);
        assert!(prompt.contains("WARNING"));
        assert!(prompt.contains("2 time(s)"));
        assert!(prompt.contains("connection refused"));
    }

    /// 测试 interpret_result 识别成功关键词
    ///
    /// 验证包含"Successfully"、"Done"、"completed"等关键词的结果被判定为成功。
    #[test]
    fn interpret_result_success() {
        assert!(GoalEngine::interpret_result("Successfully set up the environment"));
        assert!(GoalEngine::interpret_result("Done. All tasks completed."));
    }

    /// 测试 interpret_result 识别失败关键词
    ///
    /// 验证包含"Failed"、"Error"、"Unable"、"cannot"、"Fatal"等关键词的结果被判定为失败。
    #[test]
    fn interpret_result_failure() {
        assert!(!GoalEngine::interpret_result("Failed to install package"));
        assert!(!GoalEngine::interpret_result("Error: connection timeout occurred"));
        assert!(!GoalEngine::interpret_result("Unable to find the resource"));
        assert!(!GoalEngine::interpret_result("cannot open file"));
        assert!(!GoalEngine::interpret_result("Fatal: repository not found"));
    }

    /// 测试目标状态的文件持久化完整流程
    ///
    /// 验证：
    /// - 初始状态下文件为空
    /// - 保存后能够正确加载
    /// - 加载的数据与原始数据一致
    #[tokio::test]
    async fn load_save_state_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let engine = GoalEngine::new(tmp.path());

        // 初始状态应为空
        let empty = engine.load_state().await.unwrap();
        assert!(empty.goals.is_empty());

        // 保存并重新加载
        let state = sample_goal_state();
        engine.save_state(&state).await.unwrap();
        let loaded = engine.load_state().await.unwrap();
        assert_eq!(loaded.goals.len(), 2);
        assert_eq!(loaded.goals[0].id, "g1");
        assert_eq!(loaded.goals[1].priority, GoalPriority::Medium);
    }

    /// 测试目标优先级的排序关系
    ///
    /// 验证优先级从高到低的正确顺序：
    /// Critical > High > Medium > Low
    #[test]
    fn priority_ordering() {
        assert!(GoalPriority::Critical > GoalPriority::High);
        assert!(GoalPriority::High > GoalPriority::Medium);
        assert!(GoalPriority::Medium > GoalPriority::Low);
    }

    /// 测试 GoalStatus 的默认值为 Pending
    #[test]
    fn goal_status_default_is_pending() {
        assert_eq!(GoalStatus::default(), GoalStatus::Pending);
    }

    /// 测试 StepStatus 的默认值为 Pending
    #[test]
    fn step_status_default_is_pending() {
        assert_eq!(StepStatus::default(), StepStatus::Pending);
    }

    /// 测试 find_stalled_goals 检测包含耗尽步骤的目标
    ///
    /// 场景：目标处于进行中状态，但存在步骤已达到最大尝试次数。
    ///
    /// 预期结果：该目标被标记为停滞，返回其索引。
    #[test]
    fn find_stalled_goals_detects_exhausted_steps() {
        let state = GoalState {
            goals: vec![Goal {
                id: "g1".into(),
                description: "Stalled goal".into(),
                status: GoalStatus::InProgress,
                priority: GoalPriority::High,
                created_at: String::new(),
                updated_at: String::new(),
                steps: vec![
                    Step {
                        id: "s1".into(),
                        description: "Done step".into(),
                        status: StepStatus::Completed,
                        result: Some("ok".into()),
                        attempts: 1,
                    },
                    Step {
                        id: "s2".into(),
                        description: "Exhausted step".into(),
                        status: StepStatus::Pending,
                        result: None,
                        attempts: 3, // 已达到 MAX_STEP_ATTEMPTS
                    },
                ],
                context: String::new(),
                last_error: Some("step failed 3 times".into()),
            }],
        };

        let stalled = GoalEngine::find_stalled_goals(&state);
        assert_eq!(stalled, vec![0]);
    }

    /// 测试 find_stalled_goals 忽略有可执行步骤的目标
    ///
    /// 场景：目标的步骤尝试次数为 0，仍有执行空间。
    ///
    /// 预期结果：不返回任何停滞目标。
    #[test]
    fn find_stalled_goals_ignores_actionable_goals() {
        // 使用标准测试数据，步骤尝试次数均为 0
        let state = sample_goal_state();
        let stalled = GoalEngine::find_stalled_goals(&state);
        assert!(stalled.is_empty());
    }

    /// 测试 find_stalled_goals 忽略已完成的目标
    ///
    /// 场景：目标状态为 Completed，所有步骤也已完成。
    ///
    /// 预期结果：已完成的目标不应被标记为停滞。
    #[test]
    fn find_stalled_goals_ignores_completed_goals() {
        let state = GoalState {
            goals: vec![Goal {
                id: "g1".into(),
                description: "Done".into(),
                status: GoalStatus::Completed,
                priority: GoalPriority::Medium,
                created_at: String::new(),
                updated_at: String::new(),
                steps: vec![Step {
                    id: "s1".into(),
                    description: "Only step".into(),
                    status: StepStatus::Completed,
                    result: Some("ok".into()),
                    attempts: 1,
                }],
                context: String::new(),
                last_error: None,
            }],
        };

        let stalled = GoalEngine::find_stalled_goals(&state);
        assert!(stalled.is_empty());
    }

    /// 测试 build_reflection_prompt 包含步骤摘要信息
    ///
    /// 验证反思提示词包含：
    /// - 目标反思标记（[Goal Reflection]）
    /// - 目标描述
    /// - 步骤状态标记（[done]、[exhausted]）
    /// - 上下文信息
    /// - 最后的错误信息
    /// - 记忆存储引用
    #[test]
    fn build_reflection_prompt_includes_step_summary() {
        let goal = Goal {
            id: "g1".into(),
            description: "Test reflection".into(),
            status: GoalStatus::InProgress,
            priority: GoalPriority::High,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![
                Step {
                    id: "s1".into(),
                    description: "Completed step".into(),
                    status: StepStatus::Completed,
                    result: Some("worked".into()),
                    attempts: 1,
                },
                Step {
                    id: "s2".into(),
                    description: "Failed step".into(),
                    status: StepStatus::Pending,
                    result: None,
                    attempts: 3,
                },
            ],
            context: "some context".into(),
            last_error: Some("policy_denied".into()),
        };

        let prompt = GoalEngine::build_reflection_prompt(&goal);
        assert!(prompt.contains("[Goal Reflection]"));
        assert!(prompt.contains("Test reflection"));
        assert!(prompt.contains("[done] Completed step"));
        assert!(prompt.contains("[exhausted] Failed step"));
        assert!(prompt.contains("some context"));
        assert!(prompt.contains("policy_denied"));
        assert!(prompt.contains("memory_store"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 自愈反序列化测试
    // ═══════════════════════════════════════════════════════════════════════

    /// 测试 GoalStatus 反序列化所有有效变体
    ///
    /// 验证所有定义的枚举变体能够正确从 JSON 字符串解析。
    #[test]
    fn goal_status_deserializes_all_valid_variants() {
        let cases = vec![
            ("\"pending\"", GoalStatus::Pending),
            ("\"in_progress\"", GoalStatus::InProgress),
            ("\"completed\"", GoalStatus::Completed),
            ("\"blocked\"", GoalStatus::Blocked),
            ("\"cancelled\"", GoalStatus::Cancelled),
        ];
        for (json_str, expected) in cases {
            let parsed: GoalStatus =
                serde_json::from_str(json_str).unwrap_or_else(|e| panic!("{json_str}: {e}"));
            assert_eq!(parsed, expected, "GoalStatus mismatch for {json_str}");
        }
    }

    /// 测试 GoalStatus 自愈处理未知变体
    ///
    /// 场景：JSON 包含未知的枚举变体（如 "unknown"、"invalid"、大小写错误、空字符串）。
    ///
    /// 预期结果：未知变体回退到默认值 Pending，而非返回错误。
    #[test]
    fn goal_status_self_healing_unknown_variants() {
        for variant in &["\"unknown\"", "\"invalid\"", "\"PENDING\"", "\"IN_PROGRESS\"", "\"\""] {
            let parsed: GoalStatus =
                serde_json::from_str(variant).unwrap_or_else(|e| panic!("{variant}: {e}"));
            assert_eq!(parsed, GoalStatus::Pending);
        }
    }

    /// 测试 StepStatus 反序列化所有有效变体
    ///
    /// 验证所有定义的枚举变体能够正确从 JSON 字符串解析。
    #[test]
    fn step_status_deserializes_all_valid_variants() {
        let cases = vec![
            ("\"pending\"", StepStatus::Pending),
            ("\"in_progress\"", StepStatus::InProgress),
            ("\"completed\"", StepStatus::Completed),
            ("\"failed\"", StepStatus::Failed),
            ("\"blocked\"", StepStatus::Blocked),
        ];
        for (json_str, expected) in cases {
            let parsed: StepStatus =
                serde_json::from_str(json_str).unwrap_or_else(|e| panic!("{json_str}: {e}"));
            assert_eq!(parsed, expected, "StepStatus mismatch for {json_str}");
        }
    }

    /// 测试 StepStatus 自愈处理未知变体
    ///
    /// 场景：JSON 包含未知的枚举变体（如 "unknown"、"done"、大小写错误、空字符串）。
    ///
    /// 预期结果：未知变体回退到默认值 Pending，而非返回错误。
    #[test]
    fn step_status_self_healing_unknown_variants() {
        for variant in &["\"unknown\"", "\"done\"", "\"FAILED\"", "\"\""] {
            let parsed: StepStatus =
                serde_json::from_str(variant).unwrap_or_else(|e| panic!("{variant}: {e}"));
            assert_eq!(parsed, StepStatus::Pending);
        }
    }

    /// 测试 GoalStatus 自愈在完整目标 JSON 中的效果
    ///
    /// 场景：目标 JSON 的 status 字段包含无效值。
    ///
    /// 预期结果：整个目标仍能成功解析，status 字段回退到 Pending。
    #[test]
    fn goal_status_self_healing_in_full_goal_json() {
        let json = r#"{"id":"g1","description":"test","status":"totally_bogus","steps":[]}"#;
        let goal: Goal = serde_json::from_str(json).unwrap();
        assert_eq!(goal.status, GoalStatus::Pending);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // find_stalled_goals 边界条件测试
    // ═══════════════════════════════════════════════════════════════════════

    /// 测试 find_stalled_goals 处理空步骤列表
    ///
    /// 场景：目标处于进行中状态，但没有任何步骤。
    ///
    /// 预期结果：不返回任何停滞目标（空步骤不构成停滞条件）。
    #[test]
    fn find_stalled_goals_empty_steps_not_stalled() {
        let state = GoalState {
            goals: vec![Goal {
                id: "g1".into(),
                description: "No steps".into(),
                status: GoalStatus::InProgress,
                priority: GoalPriority::High,
                created_at: String::new(),
                updated_at: String::new(),
                steps: vec![],
                context: String::new(),
                last_error: None,
            }],
        };
        assert!(GoalEngine::find_stalled_goals(&state).is_empty());
    }

    /// 测试 find_stalled_goals 检测多个停滞目标
    ///
    /// 场景：存在多个目标，每个目标的步骤都已耗尽重试次数。
    ///
    /// 预期结果：返回所有停滞目标的索引列表。
    #[test]
    fn find_stalled_goals_multiple_stalled() {
        // 辅助闭包：创建停滞目标的工厂函数
        let stalled_goal = |id: &str| Goal {
            id: id.into(),
            description: format!("Stalled {id}"),
            status: GoalStatus::InProgress,
            priority: GoalPriority::Medium,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![Step {
                id: "s1".into(),
                description: "Exhausted".into(),
                status: StepStatus::Pending,
                result: None,
                attempts: MAX_STEP_ATTEMPTS,
            }],
            context: String::new(),
            last_error: None,
        };
        let state =
            GoalState { goals: vec![stalled_goal("g1"), stalled_goal("g2"), stalled_goal("g3")] };
        assert_eq!(GoalEngine::find_stalled_goals(&state), vec![0, 1, 2]);
    }

    /// 测试 find_stalled_goals 检测所有步骤已完成但仍为进行中状态的目标
    ///
    /// 场景：目标状态为进行中，但所有步骤都已完成。
    /// 这种情况通常表示目标状态未及时更新，视为停滞。
    ///
    /// 预期结果：该目标被标记为停滞。
    #[test]
    fn find_stalled_goals_all_steps_completed_is_stalled() {
        let state = GoalState {
            goals: vec![Goal {
                id: "g1".into(),
                description: "All done but still in-progress".into(),
                status: GoalStatus::InProgress,
                priority: GoalPriority::High,
                created_at: String::new(),
                updated_at: String::new(),
                steps: vec![
                    Step {
                        id: "s1".into(),
                        description: "Done".into(),
                        status: StepStatus::Completed,
                        result: Some("ok".into()),
                        attempts: 1,
                    },
                    Step {
                        id: "s2".into(),
                        description: "Also done".into(),
                        status: StepStatus::Completed,
                        result: Some("ok".into()),
                        attempts: 1,
                    },
                ],
                context: String::new(),
                last_error: None,
            }],
        };
        assert_eq!(GoalEngine::find_stalled_goals(&state), vec![0]);
    }

    /// 测试 find_stalled_goals 处理已完成步骤和阻塞步骤混合的情况
    ///
    /// 场景：目标包含已完成的步骤和阻塞状态的步骤，但无可执行步骤。
    ///
    /// 预期结果：该目标被标记为停滞（无法继续推进）。
    #[test]
    fn find_stalled_goals_mix_completed_and_blocked_steps() {
        let state = GoalState {
            goals: vec![Goal {
                id: "g1".into(),
                description: "Mixed".into(),
                status: GoalStatus::InProgress,
                priority: GoalPriority::High,
                created_at: String::new(),
                updated_at: String::new(),
                steps: vec![
                    Step {
                        id: "s1".into(),
                        description: "Done".into(),
                        status: StepStatus::Completed,
                        result: Some("ok".into()),
                        attempts: 1,
                    },
                    Step {
                        id: "s2".into(),
                        description: "Blocked".into(),
                        status: StepStatus::Blocked,
                        result: None,
                        attempts: 0,
                    },
                ],
                context: String::new(),
                last_error: None,
            }],
        };
        assert_eq!(GoalEngine::find_stalled_goals(&state), vec![0]);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // build_reflection_prompt 边界条件测试
    // ═══════════════════════════════════════════════════════════════════════

    /// 测试 build_reflection_prompt 在空上下文时省略该段落
    ///
    /// 场景：目标的 context 字段为空字符串。
    ///
    /// 预期结果：生成的提示词不包含 "Accumulated context" 段落。
    #[test]
    fn build_reflection_prompt_empty_context_omits_section() {
        let goal = Goal {
            id: "g1".into(),
            description: "Empty context".into(),
            status: GoalStatus::InProgress,
            priority: GoalPriority::High,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![Step {
                id: "s1".into(),
                description: "Step".into(),
                status: StepStatus::Completed,
                result: Some("ok".into()),
                attempts: 1,
            }],
            context: String::new(),
            last_error: None,
        };
        let prompt = GoalEngine::build_reflection_prompt(&goal);
        assert!(!prompt.contains("Accumulated context"));
    }

    /// 测试 build_reflection_prompt 在无最后错误时省略该段落
    ///
    /// 场景：目标的 last_error 字段为 None。
    ///
    /// 预期结果：生成的提示词不包含 "Last error" 段落。
    #[test]
    fn build_reflection_prompt_no_last_error_omits_section() {
        let goal = Goal {
            id: "g1".into(),
            description: "No error".into(),
            status: GoalStatus::InProgress,
            priority: GoalPriority::High,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![Step {
                id: "s1".into(),
                description: "Step".into(),
                status: StepStatus::Completed,
                result: Some("ok".into()),
                attempts: 1,
            }],
            context: "some ctx".into(),
            last_error: None,
        };
        let prompt = GoalEngine::build_reflection_prompt(&goal);
        assert!(!prompt.contains("Last error"));
    }

    /// 测试 build_reflection_prompt 在所有步骤完成时的标记
    ///
    /// 场景：目标的所有步骤都已完成。
    ///
    /// 预期结果：
    /// - 所有步骤都标记为 [done]
    /// - 不包含 [exhausted] 或 [blocked] 标记
    #[test]
    fn build_reflection_prompt_all_done_tags() {
        let goal = Goal {
            id: "g1".into(),
            description: "All done".into(),
            status: GoalStatus::InProgress,
            priority: GoalPriority::High,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![
                Step {
                    id: "s1".into(),
                    description: "First".into(),
                    status: StepStatus::Completed,
                    result: Some("ok".into()),
                    attempts: 1,
                },
                Step {
                    id: "s2".into(),
                    description: "Second".into(),
                    status: StepStatus::Completed,
                    result: Some("ok".into()),
                    attempts: 1,
                },
            ],
            context: String::new(),
            last_error: None,
        };
        let prompt = GoalEngine::build_reflection_prompt(&goal);
        assert!(prompt.contains("[done] First"));
        assert!(prompt.contains("[done] Second"));
        assert!(!prompt.contains("[exhausted]"));
        assert!(!prompt.contains("[blocked]"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // GoalPriority 比较和序列化测试
    // ═══════════════════════════════════════════════════════════════════════

    /// 测试 GoalPriority 的完整比较关系
    ///
    /// 验证所有优先级级别之间的比较操作正确。
    #[test]
    fn priority_all_comparisons() {
        assert!(GoalPriority::Critical > GoalPriority::High);
        assert!(GoalPriority::High > GoalPriority::Medium);
        assert!(GoalPriority::Medium > GoalPriority::Low);
        assert!(GoalPriority::Low < GoalPriority::Critical);
    }

    /// 测试 GoalPriority 所有变体的序列化/反序列化完整流程
    ///
    /// 验证所有优先级级别能够正确转换为 JSON 并还原。
    #[test]
    fn priority_serde_roundtrip_all_variants() {
        for priority in
            &[GoalPriority::Low, GoalPriority::Medium, GoalPriority::High, GoalPriority::Critical]
        {
            let json = serde_json::to_string(priority).unwrap();
            let parsed: GoalPriority = serde_json::from_str(&json).unwrap();
            assert_eq!(*priority, parsed);
        }
    }

    #[tokio::test]
    async fn load_state_empty_file_returns_default_state() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().join("state");
        tokio::fs::create_dir_all(&state_dir).await.unwrap();
        tokio::fs::write(state_dir.join("goals.json"), "").await.unwrap();

        let engine = GoalEngine::new(tmp.path());
        let state = engine.load_state().await.unwrap();

        assert!(state.goals.is_empty());
    }

    #[tokio::test]
    async fn load_state_invalid_json_returns_error() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().join("state");
        tokio::fs::create_dir_all(&state_dir).await.unwrap();
        tokio::fs::write(state_dir.join("goals.json"), "{not json").await.unwrap();

        let engine = GoalEngine::new(tmp.path());
        let error = engine.load_state().await.unwrap_err();

        assert!(error.to_string().contains("key must be a string"));
    }

    #[test]
    fn select_next_actionable_keeps_first_goal_on_equal_priority() {
        let mut state = sample_goal_state();
        state.goals[0].priority = GoalPriority::High;
        state.goals[1].priority = GoalPriority::High;

        assert_eq!(GoalEngine::select_next_actionable(&state), Some((0, 1)));
    }

    #[test]
    fn select_next_actionable_ignores_non_pending_steps() {
        let state = GoalState {
            goals: vec![Goal {
                id: "g1".into(),
                description: "No pending work".into(),
                status: GoalStatus::InProgress,
                priority: GoalPriority::Critical,
                created_at: String::new(),
                updated_at: String::new(),
                steps: vec![
                    Step {
                        id: "s1".into(),
                        description: "Running".into(),
                        status: StepStatus::InProgress,
                        result: None,
                        attempts: 0,
                    },
                    Step {
                        id: "s2".into(),
                        description: "Failed".into(),
                        status: StepStatus::Failed,
                        result: None,
                        attempts: 0,
                    },
                ],
                context: String::new(),
                last_error: None,
            }],
        };

        assert_eq!(GoalEngine::select_next_actionable(&state), None);
    }

    #[test]
    fn build_step_prompt_uses_no_result_for_completed_step_without_result() {
        let goal = Goal {
            id: "g1".into(),
            description: "Handle missing result".into(),
            status: GoalStatus::InProgress,
            priority: GoalPriority::Medium,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![
                Step {
                    id: "s1".into(),
                    description: "Already done".into(),
                    status: StepStatus::Completed,
                    result: None,
                    attempts: 1,
                },
                Step {
                    id: "s2".into(),
                    description: "Continue".into(),
                    status: StepStatus::Pending,
                    result: None,
                    attempts: 0,
                },
            ],
            context: String::new(),
            last_error: None,
        };

        let prompt = GoalEngine::build_step_prompt(&goal, &goal.steps[1]);

        assert!(prompt.contains("- [done] Already done: (no result)"));
        assert!(!prompt.contains("Context so far"));
    }

    #[test]
    fn build_step_prompt_retry_without_last_error_uses_unknown() {
        let mut state = sample_goal_state();
        state.goals[0].steps[1].attempts = 1;
        state.goals[0].last_error = None;

        let prompt = GoalEngine::build_step_prompt(&state.goals[0], &state.goals[0].steps[1]);

        assert!(prompt.contains("Last error: unknown"));
    }

    #[test]
    fn interpret_result_is_case_insensitive_and_matches_all_indicators() {
        for output in [
            "FAILED TO create file",
            "Could not connect",
            "PANIC: unreachable",
            "The process CANNOT continue",
        ] {
            assert!(!GoalEngine::interpret_result(output), "{output}");
        }
        assert!(GoalEngine::interpret_result("No failure keywords here"));
    }

    #[test]
    fn max_step_attempts_exposes_retry_limit() {
        assert_eq!(GoalEngine::max_step_attempts(), MAX_STEP_ATTEMPTS);
    }

    #[test]
    fn build_reflection_prompt_tags_failed_blocked_pending_and_no_result() {
        let goal = Goal {
            id: "g1".into(),
            description: "Mixed tags".into(),
            status: GoalStatus::InProgress,
            priority: GoalPriority::Medium,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![
                Step {
                    id: "s1".into(),
                    description: "Failed step".into(),
                    status: StepStatus::Failed,
                    result: Some("failed".into()),
                    attempts: 1,
                },
                Step {
                    id: "s2".into(),
                    description: "Blocked step".into(),
                    status: StepStatus::Blocked,
                    result: None,
                    attempts: 0,
                },
                Step {
                    id: "s3".into(),
                    description: "Fresh pending".into(),
                    status: StepStatus::Pending,
                    result: None,
                    attempts: 0,
                },
            ],
            context: String::new(),
            last_error: None,
        };

        let prompt = GoalEngine::build_reflection_prompt(&goal);

        assert!(prompt.contains("[blocked] Failed step: failed"));
        assert!(prompt.contains("[blocked] Blocked step: (no result)"));
        assert!(prompt.contains("[pending] Fresh pending: (no result)"));
    }
}
