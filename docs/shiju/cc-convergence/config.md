# 应用配置唯一事务路径

2026-11-14，任务5。按本轮用户要求仅静态核对，不运行 Cargo、构建、单元测试、实机测试或最终回归。任务卡删除表示本轮实现收尾，不表示行为验收通过。

## 所有者与保留边界

- 应用原生文件及对应模式状态由 `crates/vw-agent-apps/src/providers/live/commit.rs::ProviderTransactions` 持有。`commit_checked` 汇入 `commit_with_record`；提交检查租户、主机、应用、项目及 pending 所有权，持有应用锁，先恢复再要求调用者重算，暂存后重新验证输入，保存持久发布意图。恢复位于同目录 `recovery.rs`，不另建第二回滚机制。
- 项目绑定由 `live/project_binding.rs::commit_project_checked` 校验预像后调用同一 `commit_checked`。供应商路径初始化集中在 `crates/vw-agent/src/gateway/api/handlers/app_providers/paths.rs`，各应用适配器消费该事务。聚合发布/退出通过 `gateway/aggregation_resync/` 复用相同事务；提示词投影由 `agent_apps/prompt_activation_store.rs`、`prompt_deactivation_store.rs`、`prompt_sync_store.rs` 等消费相同所有者。不同字段投影不是重复事务所有者。
- `crates/vw-agent/src/gateway/api/handlers/config.rs::apply_config_patch` 分流租户全局配置与显式项目配置，调用 `update_global_for_tenant` / `update_project_for_tenant`。有状态处理器只在更新活动配置后刷新内存及配对状态。这是 ShiJu 网关配置，不是客户端原生配置的副本，保留。
- `crates/vw-agent/src/gateway/settings_store.rs::GatewaySettingsStore` 保存按租户、namespace、key 隔离的平台记录。`update_with_preimages` 在数据库事务内检查引用预像；SQLite 与 PostgreSQL 后端保留各自实现。它不替代主机文件事务，也不应被删除。
- `gateway/api/handlers/app_routing/config_api.rs` 经 `RoutingControl::authorize` 校验主机/项目与读写权限；监听设置使用 `apply_listener`，应用超时使用 `save_app_timeouts`。监听生命周期与原生客户端投影职责不同，保留。
- SDK `crates/vw-gateway-client/src/client/config_api.rs` 和 `app_routing/config_api.rs` 连接对应 HTTP 所有者，不拥有主机文件写入。Iced 入口的整体交接索引见 `docs/shiju/desktop-integration/acceptance-1.md`；本轮不扩大为桌面行为验收。

## 零删除决定

正式执行链已使用现有事务；计划列出的 `config.rs` / `settings_store.rs` 不能仅因名称相似删除。`crates/vw-agent-apps/upstream/src-tauri/` 的配置服务、设置DAO及来源 TS hooks 是来源参考，不属于正式 `src/lib.rs` 模块树，整体归档及来源移除留给任务21、22。本任务不改变 API/SDK 契约，不新增迁移、不双写、不触碰真实用户配置。

本任务删除生产符号/文件集合为空，也未新增测试。正式调用链没有发现需切换的同职责副本，因此不为制造差异改动运行代码。

## 静态核对与以后验证

使用 RustCodeGraph `status`、`explore`、指定文件 `node`，并用 `rg` 核对事务及调用点。已读取上述提交/项目预像及网关配置分流代码。图的同名方法关联可能跨来源树，不能将其作为真实模块引用证明，结论以实际模块及类型路径为界。

后续统一验证应覆盖：同一应用仅提交一次；租户/项目/主机越界在落盘前拒绝；暂存失败、发布后恢复以及外部修改预像冲突；网关设置不发生客户端双写。现有验证入口为 `providers/live/{commit_tests,recovery_tests,concurrency_tests,project_binding_tests}.rs`、网关 `config_tests.rs` 与 `app_routing/config_api_tests.rs`，本轮均未执行，未生成数据夹具或哈希对照，不宣称任何有效测试数或运行通过。

无运行行为变更，恢复只需逆转本证据提交；来源哈希和通用恢复约束沿用 `准入与所有者.md`，不覆盖其他工作区改动。
