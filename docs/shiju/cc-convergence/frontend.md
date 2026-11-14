# TS 与 React 运行入口核对

2026-11-14，任务19。依赖任务17的 [IPC证据](ipc.md)。按本轮用户要求仅静态审查，不运行 Cargo、构建、单元测试或实机测试；本文不是功能验收通过声明。

## 入口及动态资源

来源快照的 `upstream/package.json` 声明 React、Vite、Tauri；`vite.config.ts` 以 src 为根，分别将 `src/index.html` 和 `src/tray-panel.html` 构建到 dist。前者加载 `main.tsx` 并挂载 App，后者加载 `tray-panel/main.tsx` 并挂载 TrayPanel。App 的 lazy/import 页面、业务 hooks、`src/lib/api` 的 Tauri 调用都属于该来源构建闭包，不是正式桌面入口。

正式 `crates/vw-desktop/index.html` 的 data-trunk link 指向 Rust shi-ju binary；正式桌面使用 Iced，`build.rs` 生成字体/图标目录，没有运行来源 renderer 构建。`scripts/web/package.json` 只有 Node 内置静态服务 serve.mjs，所服务 index.html、shi_ju.js、shi_ju_bg.wasm 是 ShiJu Web 产物，不依赖来源 React 包。没有发现正式构建脚本引用 upstream/src 或 upstream/dist。来源自身构建清单保留给任务21/22整体归档处理，本任务不拆毁它。

保留现有 WebView、Office taskpane、TUI inline_visualization 的 HTML/JS，以及图标资源；它们与来源 React 页面职责不同。保留推理策略名 react，不能按单词匹配删除。

## 业务规则的正式去向

来源 API/hooks 的全量文件级归属索引见 `docs/shiju/desktop-integration/acceptance-1.md`；其“待细分”和“行为未验证”仍然有效，不能将该索引当作逐函数等价证明。以下绑定本次清理相关的正式规则所有者与已有测试，详细调用链分别见同目录领域证据。

|来源规则或 hook 族|正式实现及已有测试入口|领域证据|
|---|---|---|
|providers / useProviderActions、表单校验、原生配置写入|agent-apps/providers 的校验、编辑合并及 live 事务；validation_tests.rs、editor_merge_tests.rs、update_tests.rs|providers.md、config.md|
|model-fetch / useModelMetadataFill、预设元数据补空|agent-apps/models/metadata.rs、capabilities.rs；metadata_tests.rs、capabilities_tests.rs|models.md|
|auth、账户选择/刷新|AuthAccountStore 与服务刷新适配器、CredentialLease，已有授权流程测试由领域证据索引|accounts.md|
|mcp / useMcp、useMcpConnectivity|McpCatalogStore 与应用适配器，mcp/validation_tests.rs、native_file_tests.rs|mcp.md|
|skills / 技能安装、发现与分发 hooks|skills 安装事务及授权 API；local_install_tests.rs、repo_install_tests.rs、zip_install_tests.rs|skills.md|
|prompts / usePromptActions、目录设置及模板校验|PromptsStore、prompts/templates.rs、template_metadata.rs 与受控文件投影；现有领域测试索引见提示词证据|prompts.md|
|sessions / useSessionSearch|授权 catalog 及正式会话适配器；具体解析/维护测试见领域证据|sessions.md|
|usage / useMoneyFormat、useAutoExchangeRates、统计校验|usage 账本、价格规则、汇率与读取 API；tests/usage_exchange.rs、usage_tokens.rs，handlers/agent_apps/usage_read_tests.rs|usage.md|
|useUsageAutoRefresh、useUsageEventBridge、useUsageCacheBridge|usage_bridge/events.rs、授权 SSE、SDK stream_usage_revisions；usage_refresh_tests.rs 与 SDK stream_tests.rs；Iced当前以查询刷新，未宣称消费SSE|events.md|
|调度、设置保存后的副作用|GatewaySettingsStore、SyncLifecycle 与分域调度器；自动同步缺授权计划时明确拒绝，不复制来源定时器|database.md、scheduling.md|
|窗口、托盘、主题及临时表单状态|Iced/本机系统集成，非后端数据所有者；保留原生系统动作，历史实现证据见 desktop-integration|ipc.md|

以上不是补齐来源所有业务的声明，也不把失去页面调用视为规则无用。本任务没有移植新 Rust 业务、删规则或改变契约。

## 查询结果与最小删除集合

- RustCodeGraph status退出0，explore查询 useProviderActions、useSettingsMetadata、useModelMetadataFill、useUsageAutoRefresh、useUsageEventBridge；其已索引 React 调用者都位于 upstream 内。
- 在正式 `vw-desktop/src`、`vw-gateway-client/src`、`vw-agent-apps/src`、scripts/web 及桌面 index.html 查询 `(@tauri-apps|react|useUsageAutoRefresh|useUsageEventBridge)`，排除JSON大资源后，仅剩 workflow 两个 Rust 测试中的 react 策略名及 prompts/activation.rs 的 reactivating 英文注释。没有来源 React/hook 运行入口。
- 脚本、工作流、正式 Rust/配置文件对 upstream/src、upstream/dist 及来源npm/pnpm入口查询，仅命中 lifecycle/wsl.rs 的 upstream npm-prefix 注释，以及 mcp/adapters/codex_tests.rs 的来源 golden fixture include。后者是测试夹具，交任务22迁移后处理，不能直接删来源。
- 首次全类型查询被模型JSON大资源及文档字样淹没，未用其作为零调用证据；已按活动入口和脚本文件类型重查。来源HTML初查根路径不存在，已按vite root改查src/index.html、src/tray-panel.html。

正式运行链未引入来源前端，故本任务最小删除集合为空，仅固化零删除依据；来源快照保持不变。基线 fbd619001，恢复只需撤回本文，不碰业务数据或其他任务修改。测试、打包、主题实机渲染及正式版本功能等价均留待后续统一验证。
