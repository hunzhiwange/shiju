# 任务 173: 插件兼容 plugin-mcp-2

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 2（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：任务 47

历史审计编号：原任务 164，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 写入前置任务 47：同一实施范围 `plugin-mcp` 的连续切片，共用目标清单/模块入口；这是写入顺序，不代表其它包存在接口依赖。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 19 的 `crate::question::ask_sensitive`；任务 24 的 `crate::tools::mcp_common::RunningMcpPeer`；任务 28 的 `crate::tools::url_validation::is_private_or_local_host`；任务 158 的 `crate::plugins`；任务 269 的 `crate::session::ui_store::with_tenant_scope`；任务 290 的 `crate::security::SecretStore::is_secure_encrypted`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
- 执行时滚动调度，编号仅提供可行线性顺序；只等待本卡明确列出的前置完成并集成。在途旧编号按任务编号对照认领，禁止重复执行。

状态：未开始

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
- `crates/vw-agent/src/plugins/compat/mcp_environment.rs`
- `crates/vw-agent/src/plugins/compat/mcp_legacy.rs`
- `crates/vw-agent/src/plugins/compat/mcp_notifications.rs`
- `crates/vw-agent/src/plugins/compat/mcp_oauth_discovery.rs`
- `crates/vw-agent/src/plugins/compat/mcp_oauth_flow.rs`
- `crates/vw-agent/src/plugins/compat/mcp_oauth_tokens.rs`
- `crates/vw-agent/src/plugins/compat/mcp_portable.rs`
- `crates/vw-agent/src/plugins/compat/mcp_portable_overlay.rs`
- `crates/vw-agent/src/plugins/compat/mcp_public_auth.rs`

## 上下文

- 重点符号：`resolve_servers`、`runtime_cwd`、`PluginMcpDeclaration`、`PluginMcpServerError`、`PluginMcpParseOutcome`、`parse_legacy_mcp`、`McpCatalogs`、`ToolCatalogBinding`、`OAuthDiscovery`、`endpoint`、`bind`、`discover`、`discover_with`、`public_client`、`PendingAuthorization`、`authorize`、`authorize_with`、`send_provider_request`。
- 当前文本引用候选：`tools::mcp_common`、`question`、`security`、`tools::url_validation`、`plugins`、`session::ui_store`、`question::ask_sensitive`、`config::schema`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
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


## 2026-11-16 由47先行迁出下层契约（授权精确范围）

47公共窗口中：mcp_notifications.rs 的 Catalog/Listed/McpCatalogs/ToolCatalogBinding 完整迁入 mcp-client/src/notifications.rs，原文件薄重导出；resources/templates/prompts 仅接收缓存能力布尔值与原list_entries抓取闭包，取消/关闭/revision/刷新互斥原样保留。mcp_resources.rs 仅迁 RawResponse 到 mcp-client/src/protocol.rs 并调整三处目录调用签名；资源/提示词业务执行、校验和其余文件未迁。173仍未完成，后续不要复制上述共享类型或恢复旧状态。47卡与同次提交正文保存完整接线和验证范围。


### 47第二阶段授权资源协议范围（待窗口提交）

在下层RunningMcpPeer已公开后，47继续迁出mcp_resources::request/validate_contents/list_entries及其meta/string/uri共享校验到plugin-mcp；原路径重导出，call/validate_prompt/visible宿主业务保留。mcp_apps资源读取与校验随后整体迁入plugin-mcp。mcp_legacy、mcp_public_auth、mcp_oauth_discovery也由47本次授权迁入；后续173不要重复移动或恢复原实现。具体依赖与未完成连接宿主动作见47卡最新记录；173未完成。


### 47第四阶段授权范围更新（待Git窗口）

mcp_oauth_tokens与mcp_oauth_flow完整实现已迁plugin-mcp，原文件仅SourceHost注入与薄重导出；LOCKS/TOKENS唯一目标持有。原SecretStore/config目录直接复用其下层所有者，未绕过安全存储。47的连接/App绑定/认证业务现均迁出，具体强类型宿主动作与调用者接线见47卡最新记录。173未列入此次授权的portable/overlay/runtime/stdIO等仍保留，不能宣称173完成。


## 47最终集成边界（2026-11-16，替代此前阶段性待窗口描述）

实际已覆盖173的6个来源：mcp_legacy.rs、mcp_notifications.rs、mcp_oauth_discovery.rs、mcp_oauth_flow.rs、mcp_oauth_tokens.rs、mcp_public_auth.rs。其中notifications目录状态唯一归mcp-client，另五者实现归plugin-mcp；原来源为薄重导出或SourceHost适配。47未接管本卡剩余3个来源：mcp_environment.rs（resolve_servers/runtime_cwd）、mcp_portable.rs（便携协议与环境名称）、mcp_portable_overlay.rs（环境覆盖与来源重验）；仍需本卡执行者迁出这些具体职责并接其真实调用者。

实际调整的既有测试仅构造注入、不增删断言：compat/{t18,t19,t20,t22,t23,t24,t27_flow,t28,t29,t30,t61,t62,t64}_tests.rs与tools/mcp_catalog_refresh_tests.rs。均保留宿主路径，包含portable、stdio、session权限等其它职责，不整体迁走。

额外授权的mcp_resources.rs仅RawResponse/协议request/list_entries/validate_contents及meta/string/uri共享校验已迁；余下call、validate_prompt、visible为尚未迁资源/提示业务，来源路径明确保留，不能因此误删其它覆盖该文件的任务。47自身11来源均已迁或有精确宿主适配，47卡在本次提交删除，173保留这3个未迁来源及后续测试所有权安排。159 provider/transport环由其本卡负责，不能据此阻塞或扩大本次范围。

复核当前HEAD：24客户端153a33a77、153应用服务c6a2b4129、97工具目录ea6f3a415提供实际依赖；workspace/host注册、TOML及path解析与16个构造roots核对通过；未运行Cargo/测试/启动/探测，静态完成不代表编译或行为验收通过。
