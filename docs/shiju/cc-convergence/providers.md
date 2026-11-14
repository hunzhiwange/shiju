# 供应商唯一写入路径

2026-11-14，任务2。依赖任务1的[所有者基线](准入与所有者.md)。本轮按用户指令只做实现与静态审查，不运行 Cargo、单元测试、实机或回归；以下结论不代表行为验收通过。

## 现有链路与零删除依据

应用供应商的权威记录在 `crates/vw-agent/src/app_providers.rs::AppProviderStore`，固定 namespace `app_providers.records`，键包含应用与供应商ID，数据库操作均传递 `TenantScope`。`save/create/update` 分别保留 upsert、仅新增、局部更新语义；它们不是三个独立所有者。`save` 和 `update` 共用 `vw_agent_apps::providers::update::update` 校验及合并逻辑，通过 `insert_if_absent/compare_exchange` 提交同一记录。

- SDK `crates/vw-gateway-client/src/client/app_providers/update.rs::app_provider_update` 发送 PATCH，统一应用供应商路径与请求上下文，不直接写本机配置。
- API `crates/vw-agent/src/gateway/api/handlers/app_providers/update.rs::update` 传递租户与数据库到该存储；Claude 专属请求进入 `claude::write`，保留主机授权、发布输入与原生事务。目录更新与主机激活的副作用边界不能合并。
- 领域 `crates/vw-agent-apps/src/providers/update.rs::update` 只返回校验后的记录，无数据库、桌面状态或旧来源写入；原生文件发布由 `providers/live/` 与各应用适配器负责。
- Iced 入口为 `crates/vw-desktop/src/app/message/app_providers/`，通过网关 SDK 操作应用供应商。平台模型设置另有独立语义。
- `crates/vw-agent/src/gateway/api/handlers/provider.rs` 与 `client/provider_api.rs` 属推理供应商目录和 OAuth，应用供应商根为 `/app-providers`。按原有契约保留两者，不因名称相近删除推理接口。

只读来源 `crates/vw-agent-apps/upstream/src-tauri/src/services/provider/`、`database/dao/providers.rs` 及前端 `src/lib/api/providers.ts` 留在来源快照内。正式 `src/lib.rs` 没有注册 upstream 模块；正式供应商、SDK生产链路没有对 `ProviderService` 的运行调用或 `src-tauri` 模块依赖；`providers/snippets.rs` 保留敏感字段规则的来源注释，MCP测试仍有快照 include，不视为供应商运行写入。来源整体归档和移除属于任务21、22，本任务不局部删快照。

本次没有发现同一权限、租户和主机范围内需要切换的第二写入所有者，生产删除集合为空；不添加替代业务、迁移、兼容层或空泛字符串测试。供应商范围内仅保留本证据，未修改运行行为。

## 静态审查与后续验证

使用 RustCodeGraph status/explore 查询现有索引，并对实际文件核对 `AppProviderStore`、`app_providers.records`、`app_provider_update` 与来源引用。图对常见 create/update 有跨域同名边，未将其当作精确生产调用证明。

复用现有 `crates/vw-agent/src/gateway/api/handlers/app_providers/update_tests.rs::cc04_update_api_denials_and_invalid_inputs_have_no_side_effects` 等实际路由测试，以及 `crates/vw-agent/src/app_providers/model_edits_tests.rs::catalog_defaults_provider_transaction_reopens_without_cross_scope_changes`，后续统一验证必须重新运行并确认有效测试数。重启读取、跨租户隔离、失败无副作用、原生发布及旧来源不变化，本轮均未执行；没有业务夹具前后哈希或测试通过结果。

恢复基线为 `af965519d`；本任务无数据和生产行为变更，无业务数据回滚步骤。恢复本次文档只需逆转对应文档提交，不能整体 reset 或覆盖其他修改。来源引用许可与完整哈希由任务1基线及任务21归档负责。
