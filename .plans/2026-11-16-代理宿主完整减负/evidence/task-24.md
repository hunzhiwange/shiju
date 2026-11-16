# 任务24 静态交付证据

代码完成，编译/行为未验证。按本轮明确约束未运行任何 Cargo、构建、单元/实机测试。

## 实际变化与依赖

- session-engine::fork_execution::project 接收 home/tenant/Info，宿主真实 project 调用该入口；原状态投影分支下沉，测试直接使用真实 SQLite 与显式租户。保留薄宿主租户注入断言。

- llm-runtime 自注册 host_migration_24_tests，迁移完整 Claude/Codex 缓存淘汰、生命周期、供应商 fail-closed、AI SDK 依赖契约组；复用任务20最小 CacheHost，仅扩大测试态可见性，不复制 App，不反向依赖 vw-agent。

- 删除未再调用的宿主 AI SDK driver 测试转发，删除旧领域用例挂载。session-engine 注册补丁同时消除末尾重复 task23 声明，原 task23 注册保留。无依赖新增。

## 减量口径

52 个宿主来源：2960 → 2678 行，净减 282（9.5%），未达 70%。下表逐文件说明保留原因与继续可迁项，需收口任务验收，不声称整轮门槛达成。

本轮接收文件共 283 行测试；另有 llm-runtime 注册/夹具接线与 session-engine 投影生产入口。宿主生产/测试按文件口径：原 778/2182；现 738/1940。此粗分包含 cfg(test) 注册，不冒充生产收益：真实生产仅 project 分支，其余为测试迁移与转发删除。

## 全文件、符号与用例映射

### `crates/vw-agent/src/session/dynamic_tool_acceptance_tests.rs`

495 → 495 行。真实 HTTP Gateway/SDK/SQLite 鉴权、会话 fork、回调与模型搜索端到端验收；Fixture/new/directory/register/replace/entry/context/drop/replacement/response 只服务此真实边界。继续可迁：宿主路由与生命周期端口进一步收敛后再审查夹具。

符号：new, directory, register, replace, entry, context, drop, replacement, response, dynamic_tools_acceptance_replacement_survives_resume, host_stream, next_call, execute, dynamic_tools_acceptance_default_policy_and_cross_session_are_denied, dynamic_tools_acceptance_replacement_cancellation_and_revocation, dynamic_tools_acceptance_fork_requires_new_host_and_matching_specs, dynamic_tools_acceptance_search_then_call_uses_gateway_model_boundary

- `dynamic_tools_acceptance_replacement_survives_resume` → `crates/vw-agent/src/session/dynamic_tool_acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `dynamic_tools_acceptance_default_policy_and_cross_session_are_denied` → `crates/vw-agent/src/session/dynamic_tool_acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `dynamic_tools_acceptance_replacement_cancellation_and_revocation` → `crates/vw-agent/src/session/dynamic_tool_acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `dynamic_tools_acceptance_fork_requires_new_host_and_matching_specs` → `crates/vw-agent/src/session/dynamic_tool_acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `dynamic_tools_acceptance_search_then_call_uses_gateway_model_boundary` → `crates/vw-agent/src/session/dynamic_tool_acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/dynamic_tool_store.rs`

23 → 23 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：load_scoped, copy_scoped

### `crates/vw-agent/src/session/dynamic_tool_store_tests.rs`

128 → 128 行。ClientTool 使用 AgentContextHost，验证恢复规格不能恢复宿主能力；fixture 建真实 ProjectStore/SessionStore，不能以假的工具执行器替换。继续可迁：已有最小 ContextHost 夹具可复用时整体迁 session-engine。

符号：name, description, parameters_schema, execute, tools, fixture, fixture_project, dynamic_tools_restore_client_specs_do_not_restore_host_capabilities

- `dynamic_tools_restore_client_specs_do_not_restore_host_capabilities` → `crates/vw-agent/src/session/dynamic_tool_store_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/fork_execution.rs`

40 → 36 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：scope, load_scoped, create, remove, start, is_selectable, project

### `crates/vw-agent/src/session/fork_execution_tests.rs`

51 → 9 行。仅保留当前宿主 task-local 租户注入契约，持久化/投影整条用例迁 session-engine。

符号：fork_execution_boundary_and_start_are_durable_and_scoped

- `fork_execution_boundary_and_start_are_durable_and_scoped` → `crates/libs/session-engine/src/host_migration_24_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/instruction.rs`

15 → 15 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/instruction_discovery.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/instruction_rules.rs`

1 → 1 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`

639 → 639 行。真实 CLI 模型切换、取消、effort 与 observe_rpc 证据；accepted 及其反例与真实验收共享，避免另增仅测试调用的生产验证 API。继续可迁：外部验收独立 crate 后迁整组。

符号：accepted, acp_model_source_gemini_harness_requires_exact_model_and_generation_path, acp_model_source_harness_contract_no_model_request_is_failure, acp_model_source_harness_contract_handshake_is_not_model_success, acp_model_source_harness_contract_forged_model_is_failure, acp_model_source_harness_contract_rejects_unauthenticated_or_wrong_model, evidence, request, request_in_session, checkpoint, run, effort_run, acp_model_source_opencode_effort_real_cli, acp_model_source_claude_effort_real_cli, acp_model_source_copilot_effort_real_cli, acp_model_source_real_cli_session_model, acp_model_source_real_cli_cli_local, observe_rpc

- `acp_model_source_gemini_harness_requires_exact_model_and_generation_path` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_harness_contract_no_model_request_is_failure` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_harness_contract_handshake_is_not_model_success` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_harness_contract_forged_model_is_failure` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_harness_contract_rejects_unauthenticated_or_wrong_model` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_opencode_effort_real_cli` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_claude_effort_real_cli` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_copilot_effort_real_cli` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_real_cli_session_model` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_real_cli_cli_local` → `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/llm/acp/model_source/gateway.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/acp/model_source/mod.rs`

80 → 80 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：prepare, launch_identity

### `crates/vw-agent/src/session/llm/acp/model_source/profile.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/acp/model_source/profile_tests.rs`

135 → 102 行。仅保留真实租户项目配置、RelayServer HTTP、旧凭据失效及引用重建验收；纯驱动拒绝用例已迁。

符号：identity, acp_model_source_launch_profile_all_drivers_fail_closed_without_secret_errors, execute, acp_model_source_launch_profile_http_reissue_isolates_old_credentials, drop

- `acp_model_source_launch_profile_all_drivers_fail_closed_without_secret_errors` → `crates/libs/llm-runtime/src/host_migration_24_tests/profile.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_model_source_launch_profile_http_reissue_isolates_old_credentials` → `crates/vw-agent/src/session/llm/acp/model_source/profile_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`

471 → 471 行。真实 CLI 技能发现/脚本/附件与宿主 agent_defaults/初始提示词验收；accepted 反例共享同一验收判据。继续可迁：外部验收独立 crate 后整体迁。

符号：accepted, acp_skill_mount_harness_zero_requests_never_pass, acp_skill_mount_harness_old_model_report_never_passes, acp_skill_mount_harness_requires_discovery_clean_prompt_and_real_results, acp_skill_mount_harness_initial_prompt_contains_no_body, fixture, request, checkpoint, defaults, run, acp_skill_mount_real_codex, acp_skill_mount_real_claude, acp_skill_mount_real_qwen, acp_skill_mount_real_gemini, acp_skill_mount_real_codebuddy, acp_skill_mount_real_goose, acp_skill_mount_real_vibe, acp_skill_mount_real_hermes, acp_skill_mount_real_snow, acp_skill_mount_real_copilot, acp_skill_mount_real_kilocode, acp_skill_mount_real_kimi, acp_skill_mount_real_kiro, acp_skill_mount_real_opencode, acp_skill_mount_real_openclaw, acp_skill_mount_real_pi, acp_skill_mount_real_auggie, acp_skill_mount_real_cursor, acp_skill_mount_real_droid, acp_skill_mount_real_qoder, acp_skill_mount_real_trae, acp_skill_mount_real_iflow

- `acp_skill_mount_harness_zero_requests_never_pass` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_harness_old_model_report_never_passes` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_harness_requires_discovery_clean_prompt_and_real_results` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_harness_initial_prompt_contains_no_body` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_codex` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_claude` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_qwen` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_gemini` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_codebuddy` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_goose` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_vibe` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_hermes` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_snow` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_copilot` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_kilocode` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_kimi` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_kiro` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_opencode` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_openclaw` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_pi` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_auggie` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_cursor` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_droid` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_qoder` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_trae` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_real_iflow` → `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/llm/acp/skill_mounts/claude_tests.rs`

29 → 0 行。整组迁出，宿主原文件及挂载删除。

符号：acp_skill_mount_claude_retires_changed_owner_and_preserves_other_sessions

- `acp_skill_mount_claude_retires_changed_owner_and_preserves_other_sessions` → `crates/libs/llm-runtime/src/host_migration_24_tests/claude.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/llm/acp/skill_mounts/codex_tests.rs`

29 → 0 行。整组迁出，宿主原文件及挂载删除。

符号：acp_skill_mount_codex_retires_changed_owner_and_preserves_other_sessions

- `acp_skill_mount_codex_retires_changed_owner_and_preserves_other_sessions` → `crates/libs/llm-runtime/src/host_migration_24_tests/codex.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/llm/acp/skill_mounts/lifecycle_tests.rs`

109 → 0 行。整组迁出，宿主原文件及挂载删除。

符号：acp_skill_mount_lifecycle_scanner_change_retires_only_own_cached_client, acp_skill_mount_lifecycle_busy_rejects_same_session_but_allows_other_session, acp_skill_mount_lifecycle_content_removal_and_session_change_identity

- `acp_skill_mount_lifecycle_scanner_change_retires_only_own_cached_client` → `crates/libs/llm-runtime/src/host_migration_24_tests/lifecycle_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_lifecycle_busy_rejects_same_session_but_allows_other_session` → `crates/libs/llm-runtime/src/host_migration_24_tests/lifecycle_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `acp_skill_mount_lifecycle_content_removal_and_session_change_identity` → `crates/libs/llm-runtime/src/host_migration_24_tests/lifecycle_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/llm/acp/skill_mounts/mod.rs`

13 → 7 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：prepare

### `crates/vw-agent/src/session/llm/aisdk/convert.rs`

6 → 6 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/aisdk/driver.rs`

28 → 0 行。整组迁出，宿主原文件及挂载删除。

符号：dispatch_stream_request

### `crates/vw-agent/src/session/llm/aisdk/error.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/aisdk/host.rs`

14 → 14 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：begin_usage, authorize_model_request

### `crates/vw-agent/src/session/llm/aisdk/mod.rs`

31 → 29 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/aisdk/reasoning.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/aisdk/request.rs`

29 → 29 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：do_stream_request_aisdk

### `crates/vw-agent/src/session/llm/aisdk/stream.rs`

34 → 34 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：do_stream_request_aisdk_with_model

### `crates/vw-agent/src/session/llm/logging.rs`

1 → 1 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/messages.rs`

14 → 14 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：build_system_messages

### `crates/vw-agent/src/session/llm/messages_system_tests.rs`

85 → 85 行。验证 ensure_host 安装的真实 session system renderer，统一提示词标记与非空来源顺序。

符号：build_system_messages_merges_non_empty_sources_in_order, build_system_messages_does_not_duplicate_unified_prompt_for_codex, build_system_messages_keeps_pre_rendered_runtime_prompt_once, build_system_messages_filters_blank_optional_sources_and_keeps_provider_prompt

- `build_system_messages_merges_non_empty_sources_in_order` → `crates/vw-agent/src/session/llm/messages_system_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `build_system_messages_does_not_duplicate_unified_prompt_for_codex` → `crates/vw-agent/src/session/llm/messages_system_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `build_system_messages_keeps_pre_rendered_runtime_prompt_once` → `crates/vw-agent/src/session/llm/messages_system_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

- `build_system_messages_filters_blank_optional_sources_and_keeps_provider_prompt` → `crates/vw-agent/src/session/llm/messages_system_tests.rs`（名称、场景及断言保留；输入改为显式领域端口时不覆盖宿主 task-local 契约）。

### `crates/vw-agent/src/session/llm/mod.rs`

84 → 84 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/model_selector.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/model_selector_tests.rs`

1 → 1 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/native_tools.rs`

1 → 1 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/options.rs`

1 → 1 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm/prompt.rs`

119 → 119 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：ensure_host, resolve_model, acp_model_source, is_acp_model, resolve_auto_model, model_limit_for_request, stream_prompt, stream_prompt_with_tools, stream_chat_with_tools, stream_chat_with_tools_for_session, stream_chat_with_tools_for_session_with_abort, send_prompt, resolve_model_for_options

### `crates/vw-agent/src/session/llm/recovery_test_support.rs`

10 → 10 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：new

### `crates/vw-agent/src/session/llm/stream.rs`

37 → 37 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：sdk_host, project_id, client_id, emit_model, stream

### `crates/vw-agent/src/session/llm/types.rs`

6 → 6 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/llm_raw.rs`

3 → 3 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/message.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/mod.rs`

81 → 81 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/processor/utils/docs.rs`

3 → 3 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/processor/utils/file_links.rs`

7 → 7 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/processor/utils/message_format.rs`

6 → 6 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/processor/utils/output_compaction.rs`

3 → 3 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/processor/utils/session_ingest.rs`

26 → 26 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：push_user_dedup, ingest_assistant_answer, ingest_user_query

### `crates/vw-agent/src/session/processor/utils/tool_parsing.rs`

4 → 4 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：query_has_any_tool_calls

### `crates/vw-agent/src/session/prompt.rs`

17 → 17 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：environment_from_ref, workspace_resolved, system_resolved, recursive_policy, system_with_metadata, system_with_agent_metadata, system, environment

### `crates/vw-agent/src/session/retry.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/revert.rs`

10 → 10 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：revert, unrevert, cleanup

### `crates/vw-agent/src/session/runtime/dynamic_calls.rs`

2 → 2 行。现有公共兼容重导出/模块声明或测试注册；不计为本轮生产迁移收益。继续可迁：直接调用者统一入口后消除兼容路径。

符号：重导出/注册，无自定义函数。

### `crates/vw-agent/src/session/runtime/dynamic_hosts.rs`

23 → 23 行。现有宿主类型特化、租户/目录/配置/权限/事件端口注入或公共兼容入口；生产算法已有领域所有者，不计本轮迁移收益。继续可迁：直接调用者统一领域入口后删除转发。

符号：client_tool, search_tool, deref, register, authenticate, replace, calls, request_catalog

## 任务1跨目录挂载全量核对

下列外部源码仍由宿主编译（driver_tests 已迁），不因物理位于 libs 宣称完成领域自注册。宿主配置、全局 provider Host、工具上下文与真实请求鉴权相关依赖继续保留；可继续迁的纯算法子组由收口审查，不以零测试去重。

### `crates/vw-agent/src/session/instruction.rs` → `crates/libs/session-engine/tests/host_suites/instruction_tests.rs`

414 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `claim_is_scoped_by_message_id_and_clear_removes_claims` → 原宿主模块，原断言保留。

- `agent_workspace_identity_reads_only_enabled_non_empty_known_files` → 原宿主模块，原断言保留。

- `agent_workspace_identity_empty_whitelist_loads_nothing` → 原宿主模块，原断言保留。

- `fetch_url_reads_successful_response_and_returns_empty_for_http_errors` → 原宿主模块，原断言保留。

- `fetch_url_returns_empty_for_invalid_url` → 原宿主模块，原断言保留。

- `loaded_collects_uncompacted_read_and_file_read_metadata` → 原宿主模块，原断言保留。

- `loaded_ignores_non_array_metadata_and_deduplicates_paths` → 原宿主模块，原断言保留。

- `find_reads_only_agents_files` → 原宿主模块，原断言保留。

- `find_ignores_context_when_it_is_the_only_candidate` → 原宿主模块，原断言保留。

- `resolve_claims_unreadable_candidate_without_returning_content` → 原宿主模块，原断言保留。

- `resolve_walks_up_from_target_directory_skips_loaded_files_and_claims_per_message` → 原宿主模块，原断言保留。

- `resolve_skips_target_instruction_file_and_empty_instruction_content` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/messages.rs` → `crates/libs/llm-runtime/src/messages_tests.rs`

208 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `resolve_tools_removes_user_disabled_and_permission_denied_tools` → 原宿主模块，原断言保留。

- `resolve_tools_keeps_allowed_and_explicitly_enabled_tools_without_mutating_input` → 原宿主模块，原断言保留。

- `has_tool_calls_detects_all_supported_shapes` → 原宿主模块，原断言保留。

- `has_tool_calls_ignores_empty_or_unrelated_messages` → 原宿主模块，原断言保留。

- `has_tool_calls_ignores_malformed_tool_shapes` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/prompt.rs` → `crates/libs/llm-runtime/src/prompt_tests.rs`

584 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `now_ms_returns_a_unix_timestamp` → 原宿主模块，原断言保留。

- `dummy_user_carries_session_and_model_references` → 原宿主模块，原断言保留。

- `dummy_user_uses_public_model_id_instead_of_adapter_api_id` → 原宿主模块，原断言保留。

- `request_options_enable_acp_requires_true_test_flag_or_non_blank_agent` → 原宿主模块，原断言保留。

- `request_options_enable_acp_rejects_non_object_options` → 原宿主模块，原断言保留。

- `acp_route_model_defaults_to_acp_with_restricted_capabilities` → 原宿主模块，原断言保留。

- `acp_route_model_trims_named_agent_for_all_model_identifiers` → 原宿主模块，原断言保留。

- `acp_route_model_uses_default_for_non_string_agent` → 原宿主模块，原断言保留。

- `explicit_model_bypasses_auto` → 原宿主模块，原断言保留。

- `cli_local_route_bypasses_auto_selector` → 原宿主模块，原断言保留。

- `resolve_model_for_options_uses_acp_route_model_without_explicit_model` → 原宿主模块，原断言保留。

- `resolve_model_for_options_treats_blank_model_as_absent_for_acp_test` → 原宿主模块，原断言保留。

- `resolve_model_for_options_ignores_client_model_for_cli_local` → 原宿主模块，原断言保留。

- `resolve_model_rejects_unknown_bare_model` → 原宿主模块，原断言保留。

- `resolve_model_trims_malformed_qualified_model_in_error` → 原宿主模块，原断言保留。

- `block_on_runs_futures_without_an_existing_runtime` → 原宿主模块，原断言保留。

- `assistant_error_to_string_formats_all_variants` → 原宿主模块，原断言保留。

- `missing_stream_error_message_maps_outer_errors_only` → 原宿主模块，原断言保留。

- `missing_stream_error_message_exposes_http_build_error` → 原宿主模块，原断言保留。

- `prompt_entrypoints_report_parse_errors_without_streaming` → 原宿主模块，原断言保留。

- `chat_entrypoint_reports_parse_errors_without_streaming` → 原宿主模块，原断言保留。

- `public_chat_entrypoint_reports_parse_error_without_streaming` → 原宿主模块，原断言保留。

- `send_prompt_returns_parse_errors_without_streaming` → 原宿主模块，原断言保留。

- `agent_model_inheritance_preserves_provider_and_auto_resolution` → 原宿主模块，原断言保留。

- `acp_model_source_routing_unsupported_cli_defaults_to_native_without_client_model` → 原宿主模块，原断言保留。

- `agent_model_inheritance_rejects_missing_client_model` → 原宿主模块，原断言保留。

- `agent_model_inheritance_cli_local_ignores_global_and_invalid_model` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/stream.rs` → `crates/libs/llm-runtime/tests/host_suites/stream_tests.rs`

727 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `should_abort_reflects_watch_receiver_state` → 原宿主模块，原断言保留。

- `should_abort_reads_the_latest_watch_value` → 原宿主模块，原断言保留。

- `is_retryable_assistant_error_uses_api_retry_flag_and_acp_session_change` → 原宿主模块，原断言保留。

- `is_retryable_assistant_error_rejects_non_matching_unknown_errors` → 原宿主模块，原断言保留。

- `is_acp_retryable_assistant_error_accepts_transient_acp_failures` → 原宿主模块，原断言保留。

- `is_acp_retryable_assistant_error_rejects_persistent_api_failures` → 原宿主模块，原断言保留。

- `is_acp_retryable_assistant_error_accepts_transient_api_error_messages` → 原宿主模块，原断言保留。

- `is_acp_session_changed_message_is_case_insensitive` → 原宿主模块，原断言保留。

- `resolve_stream_provider_info_allows_acp_route_provider_for_acp_requests` → 原宿主模块，原断言保留。

- `resolve_stream_provider_info_preserves_an_existing_provider` → 原宿主模块，原断言保留。

- `resolve_stream_provider_info_requires_an_explicit_acp_request_for_fallback` → 原宿主模块，原断言保留。

- `resolve_stream_provider_info_rejects_non_acp_missing_provider_for_acp_requests` → 原宿主模块，原断言保留。

- `stream_returns_provider_not_found_before_requesting` → 原宿主模块，原断言保留。

- `request_options_map_enable_acp_false_when_key_is_missing` → 原宿主模块，原断言保留。

- `request_options_map_enable_acp_true_when_acp_test_is_true` → 原宿主模块，原断言保留。

- `request_options_map_enable_acp_false_when_acp_test_is_false` → 原宿主模块，原断言保留。

- `request_options_map_enable_acp_true_when_acp_agent_is_non_empty` → 原宿主模块，原断言保留。

- `request_options_map_enable_acp_false_when_acp_agent_is_empty` → 原宿主模块，原断言保留。

- `request_options_map_enable_acp_false_when_acp_agent_is_whitespace` → 原宿主模块，原断言保留。

- `request_options_map_enable_acp_accepts_agent_when_test_flag_is_false` → 原宿主模块，原断言保留。

- `request_options_map_enable_acp_rejects_wrong_value_types` → 原宿主模块，原断言保留。

- `request_options_value_enable_acp_false_when_no_relevant_keys` → 原宿主模块，原断言保留。

- `request_options_value_enable_acp_true_when_acp_test_is_true` → 原宿主模块，原断言保留。

- `request_options_value_enable_acp_false_when_acp_test_is_false` → 原宿主模块，原断言保留。

- `request_options_value_enable_acp_true_when_acp_agent_is_non_empty` → 原宿主模块，原断言保留。

- `request_options_value_enable_acp_false_when_acp_agent_is_empty_string` → 原宿主模块，原断言保留。

- `request_options_value_enable_acp_false_when_acp_agent_is_whitespace` → 原宿主模块，原断言保留。

- `request_options_value_enable_acp_accepts_agent_when_test_flag_is_false` → 原宿主模块，原断言保留。

- `request_options_value_enable_acp_rejects_non_object_and_wrong_value_types` → 原宿主模块，原断言保留。

- `acp_route_provider_info_has_expected_structure` → 原宿主模块，原断言保留。

- `stream_cancellation_prevents_provider_setup` → 原宿主模块，原断言保留。

- `model_recovery_catalog_errors_keep_structured_classification` → 原宿主模块，原断言保留。

- `model_recovery_network_requests_do_not_multiply` → 原宿主模块，原断言保留。

- `codex_plugin_compat_t41_stream_retries_emit_per_request` → 原宿主模块，原断言保留。

- `acp_model_source_routing_preserves_provider_auth_and_rejects_missing_provider` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/stream.rs` → `crates/libs/llm-runtime/tests/host_suites/transport_tests.rs`

194 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `compaction_output_budget_reaches_openai_compatible_request` → 原宿主模块，原断言保留。

- `model_transport_interruption_retries_only_before_output` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/mod.rs` → `crates/libs/llm-runtime/tests/host_suites/tests.rs`

346 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `discard_replay_strategy_is_the_default_and_can_be_selected` → 原宿主模块，原断言保留。

- `discard_replay_prompt_omits_old_history` → 原宿主模块，原断言保留。

- `parse_acp_options_maps_cli_style_runtime_overrides` → 原宿主模块，原断言保留。

- `parse_acp_options_normalizes_codex_thought_level` → 原宿主模块，原断言保留。

- `parse_acp_options_rejects_invalid_permission_mode` → 原宿主模块，原断言保留。

- `acp_session_name_tracks_local_session_id` → 原宿主模块，原断言保留。

- `build_acp_command_line_preserves_configured_args` → 原宿主模块，原断言保留。

- `lookup_acp_command_uses_built_in_agent_when_config_is_empty` → 原宿主模块，原断言保留。

- `lookup_acp_command_resolves_built_in_agent_from_explicit_selection` → 原宿主模块，原断言保留。

- `missing_session_error_matches_cli_guidance` → 原宿主模块，原断言保留。

- `normalize_acp_agent_config_rewrites_legacy_claude_code_package` → 原宿主模块，原断言保留。

- `acp_message_extractors_preserve_text_reasoning_and_tool_calls` → 原宿主模块，原断言保留。

- `acp_session_changed_matcher_accepts_nested_acp_prefixes` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/aisdk/convert.rs` → `crates/libs/llm-runtime/src/aisdk/host_tests/convert_tests.rs`

89 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `native_tool_search_request_capability_requires_responses_and_explicit_metadata` → 原宿主模块，原断言保留。

- `native_tool_search_history_namespaced_schema_is_preserved_and_budgeted` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/aisdk/driver.rs` → `crates/libs/llm-runtime/src/aisdk/host_tests/driver_tests.rs`

38 行；完整迁入 host_migration_24_tests::driver，原挂载删除。

- `native_tool_search_dependency_locked_dependency_exposes_native_contract` → `host_migration_24_tests::driver::native_tool_search_dependency_locked_dependency_exposes_native_contract`

- `native_tool_search_dependency_rejects_unknown_execution_and_ordinary_function_results` → `host_migration_24_tests::driver::native_tool_search_dependency_rejects_unknown_execution_and_ordinary_function_results`

### `crates/vw-agent/src/session/llm/aisdk/request.rs` → `crates/libs/llm-runtime/src/aisdk/host_tests/request_tests.rs`

206 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `native_tool_search_request_captures_native_and_ordinary_transport` → 原宿主模块，原断言保留。

- `native_tool_search_request_invalid_schema_fails_before_transport_for_both_modes` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/aisdk/stream.rs` → `crates/libs/llm-runtime/src/aisdk/host_tests/stream_tests.rs`

161 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `native_tool_search_request_namespace_calls_resolve_stable_ids_and_reject_unknown` → 原宿主模块，原断言保留。

- `native_tool_search_event_stream_deduplicates_and_preserves_function_namespace` → 原宿主模块，原断言保留。

- `native_tool_search_event_stream_rejects_unadvertised_search` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/aisdk/mod.rs` → `crates/libs/llm-runtime/src/aisdk/host_tests/native_search_acceptance_tests.rs`

617 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `native_tool_search_acceptance_native_and_function_discovery_complete_same_task` → 原宿主模块，原断言保留。

- `native_tool_search_acceptance_desktop_completes_both_http_paths` → 原宿主模块，原断言保留。

- `native_tool_search_acceptance_same_batch_and_empty_search_cannot_grant_access` → 原宿主模块，原断言保留。

- `native_tool_search_acceptance_unadvertised_and_forged_calls_fail_closed` → 原宿主模块，原断言保留。

- `native_tool_search_acceptance_restored_history_and_revocation_do_not_load_old_catalog` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/aisdk/mod.rs` → `crates/libs/llm-runtime/src/aisdk/host_tests/model_catalog_tests.rs`

294 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `catalog_execution_gate_blocks_before_network` → 原宿主模块，原断言保留。

- `catalog_execution_gate_blocks_image_markers` → 原宿主模块，原断言保留。

- `catalog_execution_gate_blocks_unsupported_audio_parts` → 原宿主模块，原断言保留。

- `catalog_execution_gate_blocks_unsupported_tool_definitions` → 原宿主模块，原断言保留。

- `catalog_execution_gate_blocks_unsupported_tool_history` → 原宿主模块，原断言保留。

- `catalog_execution_gate_blocks_undeclared_effort_level` → 原宿主模块，原断言保留。

- `catalog_execution_gate_blocks_unsupported_reasoning` → 原宿主模块，原断言保留。

- `catalog_execution_gate_preserves_explicitly_supported_request` → 原宿主模块，原断言保留。

- `catalog_execution_gate_preserves_unknown_media_and_tool_policy` → 原宿主模块，原断言保留。

- `catalog_execution_gate_requires_evidence_for_explicit_effort` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/llm/recovery_test_support.rs` → `crates/libs/llm-runtime/src/recovery_test_support_tests.rs`

32 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `recovery_config_updates_credentials_and_restores_distinct_environment_snapshots` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/processor/utils/file_links.rs` → `crates/libs/session-engine/tests/host_suites/file_links_tests.rs`

129 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `resolve_full_path_uses_absolute_root_or_relative_path` → 原宿主模块，原断言保留。

- `resolve_full_path_trims_relative_input_before_joining_root` → 原宿主模块，原断言保留。

- `build_file_link_reports_relative_path_open_url_and_size` → 原宿主模块，原断言保留。

- `build_file_link_reports_zero_size_for_missing_relative_file` → 原宿主模块，原断言保留。

- `build_file_link_without_root_reports_open_url_and_size` → 原宿主模块，原断言保留。

- `build_file_link_keeps_absolute_display_path_outside_root` → 原宿主模块，原断言保留。

- `maybe_inject_file_link_respects_eligibility_and_existing_links` → 原宿主模块，原断言保留。

- `maybe_inject_file_link_supports_every_file_tool_alias` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/processor/utils/session_ingest.rs` → `crates/libs/session-engine/tests/host_suites/session_ingest_tests.rs`

354 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `ingest_user_query_ignores_whitespace_only_input` → 原宿主模块，原断言保留。

- `ingest_user_query_keeps_malformed_allowed_tool_call_as_text` → 原宿主模块，原断言保留。

- `ingest_user_query_consumes_multiline_tool_input_without_user_text` → 原宿主模块，原断言保留。

- `ingest_user_query_pushes_text_and_executes_inline_tool_calls` → 原宿主模块，原断言保留。

- `ingest_user_query_replaces_duplicate_last_user_content_with_trimmed_text` → 原宿主模块，原断言保留。

- `ingest_user_query_ignores_disallowed_tool_lines_as_text` → 原宿主模块，原断言保留。

- `ingest_user_query_deduplicates_text_before_tool_but_keeps_same_text_after_tool` → 原宿主模块，原断言保留。

- `ingest_assistant_answer_returns_trimmed_text_when_no_tool_is_present` → 原宿主模块，原断言保留。

- `ingest_assistant_answer_executes_tool_call_and_keeps_surrounding_text` → 原宿主模块，原断言保留。

- `ingest_assistant_answer_keeps_rejected_tool_lines_as_text` → 原宿主模块，原断言保留。

- `ingest_assistant_answer_records_empty_input_unknown_tool_failure` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/prompt.rs` → `crates/libs/session-engine/tests/host_suites/prompt_tests.rs`

75 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `environment_wraps_missing_model_with_explicit_root` → 原宿主模块，原断言保留。

- `system_includes_environment_for_explicit_root` → 原宿主模块，原断言保留。

- `system_prompt_keeps_project_agents_independent_from_empty_identity_whitelist` → 原宿主模块，原断言保留。

- `system_prompt_initial_request_reads_explicit_workspace_agents_family` → 原宿主模块，原断言保留。

- `agents_context_base_system_excludes_instruction_content` → 原宿主模块，原断言保留。

### `crates/vw-agent/src/session/revert.rs` → `crates/libs/session-engine/tests/host_suites/revert_tests.rs`

37 行；保留真实宿主装配注册；直接使用 crate::app / config / tool context / provider 全局入口，未复制到领域夹具。

- `cleanup_without_revert_is_a_noop` → 原宿主模块，原断言保留。

- `unrevert_rejects_missing_project_context_before_snapshot_access` → 原宿主模块，原断言保留。

## 未来验证

以下均未运行；预期数为源码静态计数，不是测试通过数。

```sh

python3 scripts/cargo_slot.py cargo test -p llm-runtime --lib host_migration_24_tests -- --test-threads=1

python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_24_tests -- --test-threads=1

python3 scripts/cargo_slot.py cargo test -p vw-agent --lib fork_execution_uses_current_host_tenant_scope -- --test-threads=1

```

macOS/Unix：llm-runtime 8，session-engine 1，宿主薄用例 1；非 Unix 少 profile 1。真实 CLI ignored 用例需要固定 CLI 和 fixture 后按原测试名显式 --ignored。其余保留测试按上表原名过滤，要求有效计数 > 0。未来用真实宿主 --list 确认上述迁出名称消失，不能把零匹配当通过。

静态核对：检查旧 mod/path 挂载移除、目标 lib 自注册、类型入口、调用及依赖方向；git diff --check 只核对本任务差异。不宣称运行收益或编译耗时下降。


## Deep recovery: supersedes the original acceptance retention decision

Read final.md and remaining-host.json (owner 24: 2678 lines). Both real CLI harnesses now belong to llm-runtime::acp::acceptance_support. Original RuntimeHost executes the actual request, controls and selection store. AcceptanceHost adds only two real configuration write ports, implemented with host config APIs. Manifest validation precedes fixture environment writes and project creation; read the manifest once. No fake permission or runtime implementation.

### `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`: 639 -> 47 lines

- `accepted` -> llm-runtime/src/acp/acceptance_support/model_source.rs; original assertions and ignore conditions retained.
- `acp_model_source_gemini_harness_requires_exact_model_and_generation_path` -> llm-runtime::host_migration_24_tests::acceptance_model_source::acp_model_source_gemini_harness_requires_exact_model_and_generation_path; original assertions and ignore conditions retained.
- `acp_model_source_harness_contract_no_model_request_is_failure` -> llm-runtime::host_migration_24_tests::acceptance_model_source::acp_model_source_harness_contract_no_model_request_is_failure; original assertions and ignore conditions retained.
- `acp_model_source_harness_contract_handshake_is_not_model_success` -> llm-runtime::host_migration_24_tests::acceptance_model_source::acp_model_source_harness_contract_handshake_is_not_model_success; original assertions and ignore conditions retained.
- `acp_model_source_harness_contract_forged_model_is_failure` -> llm-runtime::host_migration_24_tests::acceptance_model_source::acp_model_source_harness_contract_forged_model_is_failure; original assertions and ignore conditions retained.
- `acp_model_source_harness_contract_rejects_unauthenticated_or_wrong_model` -> llm-runtime::host_migration_24_tests::acceptance_model_source::acp_model_source_harness_contract_rejects_unauthenticated_or_wrong_model; original assertions and ignore conditions retained.
- `evidence` -> llm-runtime/src/acp/acceptance_support/model_source.rs; original assertions and ignore conditions retained.
- `request` -> llm-runtime/src/acp/acceptance_support/model_source.rs; original assertions and ignore conditions retained.
- `request_in_session` -> llm-runtime/src/acp/acceptance_support/model_source.rs; original assertions and ignore conditions retained.
- `checkpoint` -> llm-runtime/src/acp/acceptance_support/model_source.rs; original assertions and ignore conditions retained.
- `run` -> llm-runtime/src/acp/acceptance_support/model_source.rs; original assertions and ignore conditions retained.
- `effort_run` -> llm-runtime/src/acp/acceptance_support/model_source.rs; original assertions and ignore conditions retained.
- `acp_model_source_opencode_effort_real_cli` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_model_source_claude_effort_real_cli` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_model_source_copilot_effort_real_cli` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_model_source_real_cli_session_model` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_model_source_real_cli_cli_local` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `observe_rpc` -> llm-runtime/src/acp/acceptance_support/model_source.rs; original assertions and ignore conditions retained.
### `crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs`: 471 -> 164 lines

- `accepted` -> llm-runtime/src/acp/acceptance_support/skill_mounts.rs; original assertions and ignore conditions retained.
- `acp_skill_mount_harness_zero_requests_never_pass` -> llm-runtime::host_migration_24_tests::acceptance_skill_mounts::acp_skill_mount_harness_zero_requests_never_pass; original assertions and ignore conditions retained.
- `acp_skill_mount_harness_old_model_report_never_passes` -> llm-runtime::host_migration_24_tests::acceptance_skill_mounts::acp_skill_mount_harness_old_model_report_never_passes; original assertions and ignore conditions retained.
- `acp_skill_mount_harness_requires_discovery_clean_prompt_and_real_results` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_harness_initial_prompt_contains_no_body` -> host initial prompt wiring assertion; original assertions and ignore conditions retained.
- `fixture` -> llm-runtime/src/acp/acceptance_support/skill_mounts.rs; original assertions and ignore conditions retained.
- `request` -> llm-runtime/src/acp/acceptance_support/skill_mounts.rs; original assertions and ignore conditions retained.
- `checkpoint` -> llm-runtime/src/acp/acceptance_support/skill_mounts.rs; original assertions and ignore conditions retained.
- `defaults` -> llm-runtime/src/acp/acceptance_support/skill_mounts.rs; original assertions and ignore conditions retained.
- `run` -> llm-runtime/src/acp/acceptance_support/skill_mounts.rs; original assertions and ignore conditions retained.
- `acp_skill_mount_real_codex` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_claude` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_qwen` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_gemini` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_codebuddy` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_goose` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_vibe` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_hermes` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_snow` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_copilot` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_kilocode` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_kimi` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_kiro` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_opencode` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_openclaw` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_pi` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_auggie` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_cursor` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_droid` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_qoder` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_trae` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.
- `acp_skill_mount_real_iflow` -> original host thin entry -> RuntimeHost -> domain run; original assertions and ignore conditions retained.

Original 52 paths now 1779 lines, reduction 1181 from baseline 2960. Host run functions retain only global lock, ensure_host, fixture key, real project context. Eight evidence-validator tests moved whole to independent domain test files registered by host_migration_24_tests. Future llm-runtime filter expects 16 tests on Unix, 15 otherwise; session-engine still 1. Commands above remain future-only.

Regex dependency preserves the existing diagnostic redaction exactly. Cargo.lock adds only llm-runtime -> regex. No reverse host dependency, host path/include, removed assertion, or real CLI replacement. External mounted suite mappings above remain applicable. Production host algorithms are unchanged: these are domain acceptance flows called by real host test adapters, not production migration gains.

Dynamic tool Gateway acceptance (495 lines), actual host ToolContext and prompt registration remain as documented above. Net reduction is still below 70%; whole-plan gate remains open. No Cargo, compilation, build, test or device run was performed. Static diff checks are not behavior evidence.


## Recovery 2: real host restoration and prompt contracts

Read the updated final.md (host 51915, remaining gate 16915) and owner=24 entries in remaining-host.json. No Cargo, build or tests run.

- `crates/vw-agent/src/session/dynamic_tool_store_tests.rs`: 128 -> 14 lines.
- `crates/vw-agent/src/session/llm/messages_system_tests.rs`: 85 -> 59 lines.

Dynamic restoration: the entire assertion sequence now calls session-engine::dynamic_tool_restore_contract::dynamic_tools_restore_client_specs_do_not_restore_host_capabilities. It creates actual ClientToolHost owners, persists via the actual SessionStore and verifies rebind denial, fresh host binding and deferred discovery. The real SQLite ProjectStore/SessionStore fixture moved with it. ChatSession has identical field values, deserialized to the concrete parameter type of save_session; no persistence mock. The host supplies only its real ClientTool specialization, database location and original named test entry. Unused VersionTool/tools helpers had no callers and were removed. fixture/fixture_project moved; name/description/parameters_schema/execute belonged solely to the unused VersionTool.

- `build_system_messages_merges_non_empty_sources_in_order` retains its original host entry, fixture and renderer, then calls `llm-runtime::system_message_contract::merged_sources` with actual output; all original assertions retained.
- `build_system_messages_does_not_duplicate_unified_prompt_for_codex` retains its original host entry, fixture and renderer, then calls `llm-runtime::system_message_contract::unified_once` with actual output; all original assertions retained.
- `build_system_messages_keeps_pre_rendered_runtime_prompt_once` retains its original host entry, fixture and renderer, then calls `llm-runtime::system_message_contract::rendered_runtime_once` with actual output; all original assertions retained.
- `build_system_messages_filters_blank_optional_sources_and_keeps_provider_prompt` retains its original host entry, fixture and renderer, then calls `llm-runtime::system_message_contract::provider_only` with actual output; all original assertions retained.

This round removes 140 host lines. It is assertion/fixture ownership migration, zero new production algorithm migration. Domain support functions are called by the existing real host test wrappers, not independently registered tests; do not claim these five names disappear from the host list or inflate domain test counts. No added Cargo dependency. New modules are registered by their domain lib.rs; no reverse host dependency or path/include.

Future validation: `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib dynamic_tools_restore_client_specs_do_not_restore_host_capabilities -- --test-threads=1` expects 1; `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib messages_system_tests -- --test-threads=1` expects 4. Neither command was run. Existing domain migration filters and counts in the preceding recovery remain unchanged. Static diff whitespace check only.

The 495-line real Gateway integration remains: its five tests exercise actual authenticated routes, SDK transport, live callbacks, session creation/forking and processor stream scripting. A separate gateway-wide fixture port would span task10/11/23 ownership; no facade that mocks those routes was introduced. All other source and external mappings above remain current. The whole-plan line gate is still open.
