# 任务45：工作流兼容入口与残留装配

代码完成，编译/行为未验证。遵照公共计划未运行 Cargo、fmt、构建、单元测试、实机或测试可执行文件。仅做源码、调用引用、模块注册和差异检查。

## 文件映射与行数

| 来源 | 前→后（splitlines） | 处理/保留职责 |
|---|---:|---|
| `native.rs` | 112→112 | 保留 NativeCommandAdapter、event、now_ms、default_runtime、run_bound_command、run_video_command、run_command：真实 StreamEvent/Request 转换、当前租户、会话 runtime、工具及视频准备装配。领域执行原已位于 native_command，不算本轮生产迁移。 |
| `templates_tests.rs` | 332→0 | 整组迁到 gateway-workflow/src/host_migration_45_tests.rs；直接调用 server::applications_router 和 workflow_core::{store,templates}，保留真实SQLite/Postgres及并发合同。API仍使用同一 agent_paths 全局数据库及随机租户清理。 |
| `variables.rs` | 2→0 | 删除无调用私有重导出；无业务实现，不计生产迁移收益。 |
| `code_runner.rs` | 3→0 | 删除无调用私有重导出；无业务实现，不计生产迁移收益。 |
| `store.rs` | 2→2 | 保留实际调用方所需 workflow_core::store 公开导出。 |
| `runner.rs` | 3→3 | 保留实际公共 WorkflowRuntime/运行函数兼容导出，领域实现既有不计收益。 |
| `mod.rs` | 37→30 | 删除五个无调用私有兼容模块和 templates_tests 旧挂载；其余为公共工作流API、真实native/video入口。 |
| `native_tests.rs` | 506→329 | 保留真实宿主请求、租户夹具、命令接线1条、工具/agent能力拒绝2条、Git真实执行/上游失败3条；CloudFixtureProvider仅供真实Git测试构建runtime。 |
| `template.rs` | 2→0 | 删除无调用私有重导出；无业务实现，不计生产迁移收益。 |
| `templates.rs` | 2→2 | 保留 video entry/e2e 等实际调用方所需模板公开导出。 |
| `native_agent.rs` | 113→113 | 保留 NativeTools allowed_tools/execute/call、NativeAgent prepare_stage/with_stage/emit_tool/visible_answer/run：真实工具权限执行、Request/ChatRole 转换、租户作用域与会话启动。 |
| `conditions.rs` | 2→0 | 删除无调用私有重导出；无业务实现，不计生产迁移收益。 |
| `model.rs` | 2→0 | 删除无调用私有重导出；无业务实现，不计生产迁移收益。 |

宿主合计 1118→591，净减 527（47.1%），不足70%。保留原因见逐文件说明；继续可迁：三个Git测试中的图形调度断言可进一步拆分，但真实Git、审计配置与权限执行必须仍由具体宿主验证；两个provider限制测试含真实 NativeAgent 消息转换与 NativeTools 权限边界，不能以模拟端口替代。收口任务须验收此减量缺口。

生产/测试分项：原宿主生产文件280行、测试文件838行；修改后按下表实际路径计数（非测试文件内的cfg测试挂载计装配行，不当生产算法）。领域新增均为测试及测试资源，生产算法迁移0。

- 宿主测试：838→329行。
- 宿主生产/装配：280→262行。
- workflow-engine 新测试0→305行；gateway-workflow 新测试0→331行；两库生产 lib.rs 各新增3行测试注册。

## 全用例映射

- `native_workflow_bound_command_executes_graph_and_returns_answer` → 宿主 native::tests；workflow-engine::host_migration_45_tests
- `native_workflow_unmatched_command_preserves_normal_session` → workflow-engine::host_migration_45_tests
- `native_workflow_skill_palette_selection_dispatches_same_graph` → workflow-engine::host_migration_45_tests
- `native_workflow_rejects_cross_tenant_binding` → workflow-engine::host_migration_45_tests
- `native_workflow_rejects_file_outside_project` → workflow-engine::host_migration_45_tests
- `native_workflow_failed_node_never_reports_success` → workflow-engine::host_migration_45_tests
- `native_workflow_preserves_cancellation_and_tool_restrictions` → workflow-engine::host_migration_45_tests
- `native_workflow_provider_names_preserve_tool_restrictions` → 宿主 native::tests
- `native_workflow_provider_names_preserve_agent_restrictions` → 宿主 native::tests
- `native_workflow_cloud_graph_gates_execution_on_valid_plan_and_preview` → workflow-engine::host_migration_45_tests
- `task_pool_actions_absent_steps_have_no_side_effects` → 宿主 native::tests
- `task_pool_actions_explicit_git_steps_use_native_tool_permissions` → 宿主 native::tests
- `task_pool_actions_upstream_failure_skips_downstream_send` → 宿主 native::tests
- `video_edit_template_sqlite_contract` → gateway-workflow::host_migration_45_tests
- `video_edit_template_postgres_contract` → gateway-workflow::host_migration_45_tests
- `video_edit_template_seed_rejects_invalid_graph_or_options` → gateway-workflow::host_migration_45_tests
- `video_edit_template_applications_api_shares_snapshot_and_checks_versions` → gateway-workflow::host_migration_45_tests

夹具映射：request/binding_fixture/run_events → workflow-engine 的局部输入和固定租户夹具；宿主 request/binding_fixture 仅服务真实接线。CloudFixtureAgent 与云图用例完整迁入 engine，CloudFixtureProvider 两侧各保留实际使用的外部模型端口；action_graph/action_repository 保留真实Git/工具宿主夹具。模板 yaml/seed/body/contract/assert_reopened_edit/request 随整组迁往 gateway-workflow。云图资源复制到 engine/testdata，原资源仍有其他调用者不删除。

## 接线、依赖与未来验证

宿主 native::run_command → workflow_engine::native_command::run_command；原生工具与agent实现既有Host端口，未改变依赖方向。模板路由 → workflow_core 的真实存储。新测试均由各库 lib.rs cfg(test/native) 自注册，无 path/include 宿主源码，无 vw-agent 反向依赖。gateway-workflow 增加已有依赖的测试特性和模板资源/摘要/测试工具dev依赖。删除五个私有重导出前全仓引用检索未发现调用者。任务1外部挂载登记未发现本组额外path测试。

`python3 scripts/cargo_slot.py cargo test -p workflow-engine --lib host_migration_45_tests -- --test-threads=1`：静态预期 8 个非零用例（Postgres依赖测试环境，宏可能跳过）；未运行。
`python3 scripts/cargo_slot.py cargo test -p gateway-workflow --lib host_migration_45_tests -- --test-threads=1`：静态预期 4 个非零用例（Postgres依赖测试环境，宏可能跳过）；未运行。

宿主未来对上述6个保留用例逐条执行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib workflow::native::tests::<完整用例名> -- --exact --test-threads=1`，每条预期1个；先用对应 `-- --list` 获取真实名称。迁出用例需宿主 list 确认旧挂载消失（首条真实接线用例有意保留），不能用零测试冒充通过。编译收益、运行数量与断言结果均未取得。

## 任务59恢复轮补充

按 recovery-priorities.json owner45 的 graph/step/no-side-effect 拆分执行。native_tests.rs 329→117，额外净减212行；本组累计1118→379（66.1%），仍不足70%，剩余native生产装配262行及117行真实宿主测试，不能宣称达到70%。

- action_graph、action_repository及三条 task_pool_actions_* 的全部断言 → workflow-engine/src/test_support/actions.rs。原宿主同名3条测试仍注册，调用领域断言驱动，注入宿主 action_runtime。真实 NativeTools、full_access、approval上下文和外置审计临时目录仍由宿主创建；目录句柄返回驱动并活至整次run_workflow结束。Git仓库/提交/合并仍真实执行，不模拟权限。
- 两条 native_workflow_provider_names_* 的矩阵和全部断言 → test_support/permissions.rs；宿主仍构造实际 NativeTools/NativeAgent。驱动直接调用真实 trait 对象，未重写许可检查、未伪造缺少消息错误。
- binding_fixture → test_support/binding.rs，使用宿主注入的真实租户。原真实 bound_command 测试及断言仍在宿主。
- CloudFixtureProvider → test_support/provider.rs。engine 的原8条自注册领域用例复用同一provider和binding源码（均为本库源码），消除重复夹具；生产Provider不会启用该测试模块。
- test-support 为实际宿主dev依赖启用的显式特性，按职责拆分4个小文件；没有新增Host框架/反向依赖/新单体。git2沿用宿主0.21.0及既有features，仅测试特性启用；anyhow亦为可选依赖。Cargo.lock仅给workflow-engine补既有git2依赖边。

静态核对：宿主同名6条测试注册保持，engine8条领域注册保持，gateway4条保持；断言驱动不是新增独立测试，不能增加通过数。真实生产算法迁移仍为0。未来仍按前文6条宿主精确名称过滤（每条预期1），领域两个过滤预期8/4；test-support由宿主dev依赖统一启用。所有Cargo、构建、测试、实机与fmt命令均未运行，编译和行为未验证。当前差异检查仅覆盖本任务。原任务文件已在首轮删除，无文件可再次清理。
