# 任务 47: 插件兼容 plugin-mcp-1

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 1（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：无

历史审计编号：原任务 163，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 无本计划内已确认前置；核对已有基线与本卡实际导出后即可进入就绪队列，不等待其它领域。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 24 的 `crate::tools::mcp_common::RunningMcpPeer`；任务 28 的 `crate::tools::url_validation::is_private_or_local_host`；任务 56 的 `crate::tools::mcp_tool::mcp_tool_id`；任务 153 的 `crate::agent_apps::mcp::acp_mount::AcpMountLeases`；任务 158 的 `crate::plugins::mcp::load_plugin_mcp_declarations`；任务 184 的 `crate::agent::loop_::approval::NonCliApprovalPrompt`；任务 190 的 `crate::tools::ToolCallResult`；任务 197 的 `crate::agent_apps::mcp::revocation::runtime_selection_is_current`；任务 269 的 `crate::session::ui_store::current_tenant_scope`；任务 311 的 `crate::agent::loop_::NonCliApprovalContext`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
- 执行时滚动调度，编号仅提供可行线性顺序；只等待本卡明确列出的前置完成并集成。在途旧编号按任务编号对照认领，禁止重复执行。

状态：部分迁移，剩余职责有具体接口前置；未集成、未提交

目的：迁移，将本任务职责归入 `plugin-mcp`。

来源任务：用户要求尽可能全部拆分vw-agent；覆盖映射见coverage.md。

预计会话范围：一个领域的一组源码迁移；大领域已拆成连续任务，同包逐项接线。测试源码和资源仅随实现迁移，不执行。

## 并行实施边界

- 按 plan.md 的并行协议执行：独立工作树负责领域实现，主协调会话负责公共接线与逐项合入；禁止在同一检出目录同时执行并行任务。
- 本任务只修改下方来源文件、对应目标包及本编号任务记录。根清单、vw-agent 清单/lib.rs、清单外父模块和直接调用者由协调会话独占修改。
- 开始前检查当前基线的 use/super/宏及具体符号所有者。候选引用不是完整依赖图；发现需要另一在途任务的新接口时，报告具体符号，协调者将依赖方顺延，不猜签名或新造端口。
- 将公共接线逐文件列入本任务的“接线交接”记录，给出原/新导入、feature/cfg、注入位置和需删除的旧实现；不得仅交付泛化建议。
- 领域差异静态审阅后用 $git-commit 提交本工作树变更，保留本卡，报告提交与接线清单。此时仅为待合入产物，不标记已完成。协调会话接线、合入并复核后提交删除本卡，再归档已合入工作树。

## 文件

- 目标：`crates/libs/plugin-mcp/Cargo.toml`、`crates/libs/plugin-mcp/src/lib.rs` 及保持原职责布局的子模块；包已存在则修改。
- 公共接线（仅协调会话修改，本任务提供精确交接）：根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`、`crates/vw-agent/src/lib.rs`、相关原mod.rs与搜索确认的直接调用者清单。
- 来源及相邻测试/资源：
- `crates/vw-agent/src/plugins/compat/mcp_app_execution.rs`
- `crates/vw-agent/src/plugins/compat/mcp_apps.rs`
- `crates/vw-agent/src/plugins/compat/mcp_auth.rs`
- `crates/vw-agent/src/plugins/compat/mcp_capabilities.rs`
- `crates/vw-agent/src/plugins/compat/mcp_catalog.rs`
- `crates/vw-agent/src/plugins/compat/mcp_catalog_refresh.rs`
- `crates/vw-agent/src/plugins/compat/mcp_connection_sessions.rs`
- `crates/vw-agent/src/plugins/compat/mcp_connection_settings.rs`
- `crates/vw-agent/src/plugins/compat/mcp_connections.rs`
- `crates/vw-agent/src/plugins/compat/mcp_content.rs`
- `crates/vw-agent/src/plugins/compat/mcp_elicitation.rs`

## 上下文

- 重点符号：`register`、`revoke`、`call`、`association`、`resource`、`validate_resource`、`HttpContext`、`prepare`、`bind`、`source_config`、`discover_oauth`、`runtime_headers`、`runtime_oauth_headers`、`hook_source_matches`、`client_info`、`validate_server_protocol`、`list_tools`、`collect`。
- 当前文本引用候选：`tools::mcp_common`、`session::ui_store`、`tools`、`agent::loop_`、`tools::mcp_tool`、`tools::discover_dynamic_mcp_tools`、`agent_apps::mcp`、`tools::execute_bound_app_tool`、`plugins`、`plugins::mcp`、`tools::current_tool_use_context`、`tools::url_validation`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
- 领域边界：复用 plugin-types；compat/MCP/hook 细分子包任务见文件清单；安装路径审计与信任确认不可省略。 agent-plugins入口延后组合；安装、MCP与hook不得相互反向依赖，跨域调用用前置tool-context或已有业务端口。

## 步骤

1. 读取上述源码、父模块声明、相邻测试和调用者；用 `rg -n 'crate::|super::|pub\(crate\)|include!' crates/vw-agent/src` 限定到本任务查看实际依赖，记录旧路径到目标模块映射。测试文件单独排程时按其生产实现实际所有者迁入，不为测试再建包。
2. 按领域边界迁移本任务源码；剥离应用宿主动作到已存在的窄接口，更新资源路径与必要可见性。禁止新增未调用抽象，不合并行为不同的旧实现。
3. 建立/扩展目标最小清单，复用已拆出的真实所有者；完整转发原feature/cfg。将调用者正向接线和旧位置薄重导出列入交接，由协调会话实施。后续任务尚未迁的调用者通过真实宿主适配接线，不让新包依赖它。
4. 逐项审阅参数/错误/异步取消、租户、权限、全局状态、序列化与平台语义等价；仅调整测试源码导入，不运行或追加测试。将尚需闭环迁移的宿主适配具体路径写入本任务提交正文。
5. 按并行实施边界提交领域变更与接线交接，保留本卡供协调会话合入后收尾；不得提前宣称任务完成。

## 静态验收

- 运行：`git diff --check`；逐文件阅读本任务完整差异和迁移后的清单。
- 运行：`rg -n 'vw-agent|vw_agent|crate::app' crates/libs/plugin-mcp`；审阅是否存在库对主包的反向依赖，文档与测试引用单独检查。
- 检查：本任务全部来源已迁/明确为组合转发，生产实现不重复；类型/状态只有一个所有者；工厂注册和资源完整；后续文件已分配所有者。
- 所需证据：真实迁移映射、最小依赖、窄端口调用者、cfg/feature、直接调用接线、测试/资源迁移清单及差异检查结果。仅静态证据，不能声称编译或行为通过。
- 待统一验收：本任务来源的既有测试、直接调用集成、原支持平台与特性、涉及UI的渲染和外部能力；将具体路径及组合写提交正文，本轮全部禁止运行。

## 2026-11-15 实施证据与接线交接

遵守主会话最新指令：当前 checkout 实施，不创建 worktree，不修改公共文件，不暂存/提交；禁止 Cargo/测试。保留本卡。

### 已迁移

- `compat/mcp_capabilities.rs` → `plugin-mcp/src/mcp_capabilities.rs`，旧文件薄重导出；版本白名单与默认能力原样保留。
- `compat/mcp_catalog.rs` → `plugin-mcp/src/mcp_catalog.rs`，旧文件薄重导出；30 秒/100 页/2048 项/65536 字节限制原样保留。
- `compat/mcp_content.rs` → `plugin-mcp/src/mcp_content.rs`，旧文件薄重导出；ToolCallResult/ToolCallTelemetry 使用现有 tool-types 唯一所有者。
- `compat/mcp_elicitation.rs` → `plugin-mcp/src/mcp_elicitation.rs`，旧文件薄重导出；FormOwner 唯一所有者迁入新包。原 ui_store::with_tenant_scope + question::ask_sensitive 改为现有 agent-question::ask_sensitive(input, form, owner.tenant)，保留取消/关闭/120 秒超时、用户同意与拒绝语义；URL 私网判断复用 network-policy。
- `compat/mcp_apps.rs` 的 BINDING 与 association → `plugin-mcp/src/mcp_apps.rs`；原文件仅重导出这两项，resource/validate_resource 暂留，等待 mcp_resources 实际所有者交付。
- 无资源、include 路径或 feature/cfg 变化；新包仅依赖实际调用的协议、结果与用户提问所有者，不反向依赖 vw-agent。

### 公共接线（主会话独占）

1. 根 `Cargo.toml` workspace.members 添加 `crates/libs/plugin-mcp`。
2. `crates/vw-agent/Cargo.toml` dependencies 添加 `plugin-mcp = { path = "../libs/plugin-mcp", version = "0.6.0" }`，不设 optional/cfg；这五个原模块为无条件声明。
3. `crates/vw-agent/src/plugins/compat/mod.rs` 保留现有五个 mod 声明；`plugins/mod.rs` 保留现有 mcp_capabilities、mcp_content、mcp_catalog、mcp_elicitation、mcp_apps 重导出。来源文件已提供薄重导出，不再保留重复生产实现。
4. 直接调用者经原重导出保持签名：`tools/mcp_common/mod.rs`（FormOwner/request）、`tools/mcp_tool.rs`（association/BINDING/resource）、`compat/mcp_notifications.rs`（list_tools）、`compat/mcp_resources.rs`（validate_content/collect）、`compat/mcp_oauth_flow.rs`、`mcp_oauth_tokens.rs`、`mcp_connections.rs`、`mcp_connection_sessions.rs`（FormOwner）。如正向导入，分别替换为 `plugin_mcp::mcp_elicitation`、`mcp_catalog`、`mcp_content`、`mcp_apps`；resource 仍走宿主，不能切换到尚不存在的新包函数。

### 尚未迁移的具体前置（禁止猜测接口）

- `mcp_connections.rs:19,27,59-60,252,367`：依赖宿主 `RunningMcpPeer`、`open_server_peer`、`current_tool_use_context`、`AcpMountLeases`、runtime_selection_is_current，以及 `mcp_notifications::{McpCatalogs,ToolCatalogBinding}`、`mcp_oauth_tokens` 的真实实现。当前 mcp-client 仅导出元数据/配置工具，没有 RunningMcpPeer/open_server_peer；不能用同名猜测端口连接。
- `mcp_catalog_refresh.rs`：对 McpSessionConnections 的固有 impl，直接访问 state/tool_sources，依赖上述连接所有者与 `tools::{catalog::ToolCatalog,discover_dynamic_mcp_tools}`，不可独立搬迁。
- `mcp_connection_sessions.rs`：持有 McpSessionConnections 的全局唯一连接状态并调用宿主 app execution 撤销；随连接所有者迁移，不能复制全局状态。
- `mcp_auth.rs:2,134,159,187,208` 与 `mcp_connection_settings.rs:2`：消费尚在宿主的 `mcp_legacy::{PluginMcpDeclaration,PluginMcpParseOutcome,PluginMcpServerError}`、`plugins::mcp::load_plugin_mcp_declarations`、`mcp_public_auth::policy`、`mcp_oauth_discovery::{OAuthDiscovery,discover}`。需其真实导出后迁移，不引入 plugin-mcp → vw-agent 反向依赖。
- `mcp_app_execution.rs`：宿主 ToolRuntimeContext、session_tools 与 execute_bound_app_tool 尚无已迁出的执行所有者；绑定必须保留完整会话工具集、租户、审批发送/接收器、撤销 token，不能降为 JSON 或万能回调。
- `mcp_apps.rs::resource/validate_resource`：依赖原 mcp_resources::request/validate_contents 与 RunningMcpPeer；当前保持原实现而非复制资源校验。

### 静态验证与后续验收

定向 git diff --check 与反向依赖搜索通过（无匹配）；已逐文件审阅迁移差异。静态 TOML 解析尝试失败：当前 Python 无 tomllib/toml 模块，未安装依赖；清单仅人工审阅，不能记解析通过。未执行 Cargo、测试或实机验证，不声称编译通过。
既有测试 t19_tests.rs、t24_tests.rs、t27_tests.rs、t27_flow_tests.rs、t28_tests.rs、t29_tests.rs 仍包含宿主连接/OAuth等未迁职责，本轮留原路径经薄重导出消费新实现；未来与其真实所有者整体迁移，未复制或弱化断言。后续需验收原平台/特性组合、MCP wire metadata、分页边界、表单租户隔离/取消与 UI 资源读取。


## 2026-11-16 恢复：先解除 24 客户端类型环

当前仅写独占领域文件；按最新指令公共接线与 Git 必须申请 22 窗口，未暂存或提交。此轮不是任务完成。

下层唯一所有者明确为 mcp-client（不依赖 plugin-mcp/agent-plugins）：
- 新 `src/protocol.rs`：FormOwner（原 plugin-mcp/mcp_elicitation）与 RawResponse（原宿主 mcp_resources）的完整状态/方法，`take` 为实际跨包协议请求公开。
- 新 `src/notifications.rs`：McpCatalogs、ToolCatalogBinding 与内部 Catalog/Listed 完整迁入。tools 仍取 Peer；resources/templates/prompts 改为 `(cacheable: bool, fetch: F)`，只注入原 list_entries 动作，不携带应用上下文。闭合 token、revision、分页重试与刷新互斥保持原样。tools_revision 为现存宿主测试调用提供公开观察方法，不能留 dependency 的 cfg(test)。
- 新 `src/catalog.rs`、`capabilities.rs`、`wire_result.rs`：原分页、协议版本校验与原始结果保留完整下沉，plugin-mcp 原模块仅重导出；wire_result 的 WIRE_RESULT 同一常量供上层 normalize 使用。
- 宿主 mcp_notifications 为薄重导出；mcp_resources 的 RawResponse 改薄重导出，其三个目录调用显式传入原服务能力 list_changed 与原 list_entries 闭包。mcp_resources 其余权限与业务执行保持原样。不修改任务24的 tools/mcp_common 或任务153的 agent_apps/mcp。

### 请求 22 公共文件/Git 窗口（尚未执行）

1. `crates/libs/mcp-client/src/lib.rs` 增加公开 `protocol`、`notifications`、`catalog`、`capabilities`、`wire_result` 五模块，并重导出 `protocol::{FormOwner,RawResponse as RawMcpResponse}` 和 `notifications::{McpCatalogs,ToolCatalogBinding}`，供24直接消费真实类型。
2. `crates/libs/mcp-client/Cargo.toml` 增加实际依赖 `tokio = {version="1", features=["sync","time","macros"]}`、`tokio-util = "0.7.18"`、`vw-api-transport = {path="../api-transport",version="0.6.0"}`。无需 plugin-mcp/agent-plugins；现有 rmcp client/anyhow/serde_json 复用。
3. 根成员与 vw-agent 对 plugin-mcp 的依赖接线仍按前述清单。plugin-mcp 清单已添加 mcp-client 正向依赖。
4. 此轮应提交的新增文件精确为 mcp-client/src/{protocol,notifications,catalog,capabilities,wire_result}.rs；禁止夹带24的 mcp-client 其他未提交内容。宿主来源为本卡原五文件加 mcp_notifications.rs、mcp_resources.rs；目标 plugin-mcp 内部与本卡记录共同提交。公共文件仅提交窗口中本次接线差异。

### 173 授权范围

此次173范围仅消费 `compat/mcp_notifications.rs` 全文件目录状态、`compat/mcp_resources.rs` 中 RawResponse 与三个目录调用接线；mcp_resources 的资源/提示词业务执行仍在173。OAuth/声明其余授权尚未写入，不表示已完成迁移。为24提供真实下层类型是本次先行闭环，不宣称47全部闭环。

静态证据：定向 git diff --check 通过；新增五个下层模块搜索无插件/宿主反向依赖。Cargo/测试/程序启动均未运行。下一步需22授予上述公共接线和 Git 窗口，落实真实导出后24才能继续客户端迁移；随后47消费实际 RunningMcpPeer/open_server_peer 导出推进原剩余范围。


## 2026-11-16 公共接线窗口落实

已获22窗口，基于24基础提交 d5e737d6f，仅扩展 mcp-client 五模块导出及 tokio(sync/time/macros)、tokio-util、vw-api-transport 依赖，保留原 helpers/config/tests。根成员与宿主依赖已接通 plugin-mcp。FormOwner、RawMcpResponse、McpCatalogs、ToolCatalogBinding 现由 mcp-client 根公开，协议能力/分页/wire_result 由公开子模块提供；客户端不依赖 plugin-mcp 或 agent-plugins。plugin-mcp 的 FormOwner/能力/分页/wire preservation 是薄重导出，不复制状态。之前“请求窗口/尚未执行”的条目已由本节替代。

按授权用 git-commit 技能与 /tmp/shiju-scoped-commit.py 独立索引；清单/lib 使用基于HEAD精确 roots，绝不暂存 mcp-client 整目录。47残余连接池、宿主app执行、OAuth/声明，以及173资源业务仍待下层客户端执行接口/具体所有者闭环；本卡保留，不宣称完成。已有173目录状态与RawResponse局部迁出清单保持前节精确范围。

静态验收：pip._vendor.tomli 解析四清单；git diff --check；逐文件迁移差异审阅；下层反向依赖搜索。Cargo/测试/启动/探测全部未运行，编译及行为仍待授权验收。
