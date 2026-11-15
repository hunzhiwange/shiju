# 任务 24: 工具能力 mcp-client-1

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 1（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：无

历史审计编号：原任务 97，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 无本计划内已确认前置；核对已有基线与本卡实际导出后即可进入就绪队列，不等待其它领域。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 1 的 `crate::project::instance::project`；任务 158 的 `crate::plugins::mcp_session_connections`；任务 190 的 `crate::tools::current_tool_use_context`；任务 197 的 `crate::agent_apps::mcp::runtime::selected_runtime_servers`；任务 269 的 `crate::session::ui_store::current_tenant_scope`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
- 执行时滚动调度，编号仅提供可行线性顺序；只等待本卡明确列出的前置完成并集成。在途旧编号按任务编号对照认领，禁止重复执行。

状态：已阻塞

目的：迁移，将本任务职责归入 `mcp-client`。

来源任务：用户要求尽可能全部拆分vw-agent；覆盖映射见coverage.md。

预计会话范围：一个领域的一组源码迁移；大领域已拆成连续任务，同包逐项接线。测试源码和资源仅随实现迁移，不执行。

## 并行实施边界

- 按 plan.md 的并行协议执行：独立工作树负责领域实现，主协调会话负责公共接线与逐项合入；禁止在同一检出目录同时执行并行任务。
- 本任务只修改下方来源文件、对应目标包及本编号任务记录。根清单、vw-agent 清单/lib.rs、清单外父模块和直接调用者由协调会话独占修改。
- 开始前检查当前基线的 use/super/宏及具体符号所有者。候选引用不是完整依赖图；发现需要另一在途任务的新接口时，报告具体符号，协调者将依赖方顺延，不猜签名或新造端口。
- 将公共接线逐文件列入本任务的“接线交接”记录，给出原/新导入、feature/cfg、注入位置和需删除的旧实现；不得仅交付泛化建议。
- 领域差异静态审阅后用 $git-commit 提交本工作树变更，保留本卡，报告提交与接线清单。此时仅为待合入产物，不标记已完成。协调会话接线、合入并复核后提交删除本卡，再归档已合入工作树。

## 文件

- 目标：`crates/libs/mcp-client/Cargo.toml`、`crates/libs/mcp-client/src/lib.rs` 及保持原职责布局的子模块；包已存在则修改。
- 公共接线（仅协调会话修改，本任务提供精确交接）：根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`、`crates/vw-agent/src/lib.rs`、相关原mod.rs与搜索确认的直接调用者清单。
- 来源及相邻测试/资源：
- `crates/vw-agent/src/tools/mcp_common/mod.rs`
- `crates/vw-agent/src/tools/mcp_common/tests.rs`
- `crates/vw-agent/src/tools/mcp_common/wire_check.rs`
- `crates/vw-agent/src/tools/mcp_common/wire_http.rs`
- `crates/vw-agent/src/tools/mcp_common/wire_stdio.rs`

## 上下文

- 重点符号：`NoopClient`、`unsupported_mcp_capabilities_json`、`load_resolved_acp_config`、`load_resolved_acp_config_from`、`load_resolved_runtime_mcp_config_from`、`load_resolved_acp_config_from_with_plugin_options`、`with_server_peer`、`with_server_peer_timeout`、`with_server_client_timeout`、`RunningMcpPeer`、`runtime_stdio_cwd`、`open_server_peer`、`open_connection_check_peer`、`management_for_server`、`management_timeout`、`mcp_tool_allowed`、`auth_method_names`、`server_name`。
- 当前文本引用候选：`plugins`、`plugins::mcp_elicitation`、`tools::current_tool_use_context`、`agent_apps::mcp`、`plugins::validate_mcp_server_protocol`、`plugins::preserve_mcp_wire_result`、`config::schema`、`plugins::mcp_client_info`、`project::instance`、`session::ui_store`、`config::get_project_for_tenant`、`plugins::mcp_session_connections`、`plugins::plugin_mcp_runtime_cwd`、`plugins::plugin_mcp_oauth_access`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
- 领域边界：具体工具按能力分包，工厂由 tool-assembly 组合；涉及会话、插件与调度器的工具只依赖前置端口，不反向依赖会话或主循环。 MCP连接与协议客户端不得依赖 agent-plugins；插件鉴权/交互请求由调用者提供，客户端只持有现有请求边界。

## 步骤

1. 读取上述源码、父模块声明、相邻测试和调用者；用 `rg -n 'crate::|super::|pub\(crate\)|include!' crates/vw-agent/src` 限定到本任务查看实际依赖，记录旧路径到目标模块映射。测试文件单独排程时按其生产实现实际所有者迁入，不为测试再建包。
2. 按领域边界迁移本任务源码；剥离应用宿主动作到已存在的窄接口，更新资源路径与必要可见性。禁止新增未调用抽象，不合并行为不同的旧实现。
3. 建立/扩展目标最小清单，复用已拆出的真实所有者；完整转发原feature/cfg。将调用者正向接线和旧位置薄重导出列入交接，由协调会话实施。后续任务尚未迁的调用者通过真实宿主适配接线，不让新包依赖它。
4. 逐项审阅参数/错误/异步取消、租户、权限、全局状态、序列化与平台语义等价；仅调整测试源码导入，不运行或追加测试。将尚需闭环迁移的宿主适配具体路径写入本任务提交正文。
5. 按并行实施边界提交领域变更与接线交接，保留本卡供协调会话合入后收尾；不得提前宣称任务完成。

## 静态验收

- 运行：`git diff --check`；逐文件阅读本任务完整差异和迁移后的清单。
- 运行：`rg -n 'vw-agent|vw_agent|crate::app' crates/libs/mcp-client`；审阅是否存在库对主包的反向依赖，文档与测试引用单独检查。
- 检查：本任务全部来源已迁/明确为组合转发，生产实现不重复；类型/状态只有一个所有者；工厂注册和资源完整；后续文件已分配所有者。
- 所需证据：真实迁移映射、最小依赖、窄端口调用者、cfg/feature、直接调用接线、测试/资源迁移清单及差异检查结果。仅静态证据，不能声称编译或行为通过。
- 待统一验收：本任务来源的既有测试、直接调用集成、原支持平台与特性、涉及UI的渲染和外部能力；将具体路径及组合写提交正文，本轮全部禁止运行。

## 2026-11-15 执行记录与接线交接

本次遵守主协调会话最新指令：当前 checkout，不建 worktree，不执行 Cargo/测试，不暂存或提交；等待协调者串行提交窗口。领域只完成以下独立部分，连接层未完成，不可删卡。

### 已迁移部分

- `tools/mcp_common/mod.rs` 的 `merge_global_mcp_server_values` 至 `apply_mcp_server_override_values` 连续纯配置实现 → `crates/libs/mcp-client/src/config.rs`；只有两项真实宿主调用入口改为 `pub`，其余保持私有。
- 原 `management_for_server` 至 `resource_contents_json` 连续实现 → `crates/libs/mcp-client/src/lib.rs`；包含 timeout、allow/deny、认证方式名称、server 名称/传输/查找/摘要、资源 JSON；函数体原样迁移。
- 原 tests.rs 中 7 个自足测试随对应实现迁入新包 `src/tests.rs`，原断言不变；配置、连接及带宿主依赖的测试仍留宿主。
- 最小依赖：acp-config、agent-client-protocol（保留原两项 unstable feature）、anyhow、rmcp（仅 client，默认 feature 关闭）、serde_json、tracing。新迁移代码没有平台 cfg，未改变原连接层 cfg。

### 本次公共接线（协调者独占）

1. 根 `Cargo.toml` workspace members 增加 `crates/libs/mcp-client`（若已有匹配则不重复）。
2. `crates/vw-agent/Cargo.toml` dependencies 增加 `mcp-client = { path = "../libs/mcp-client", version = "0.6.0" }`。
3. 本卡允许修改的 `tools/mcp_common/mod.rs` 已以 `pub use mcp_client::{auth_method_names, find_server_by_name, management_for_server, management_timeout, mcp_tool_allowed, resource_contents_json, server_name, server_summary, server_transport}` 替换旧实现，并私有导入两个配置合并入口；所有旧直接调用者保持原导入仍有效。不需要修改 `tools/mod.rs`、宿主 lib.rs 或工厂注册。
4. `vw-agent` 原 MCP helper 测试保留必要宿主集成断言；7 项迁走测试不要重复保留，原生产 helper 实现已删除。

### 阻塞的具体接口证据（不是候选引用）

- `NoopClient` 字段实际持有 `crate::plugins::McpCatalogs`、`RawMcpResponse`、`mcp_elicitation::FormOwner`；`create_elicitation` 调用 `mcp_elicitation::request` 并传递 request context cancellation、closed token 和原始 schema。
- `wire_http.rs`、`wire_stdio.rs` 真实使用 `RawMcpResponse::capture` 与 `preserve_mcp_wire_result`；HTTP 协议捕获依赖 `validate_mcp_server_protocol`。
- `plugins/compat/mcp_notifications.rs:140,156,176` 的 resources/templates/prompts 直接接收 `RunningMcpPeer`，构成客户端类型与目录状态的闭环。该文件属于卡 173；`mcp_content.rs` 与 `mcp_elicitation.rs` 属于卡 47。本卡不能越界迁移，也不能让 mcp-client 依赖 plugin-mcp/agent-plugins。
- 剩余 `open_server_peer_inner` 真实调用插件 OAuth、HTTP header、cwd 动作；`with_server_client_timeout` 真实使用会话连接缓存；配置加载真实读取项目、租户、tool-use context、插件发现和 managed runtime selection。这些行为仍在原宿主源码，未伪造无状态替代。
- 已调查目标库：执行开始时 `crates/libs/mcp-client`、`agent-plugins` 不存在，相关类型只有宿主所有者。需要协调者与卡 47/173 明确共享协议类型/目录状态的下层唯一所有者，并交付真实宿主交互、OAuth、cwd、会话连接边界，才能继续迁出 NoopClient、wire_check/http/stdio 与剩余加载/连接入口。

### 静态证据与剩余验收

- `git diff --check -- crates/vw-agent/src/tools/mcp_common crates/libs/mcp-client` 通过；新文件另逐项检查尾随空白。
- 用 Python 比较 git HEAD 的原函数体和迁移后源码，两段生产实现及 7 个测试体保持等价（只新增必要 pub）。这是静态文本比对，不是测试执行。
- `rg -n 'vw-agent|vw_agent|crate::app' crates/libs/mcp-client` 无匹配，没有反向宿主依赖。
- Python 系统环境无 tomllib/toml，未能自动解析 TOML；已人工核对清单，协调者集成时补静态 TOML 解析。
- 未运行 Cargo、单元测试及实机测试，未更新 Cargo.lock，未提交。未来验收需覆盖新包 7 项测试、宿主剩余 config/peer 测试，以及全部原支持的 native/wasm cfg 和连接生命周期；本轮禁止执行。

## 2026-11-16 第一阶段公共接线与限定提交

本次获公共接线/Git窗口，仅提交本卡原有纯配置与元数据迁移。根 workspace 已注册 mcp-client，vw-agent 已加入真实路径依赖，宿主 mcp_common 通过重导出消费真实库实现。47 的 protocol.rs、notifications.rs、catalog.rs、capabilities.rs、wire_result.rs 与 plugin-mcp 全部排除，不在本次 lib.rs/Cargo.toml 接线。

复核发现宿主还有3项直接调用私有解析器的既有测试，已一并迁入新包 src/tests.rs；测试模块由 config.rs 引入以访问其私有实现，不额外公开解析器/字段。目前累计迁移10项既有测试，原断言及函数体不变；宿主其余配置集成与连接测试保留。旧测试无用 ResourceContents 导入已移除。

静态证据：pip._vendor.tomli 解析根/宿主/目标三份 TOML 成功；两段迁移生产函数体及10项测试体与 HEAD 原文比对等价；限定 git diff --check 通过。未运行Cargo、单元测试及实机测试，待统一验收；未更新锁文件。

状态仍为已阻塞，不删卡：尚需47先独立接线提交下层真实导出，再由24消费并继续完整客户端迁移。下一窗口继续核对目录状态/RunningMcpPeer闭环、交互及OAuth等宿主边界。此前“未提交”仅描述11月15日现场，本阶段使用独立索引限定提交，下层五模块和其它任务变更原样保留。

## 2026-11-16 第二阶段：消费47真实类型，待公共接线/Git窗口

已核对47提交 cb7d23bd1 的真实导出；本阶段不修改其 protocol/notifications/catalog/capabilities/wire_result 五模块或 plugin-mcp。

### 当前独占领域产物

- `tools/mcp_common` 的 NoopClient、unsupported capability policy、remote header/client、connect timeout、RunningMcpPeer 与完整握手实现迁至 `mcp-client/src/client/mod.rs`。
- wire_check/http/stdio 完整迁至 `mcp-client/src/client/`；旧 wire_check 删除，旧 wire_http/stdio 仅重导出 ConnectionCheckCleanup/ProbeChild，保留现有探针调用路径。
- 用真实 `mcp_client::{FormOwner, RawMcpResponse, McpCatalogs}`，协议/原始结果直接调用下层 capabilities 与 wire_result；没有复制状态、plugin-mcp 依赖或 Any/万能JSON上下文。
- 仅为已有真实调用新增 `ClientHost` 三项强类型动作：stdio cwd、异步 HTTP scoped headers（含原OAuth调用）、异步 elicitation request。宿主 `AgentMcpHost` 委托原实际函数；owner 的租户/session、request cancellation、closed token、原始wire schema全部原样保留。
- NoopClient持有注入host，不存在默认授权；Default仍无form owner/host，不声明交互能力。HTTP探针仍执行原header验证路径；stdio cwd、private process group、PATH增强、响应保留、协议拒绝后cancel、禁重试、取消顺序与清理等待维持原控制流。
- 随实现迁移10项既有客户端测试与2个异步辅助函数至 client/tests.rs；宿主配置与真实连接集成测试保留。未运行或新增测试。
- 宿主仍有 scoped配置加载、插件发现合并、managed runtime选择和session连接缓存适配；这些拥有宿主状态的真实动作维持原路径。完整迁移尚未最终静态闭环，不删卡。

### 就绪申请公共接线/Git窗口（当前归50，本轮未写）

1. `mcp-client/src/lib.rs` 仅新增 `pub mod client;`，不动47五模块。
2. `mcp-client/Cargo.toml`：rmcp 在现有client上追加 transport-io、transport-child-process、transport-sse-client-reqwest、transport-streamable-http-client-reqwest；tokio追加rt/process/io-util；tokio-util启用codec。新增真实使用依赖 futures-util 0.3.32、reqwest_012 0.12（default-features=false/json/rustls-tls-webpki-roots/stream）、sse-stream 0.2.1、url 2、axum 0.8.8（default-features=false，仅http::Uri）、libc 0.2、shell-command路径../shell-command 0.6.0启用shell-tokio。拟清单已写 /tmp/shiju24-client-manifest.toml 并由pip._vendor.tomli静态解析通过，应用前重新核对最新清单。
3. 宿主调用者/父模块无需新增修改：mcp_common函数签名维持原样；RunningMcpPeer转发新类型；探针cleanup及child走薄重导出。根、宿主清单已有路径依赖，无额外增量。
4. 提交限定本卡来源、目标client子目录、lib模块声明和清单精确增量及24卡；不碰50独占session-port、child_agent/task_session、tools/mod/agent_hosts/toolset/thread_tool。

### 本阶段静态证据

三个wire文件与HEAD原文在必要crate路径/pub替换后完全一致；迁移的12个测试/辅助函数体逐项原文比对通过。git diff --check限定领域通过，新client目录没有crate::plugins/vw_agent/plugin_mcp反向依赖。尚未公共模块/清单接线，当前不是可交付完整目标；需窗口补齐接线并复核提交。未运行Cargo、单元测试及实机测试，待统一验收。未来验证需覆盖两地既有测试和原native/wasm支持组合、stdio进程组/HTTP探针清理、租户身份与OAuth/交互取消路径。

## 2026-11-16 第二阶段独占 API/清单接线完成，申请 Git 租约

获授 mcp-client 的 lib.rs/Cargo.toml 精确增量后已实施上节清单接线，未修改全局根或宿主清单、其他通用父模块、调用者或47来源。

真实公开 API（已写源码可供47/56消费）：

- 根导出 `mcp_client::{ClientHost, NoopClient, RunningMcpPeer, ConnectionCheckCleanup, ProbeChild, unsupported_mcp_capabilities_json, open_server_peer}`。
- `open_server_peer(host: Arc<dyn ClientHost>, server: McpServer, timeout: Duration, closed: CancellationToken, form_owner: Option<FormOwner>) -> Result<RunningMcpPeer>`。
- native 根导出 `open_connection_check_peer(host: Arc<dyn ClientHost>, server: McpServer, timeout: Duration, http_io: CancellationToken, child: Option<&mut Option<ProbeChild>>, cleanup: Option<ConnectionCheckCleanup>) -> Result<RunningMcpPeer>`。内部握手入口不公开；探针不接受form owner。
- `ClientHost` 已在 `src/client/mod.rs` 给出实际签名：同步 stdio cwd；借用server/owner的异步 headers；持有 owner/server/request/cancelled/closed/wire_schema 的异步 elicitation。只有已有真实调用，没有默认授权或无调用抽象。
- 宿主 `mcp_common::open_server_peer/open_connection_check_peer` 已直接转发上述根 API 并注入 AgentMcpHost；删除宿主多余 inner 包装，保留现有外层签名和探针导入路径。

复核：47的5模块 git diff 均为空；其已有依赖除本次必要 rmcp/tokio/tokio-util 特性追加外全部保留；TOML解析通过；3个wire机械迁移等价；限定diff --check通过。未运行Cargo、任何测试及实机测试。源码已接线，Git租约尚归98，本轮不暂存/提交。

请求下一 Git 提交窗口；范围为mcp-client/Cargo.toml、src/lib.rs、src/client/五文件、tools/mcp_common本卡来源五文件和24卡。继续限定独立索引提交，不纳入他人差异。24卡尚保留未闭环状态，提交前最终审查宿主配置/会话适配保留范围和调用者类型一致性；不因接口已公开就删卡。原 scoped config/project/tenant/plugin merge 与session缓存真实宿主适配暂留，需与相应领域所有者继续闭环。

## 2026-11-16 第二阶段提交范围复核

已获24的全局清单/Git租约；本阶段不需要全局根/宿主清单增量，仅将已完成领域产物以独立索引限定提交。现HEAD包含175的0eec5d253及后续桌面提交，均原样保留；当前工作区的桌面workflow暂存内容不纳入本次提交。47五模块仍无本卡修改。

本卡来源责任复核：

- 原wire_check生产实现已移走并删除旧文件；wire_http/stdio旧文件仅剩真实类型薄重导出，没有第二份wire实现。
- NoopClient、协议能力拒绝、握手超时、transport初始化、HTTP协议/原始响应与stdio进程生命周期全部在新client模块；RunningMcpPeer唯一关联新NoopClient，插件47类型保持唯一所有者。
- mcp_common剩余生产文件的必要性：工具入口解析当前tenant/project/tool context；组合全局配置、插件发现及managed runtime selection；从会话缓存取得连接；AgentMcpHost调用插件OAuth/headers、交互及cwd，并调用新库握手API。这些路径仍使用原真实宿主，不反向引入新库。
- 尚未完全迁走的客户端配置编排：load_resolved_mcp_config/merge_runtime_mcp_servers中的配置合并顺序与native_mcp_selection过滤，仍与宿主上下文读取同处。后续需要在保留宿主范围/权限语义的前提下，将可独立编排下沉并留下窄组合入口；因此本阶段不假称整卡闭环，不删24卡，状态保留已阻塞。
- with_server_peer/with_server_peer_timeout只转换Peer回调，with_server_client_timeout保持原会话缓存作用域/持久连接调用，未新建重复会话状态；runtime_stdio_cwd及AgentMcpHost属于必要真实宿主适配，后续领域接线消费当前公开ClientHost接口。

提交正文记录本阶段映射、API、依赖/feature与待验路径；已有静态等价/TOML证据继续有效，本轮仅刷新限定差异与提交空白检查，不重复运行已通过检查。未运行Cargo、单元测试及实机测试，待统一验收。完成本阶段提交后归还Git租约。
