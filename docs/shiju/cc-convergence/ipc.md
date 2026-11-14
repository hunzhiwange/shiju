# Tauri 命令调用者收口

2026-11-14。任务17。本轮按用户要求只实现与静态审查，不运行任何 Cargo、构建、单元测试或实机测试。本文不构成行为验收通过证据。

## 当前调用链与最小删除集合

原来源注册入口为 `crates/vw-agent-apps/upstream/src-tauri/src/lib.rs` 的 `tauri::generate_handler!`，其中供应商、配置、账号等 `commands::*` 只属于来源程序。正式 `crates/vw-agent-apps/src/lib.rs` 未引入 upstream 模块；正式桌面为 Rust/Iced，不运行来源 TS hooks 或来源 Tauri 注册表。

对 `crates/vw-desktop/src`、`crates/vw-gateway-client/src`、`crates/vw-agent/src/gateway/api`、`crates/vw-agent-apps/src` 查询 `tauri|generate_handler|invoke\(`，没有正式 Tauri 调用/注册。残余命中逐项保留：

- `gateway/api/handlers/resumable_stream.rs` 的 cancellation.invoke 是 Rust 取消回调，与 IPC 无关。
- `gateway/api/handlers/file_import.rs` 的 tauri origin 是来源字符串校验，不是命令分发器。
- `agent-apps/src/sessions/terminal/otty.rs` 的 invoke 是受控 OTTY CLI 调用，不能按函数名删除。
- `agent-apps/src/mcp/adapters/codex_tests.rs` 的 upstream 引用仅为来源 golden fixture，交由任务22先迁移夹具后处理，不是运行 IPC。

全 crates 排除 upstream 后查询 `@tauri-apps|tauri::|generate_handler!|invoke_handler` 无匹配（rg退出1）；四个正式包清单查询 Tauri/invoke 亦无匹配。故本任务最小运行代码删除集合为空，不新增另一层 IPC 适配器或动态命令入口。

## 原业务调用者的正式去向

桌面交接的来源行为索引见 `docs/shiju/desktop-integration/acceptance-1.md`；本轮各域已绑定真实符号、权限及所有者：

|来源职责|正式SDK/生产去向|本轮领域证据|
|---|---|---|
|应用安装、升级、取消|agent_apps_read_api / agent_apps_lifecycle_api → agent_apps生命周期handler|installation.md|
|供应商与原生配置|app_providers_api → AppProviderStore / ProviderTransactions|providers.md、config.md|
|模型目录|model_catalog_api → models目录解析|models.md|
|账号授权与刷新|agent_apps_auth_read/accounts/flow → AuthAccountStore|accounts.md|
|MCP|mcp_management_read/write/distribution → McpCatalogStore|mcp.md|
|技能/插件|agent_apps_skills → 安装事务及各作用域存储|skills.md|
|提示词/规则|agent_app_prompts_api / agent_app_prompt_resources_api → PromptsStore|prompts.md|
|外部会话|external_sessions → 授权catalog及应用适配器|sessions.md|
|用量/价格|apps_usage_read/write_api → usage账本与价格规则|usage.md|
|后台调度与通知|领域生命周期及usage事件，不转发来源事件名|scheduling.md、events.md|
|原生路由与会话relay|独立绑定与权限，不合并成任意主机入口|relay.md|

上述证据均位于本文同目录。原生窗口、文件选择、外链和托盘等本机动作继续由 Iced/系统集成处理，不强行改成远端业务命令。工作区与组合分别使用 workspace_memory_api、project_profile_api；备份/同步使用现有强类型领域合同，自动同步未具备授权计划时明确拒绝，不能因清理IPC声称完整自动上传已交付。

`crates/vw-agent/src/gateway/router.rs::handler_router` 通过固定领域 router 合并注册；实际注册位置不是任务卡过时提示的 api/mod.rs。`handlers/agent_apps/access.rs` 的管理员 principal/host/grant 策略保持默认拒绝，请求头不能建立主机所有权；桌面可见性不能代替服务端授权。没有把未知来源命令名映射为任意主机执行的通配桥接。

## 静态证据、验证与恢复

- RustCodeGraph status退出0，随后 explore查询真实SDK/账号符号；图中可见正式GatewayClient调用者。新增文件与配置使用rg补充核对。
- 上述源码查询退出0（列出非IPC残余）或1（无匹配）；不是行为测试。最初使用过时routes文件名/未匹配glob的查询未产生证据，已改查实际gateway/router.rs和整个message目录。
- 本任务只新增本文；不改变API/SDK契约、业务数据或运行权限。不新增字符串匹配业务测试。
- 基线 `d10e53d54`。恢复只需撤回本文；来源快照整体归档/移除由任务21/22处理，不能提前删除仍被测试include的夹具。
- 未运行单元测试、未知路由HTTP请求、跨作用域行为验证、数据恢复或macOS实机渲染；后续统一验证需覆盖正式SDK路由、未知命令拒绝和授权失败无副作用。
