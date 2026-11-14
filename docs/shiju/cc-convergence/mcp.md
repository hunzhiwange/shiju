# MCP 唯一管理与分发路径

2026-11-14，任务6静态核对。用户本轮要求只实现，不运行任何 Cargo、单元测试、实机测试或最终回归。本文不代表行为验收通过。

## 权威记录与调用链

- 母任务11来源见 `docs/vibewindow/agent-apps/mcp/后端来源与能力.md`；该历史盘点中的未交付描述不替代当前源码。正式 `crates/vw-agent-apps/src/lib.rs` 已导出 `mcp`，`upstream` 不属于正式模块树。
- Iced `crates/vw-desktop/src/app/message/settings/mcp_management_form.rs::save` 调用 SDK `mcp_management_create/update`；SDK `crates/vw-gateway-client/src/client/mcp_management_write.rs` 仅向所选网关 `/v1/agent-apps/mcp` 写入，不另存桌面目录。
- 网关 `crates/vw-agent/src/gateway/api/handlers/mcp_management_create.rs::create_catalog_record` 授权租户/项目及目标主机后调用 `McpCatalogStore::create`。更新、删除分别由同目录 `mcp_management_update.rs`、`mcp_management_delete.rs` 委托同一 store。
- 权威所有者是 `crates/vw-agent/src/agent_apps/mcp/` 的 `McpCatalogStore`。`catalog_create.rs::catalog_location` 使用 `agent_apps.mcp.catalog.v1` 命名空间，并区分项目；底层 `GatewaySettingsStore` 保存带租户身份的记录。`mod.rs::save_entry` 以 compare_exchange 保证修订冲突明确失败。
- 单项与批量分发在 `mcp_management_bindings.rs` 汇入同一 `execute`，经目录及原生写入授权、可信安装发现后组装 `McpBindingTarget`。`bulk.rs::set_app_bindings` 按记录ID和修订去重尝试，再调用 `binding.rs::set_app_binding`。
- `binding.rs` 保存期望绑定后解析执行凭据，`publish_binding` 核对修订与受管所有权，并经 `NativeMcpWrite` 和 `ProviderTransactions` 进行主机文件投影。失败保留期望修订和明确结果供重试；文件投影并非第二份权威目录，不强行合并两类持久化。

## 保留边界与零删除依据

目录定义、CLI 原生文件、运行时连接和 ACP 会话选择职责不同。`crates/vw-agent/src/gateway/api/handlers/mcp.rs` 的工作区检查及会话绑定工具调用，和 SDK `mcp_api.rs` 的运行时接口仍有真实调用者，不是受管目录 CRUD 的可删除副本。凭据解析继续由 `AuthAccountStore` 的授权作用域负责，不能改成桌面保存密钥。

`upstream/src-tauri/src/services/mcp.rs`、`database/dao/mcp.rs` 及 `upstream/src/lib/api/mcp.ts` 是只读来源；本域不逐文件拆除快照，统一归档和删除归任务21、22。当前生产目录写入与分发已汇入上述所有者，因此本任务生产代码及接口零变更，不为清理制造新抽象、后门或测试副本。

## 静态检查与后续验证

使用 RustCodeGraph status、explore、node 与 callers 检查上述链路。`callers set_app_binding` 显示生产调用来自 `bulk.rs::set_app_bindings`；其余为既有测试。SDK路径初始猜测 `mcp_management_api.rs` 不存在，已依据实际文件调整为 `mcp_management_write.rs`，不新增替代接口。

现有行为测试入口包括 `crates/vw-agent/src/agent_apps/mcp/{catalog_create_tests,binding_tests,bulk_tests,resync_tests}.rs`、网关 `mcp_management_bindings_tests.rs`、SDK `mcp_management_write_tests.rs` 及 Iced `mcp_management_form_tests.rs`。本轮全部未运行；未创建脱敏数据夹具，也未进行前后哈希、失败恢复或跨租户行为验证，留待统一验证。没有 Cargo 槽位或缓存路径可报告。

恢复基线为本任务前 `f071c9f05`，源码和业务数据均未变。仅需逆转本任务文档即可恢复；不得回滚其他领域或用户现有未提交修改。公共恢复约束见 `准入与所有者.md`。
