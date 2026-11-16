# 任务25 静态迁移证据

代码完成，编译/行为未验证。未运行任何 Cargo、构建、单元或实机测试。

## 全文件及生产符号映射

既有 session-engine 生产实现不计入本轮生产迁移。此次生产变化为删除私有 NativeSessionEngine 和 wiring 转发，所有实际调用仍经 session::runtime 公开入口进入同一领域算法；没有新业务分支。

| 来源 | 前→后行数 | 职责/去向 |

|---|---:|---|

| `runtime/host.rs` | 63→0 | 删除 NativeSessionEngine 私有转发实现；无外部构造者，具体运行入口直接绑定领域泛型。 |

| `runtime/lifecycle.rs` | 34→34 | create/load/reserve/fork/visible_tasks/validate 全组绑定 NativeSessionIo，租户目录规则仍由领域实现。 |

| `runtime/mod.rs` | 38→36 | 运行入口导出及真实宿主 lifecycle/prompt 契约挂载；保留4条生命周期和11条 prompt 真实集成场景。 |

| `runtime/prompt.rs` | 6→10 | start_prompt 绑定实际 processor::run；runner 入口绑定 AgentContextHost。 |

| `runtime/types.rs` | 11→11 | SessionToolset 绑定具体 MCP 连接；session-port 类型兼容出口。 |

| `runtime/wiring.rs` | 50→0 | 删除所有二次转发；公开路径保持，由 lifecycle/prompt 导出。 |

| `session.rs` | 65→65 | create/get/touch/update/list/remove/fork 全组绑定 NativeSessionIo；保留无项目上下文拒绝副作用的宿主接线契约。 |

| `session_tests.rs` | 20→20 | 保留 context_bound_operations_reject_missing_project_before_side_effects，覆盖真实宿主函数及回调不运行。 |

| `session_io.rs` | 20→20 | 真实安装 home/version、插件 close_hooks/close_mcp 生命周期端口，SessionIo 绑定。 |

| `skill_checkpoint.rs` | 26→23 | NativeMountRevision 调用 acp::mounts；save/restore 绑定真实端口，copy/remove 兼容调用。 |

| `skill_checkpoint_tests.rs` | 199→0 | 五条完整测试迁入 session-engine::host_migration_25_tests；原挂载删除。 |

| `status.rs` | 2→2 | 既有领域类型/服务的公共兼容重导出；实际调用仍使用该路径，保留以避免扩大跨任务改动。 |

| `store.rs` | 2→2 | 既有领域类型/服务的公共兼容重导出；实际调用仍使用该路径，保留以避免扩大跨任务改动。 |

| `summary.rs` | 12→12 | summarize 安装真实 LLM host；refresh 两入口绑定 NativeSessionIo。 |

| `system.rs` | 24→24 | 递归编排策略注入、provider host 初始化和 NativeSessionIo 绑定；既有宿主 system 测试挂载保留。 |

| `task_session.rs` | 2→2 | 既有领域类型/服务的公共兼容重导出；实际调用仍使用该路径，保留以避免扩大跨任务改动。 |

| `title.rs` | 15→15 | LLM host 初始化及标题调度绑定，保留真实 host 标题配置测试。 |

| `todo.rs` | 2→2 | 既有领域类型/服务的公共兼容重导出；实际调用仍使用该路径，保留以避免扩大跨任务改动。 |

| `turn_fork.rs` | 3→3 | TurnFork 泛型绑定 NativeSessionIo；另外两符号兼容导出。 |

| `ui_config.rs` | 2→2 | 既有领域类型/服务的公共兼容重导出；实际调用仍使用该路径，保留以避免扩大跨任务改动。 |

| `ui_store.rs` | 20→20 | 可信 agent_binding 投影经 tenant scope 读取，UI 写入边界不变。 |

| `ui_types.rs` | 3→3 | 既有领域类型/服务的公共兼容重导出；实际调用仍使用该路径，保留以避免扩大跨任务改动。 |

## 全用例映射

原 session/skill_checkpoint_tests.rs 的以下完整用例各自原名迁入 `session-engine::host_migration_25_tests`：

- `host_migration_25_tests::codex_skills_replay_checkpoint_persists_sources_and_rejects_rewound_history`

- `host_migration_25_tests::codex_skills_replay_fork_copies_only_sources_and_rewind_selects_older_checkpoint`

- `host_migration_25_tests::codex_skills_replay_checkpoint_isolates_tenant_project_session_and_agent`

- `host_migration_25_tests::codex_skills_replay_current_source_policy_and_explicit_provenance_are_revalidated`

- `host_migration_25_tests::codex_skills_replay_crlf_flags_and_unsupported_path_conditions_are_not_bypassed`

使用真实 agent-storage 与 agent-project/tenant task-local，子进程临时 root/home 隔离沿用现有迁移测试模式；移除与检查点断言无关的 LLM RecoveryConfig。MountRevision 是明确外部端口，仅返回无挂载状态，不冒充 ACP 挂载验收。copy/remove 改为领域 scope 参数入口；保留历史锚点、rewind、源正文不落盘、隔离与策略所有原断言。

其它挂载逐项保留（未借迁移声明已验收）：

- `crates/vw-agent/src/session/session_tests.rs`: `context_bound_operations_reject_missing_project_before_side_effects`

- `crates/libs/session-engine/tests/host_suites/runtime/prompt_tests.rs`: `prompt_runtime_emits_typed_tool_lifecycle`, `prompt_runtime_cancel_is_idempotent`, `shell_session_stop_cancels_owned_processes`, `shell_session_normal_return_preserves_process`, `shell_session_external_stop_reaps_returned_process`, `shell_session_transport_interrupt_preserves_process`, `shell_session_completed_handle_does_not_cancel_later_prompt_process`, `recovery_notice_runtime_forwards_progress_without_closing_consumer`, `native_tool_search_acceptance_runtime_internal_calls_keep_consumer_open`, `task_pool_recovery_prompt_terminal_precedes_confirmed_shutdown`, `task_pool_recovery_cancel_after_terminal_reaches_running_processor`

- `crates/libs/session-engine/tests/host_suites/runtime/lifecycle_tests.rs`: `load_session_rejects_directory_or_scope_mismatch`, `created_session_preserves_native_identity`, `session_tool_create_marks_visible_parent_relation`, `dynamic_tools_restore_fork_copies_specs_without_capabilities`

- `crates/libs/session-engine/tests/host_suites/system_tests.rs`: `unified_prompt_declares_instruction_priority_and_general_assistant_role`, `detect_git_from_dir_walks_up_to_parent_repo`, `detect_git_from_dir_accepts_trimmed_path_and_git_file_marker`, `resolve_is_git_falls_back_to_directory_detection_without_project_context`, `environment_from_ref_handles_missing_and_malformed_model_refs`, `environment_from_ref_preserves_parsed_ids_when_model_lookup_fails`, `agcollab_prompt_matches_runtime_capabilities`

- `crates/libs/session-engine/tests/host_suites/title_tests.rs`: `title_generation_does_not_require_a_title_agent_configuration`

## 数量与后续验证

本组宿主 619→306，净减 313 行（50.6%），不足70%。测试 219→20 行；其余为生产/兼容/注册 400→286 行，并非全部真实业务。接收测试新增 214 行，lib 注册新增3行。保留项原因见全表。

继续可迁项：公共兼容 re-export 可在调用者所属任务统一切换领域路径后删除；session.rs/lifecycle.rs 泛型绑定若改全体调用者可进一步减少，但真实 IO 生命周期仍须宿主持有；system/title 真实 host 安装不能无条件移入库。收口须核验整体减量，不将本任务当作70%达标。

依赖：vw-agent → session-engine → agent-storage/agent-skills/session-store/agent-project；无反向依赖、无 path/include 宿主源码。无需新增依赖，现有 tempfile/uuid/tokio 已齐全。

未来：`python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_25_tests -- --test-threads=1`，预期5条非零测试。宿主按上表各完整模块+函数名使用 `cargo test -p vw-agent --lib <精确名> -- --exact`；迁走原 skill_checkpoint::tests 用真实 list 核对消失，不能以零测试称通过。

静态核对：五条测试名保留、原 mod 删除、NativeSessionEngine 仅内部使用且所有替代入口参数/泛型绑定一致；未声称编译、运行或构建性能收益。
