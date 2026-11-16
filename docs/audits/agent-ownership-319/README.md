# 319 全源码归属静态审计

本记录消费 318 提交 fb8295d1c，覆盖 coverage.md 的 2991 条历史路径，以及当前主包 1378 个源码、测试和资源文件（排除 Finder 的 .DS_Store）。agent-composition 是主包职责，不新增包。

## 可复核清单

- retained.tsv：每个保留文件的原任务归属、组合角色、实际引用领域库、文本入边、宿主函数和绑定子系统、内容 SHA256。声明/转发文件保持兼容路径；含函数文件绑定主包实际配置、租户、项目、工具、服务或平台动作。测试入口需要组合环境，不持有生产状态。
- coverage.tsv：逐条原路径与当前保留情况、原职责包内同名目标候选。候选列用于定位；同名不证明语义等价，多候选不能解释为重复实现。领域继续细分的真实所有者以 retained.tsv 的直接库引用和领域包重导出为准。
- includes.tsv：静态字符串 include!/include_str!/include_bytes! 和 CARGO_MANIFEST_DIR concat 引用及目标存在性，覆盖库及主包。不是宏展开结果；非 agent 领域条目不纳入本卡修复。
- manifests.json：621 个包及 621 个 workspace 成员，本地生产/构建依赖路径检查零缺失、合并平台 cfg 后依赖图零环。dev-dependencies 不参与生产依赖环判断。
- features.json：53 条 feature 转发边静态检查，无缺失依赖或本地目标 feature。外部依赖 feature 未通过 Cargo 解析验证。

## 本卡修复与唯一所有者

1. agent/loop_::utils_tests、gateway/api/handlers::computer_control_tests、session::task_session_tests、task-scheduler::task_graph_tests 的旧声明指向不存在文件。删除失效声明，保留现有真实注册：agent-loop/src/lib.rs、gateway-computer-control/src/handlers.rs、session-engine/src/task_session.rs、task-scheduler-core/src/lib.rs。未删改测试断言。
2. session/llm/acp 的 recovery 旧声明没有文件且没有旧路径调用；实现与注册已经在 llm-runtime/src/acp/recovery.rs 和 mod.rs，移除失效声明。
3. session/child_agent/coordinator.rs 的 mailbox/delivery 是宿主权限适配，实际文件在同级，补显式 path。状态只在 session-engine::child_agent::coordinator；消息契约在 session-port。授权仍使用原 context 和 target，不改变租户或取消参数。
4. model_relay/stream_observer_tests.rs 的共享样本改指 app-diagnostics-runtime/src/diagnostic_stream_fixtures.rs，保持原样本与断言。
5. gateway/aggregation_lifecycle_tests.rs 原样迁至 gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs，并修正 aggregation_publish_tests.rs 的 path。此测试仍通过主包既有 include 链在宿主环境编译，不独立启动服务。
6. gateway-workflow/src/host_tests.rs 的 talking_head.yaml 原先指向不存在的仓库根 testdata；改指 workflow-engine/testdata/workflow/talking_head.yaml，消费真实领域资源。
7. provider/error.rs 是未注册旧副本，主包 provider::error 实际 inline 重导出 provider-errors，agent-provider::error 也只重导出它。旧副本与真实所有者差异已对照：真实所有者额外包含阿里云市场激活提示；JSON object 判断仅 filter/and_then 写法不同；错误类型、状态码、重试及序列化契约相同。移除未注册旧副本，保持当前可达行为。
8. 原 provider/error_tests.rs 也未注册；其中流式样本没有 type:error，却断言 Some，与旧实现和当前实现均不相符；另一断言使用真实匹配表未包含的文本。原样保存在 provider-errors/tests/fixtures/legacy_error_tests.rs，不盲目注册、弱化或声称通过。未来验收应先明确历史样本预期。真实 provider-errors/src/error_tests.rs 继续注册，不受影响。

## 依赖与保留边界

vw-agent 主包只组合已有领域实现和宿主绑定，未新增万能 core 或复制状态。SessionSettings、ChildAgentCoordinator 等主包同名包装通过 Deref/委托使用领域实例；ToolUseContext、ToolRuntimeContext 等为绑定具体宿主的泛型别名。Host/Source 等无状态实现不是新的领域状态所有者。

清单搜索中的 vw-agent-apps 是已有独立契约包，不是 vw-agent 反向依赖。六个 cli-* 包（cli-agent-session、cli-task-commands、cli-bootstrap、cli-command-schema、cli-runtime-status、cli-security-commands）是上层 CLI 入口，依赖应用组合包；vw-agent 不反向依赖它们，未形成领域环。其它领域包无直接 vw-agent 依赖。根清单无需追加成员；本卡不改变依赖或 feature/cfg。

纯文本模块和 include 核验在候选修复后，主包显式 mod/path 零缺失，agent 相关库静态 include 目标零缺失。现存外部 Working 改动（含桌面文件）不属于本卡，保持原样，不纳入提交。

## 证据限制与后续验收

本轮仅静态证据，不证明 Rust 名称解析、宏展开、类型兼容或行为通过。未运行 Cargo、单元测试、实机测试、产品启动或外部服务探测，也未更新 Cargo.lock。需以后明确授权验收：上述四个领域测试入口、子代理 mailbox/delivery/continuation、stream_observer、聚合 publish/lifecycle、gateway-workflow host_tests，以及主包直接调用集成；覆盖 native/wasm、macOS/Windows/Linux 和主包全部现存 feature 组合。UI 渲染及真实外部能力仍未验证。
