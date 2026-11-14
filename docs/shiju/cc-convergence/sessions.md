# 会话唯一索引与写入路径

2026-11-14，任务9。本轮按用户要求只做实现与静态审查，不运行 Cargo、构建、单元测试、实机或最终回归。以下不是行为验证通过声明。

## 实际所有者与调用者

- Iced `crates/vw-desktop/src/app/components/agent_apps/external_sessions.rs` 的加载入口调用 `GatewayClient::external_sessions_catalog`；删除后重新加载也复用此 SDK。SDK `crates/vw-gateway-client/src/client/external_sessions/catalog.rs` 只访问选定网关的 `/v1/external-sessions/catalog`，不扫描桌面历史。
- 网关 `crates/vw-agent/src/gateway/api/handlers/external_sessions/catalog.rs::execute` 经 `authorize_external_history` 解析认证主体、ProjectScope、目标主机、受管实例和根目录，再调用 `scan::scan`；每次翻页重新校验权限，快照绑定主体、项目、主机及实例根目录。
- `external_sessions/scan.rs::scan` 只调正式 `vw-agent-apps::sessions::providers` 的应用适配器；13种来源按身份分发，Codex 文件与状态数据库由其适配器处理，不再从桌面直接调用来源实现。
- `crates/vw-agent-apps/src/sessions/catalog.rs::Catalog::from_batches` 是元数据合并和去重入口：完整身份作为键，相同记录折叠，冲突副本拒绝，跨项目拒绝；稳定排序以完整序列化身份收尾。`Catalog::page` 绑定过滤条件及快照生命周期，不读正文或改写来源。
- 恢复由 `sessions/resume.rs::ResumePlan` 基于已验证身份和目标主机上下文重建 argv；不从显示路径或用户提供命令建立权限，不创建第二份识矩会话索引。
- Codex维护写入由 `external_sessions/maintenance/host.rs::execute` 调正式 `sessions::maintenance` 与 `migration`。网关复验主体、根目录、活动状态、配置和选项；私有账本绑定作用域及根目录，跨进程锁序列化维护。读目录不会自动启动迁移；删除或维护后失效对应快照。

## 保留边界与零删除

`crates/vw-agent/src/session/store.rs` 持有识矩自主代理会话索引和消息；`handlers/session.rs`、`client/session_api.rs` 提供其 CRUD、流消息和UI快照接口。这些记录与外部原生历史的身份、存储和生命周期不同，不能为了单一所有者强合。

`crates/vw-agent-apps/src/lib.rs` 明确 `upstream` 只作来源参考、不在正式模块树或构建内。来源 `codex_history_migration.rs` 和 TS 会话搜索不是当前正式运行写入者；本任务不逐文件破坏冻结快照，统一归档/移除由任务21、22处理。

当前未找到本任务同权限、同来源、同职责的第二个运行所有者，因此生产代码删除集合为空，未切换调用者、未新增迁移或测试后门。前置合同 `docs/shiju/external-sessions/contracts.md` 和 `coverage.md` 的“待实现”是生成时状态；本次以现存生产调用链绑定，不能将旧状态或缺失任务卡直接作为通过/缺实现证据。

## 静态检查与后续验证

RustCodeGraph `status` 成功，但新会话目录未入图，`node --file .../sessions/catalog.rs` 返回无索引匹配，因此回退 `rg` 与源文件读取；未初始化或同步索引。已审阅上述 API、SDK、领域、桌面、来源模块边界。`rg external_sessions_catalog` 的桌面业务命中为加载和删除后重读，两者复用同一 SDK。

本任务未操作业务数据、未生成夹具或哈希，不声称租户隔离、重启读取、回滚、终端恢复经过执行验证。未新增 `cc_convergence_sessions.rs`：本次没有行为改动，按计划零删除路径保留现有边界；后续统一验证应覆盖实际公开 API 的目录去重、跨作用域拒绝、浏览/恢复不改原始事件及维护失败恢复。现有 `sessions/delete/hermes_tests.rs` 等测试未运行。

本任务仅新增本文。恢复只需撤回本文提交，不能整体重置共享工作区；无业务数据回滚步骤。准入与基线见 `准入与所有者.md`。
