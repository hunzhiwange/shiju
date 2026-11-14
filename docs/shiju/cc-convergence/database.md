# 数据库与全局状态工厂

2026-11-14，任务11静态核对。按本轮用户要求不运行任何 Cargo、构建、单元测试、实机或最终回归。以下是源码边界证据，不是数据库行为验收结果。

## 权威链路与保留边界

- `crates/vw-agent/src/global_database.rs`：`GlobalDatabase::for_home` → `for_home_with_postgres_settings`，以绝对配置路径查 `BACKENDS_BY_CONFIG_PATH`，复用 `Arc<GlobalDatabaseBackend>`；仅首次按 `DatabaseProvider` 显式选择 SQLite/PostgreSQL，加载失败返回错误，不自动换库。SQLite 旧路径冲突明确拒绝，保留既有迁移职责。
- `crates/vw-agent/src/gateway/settings_store.rs`：`GatewaySettingsStore::new(database)` 包装注入的数据库；SQLite 初始化同库表，PostgreSQL 委托对应适配器。`gateway_settings` 主键为 `(platform_id, company_id, namespace, config_key)`，`get/upsert` 使用请求租户。领域设置仓库不是第二个独立数据库工厂。
- `crates/vw-agent/src/gateway/api/handlers/desktop.rs::desktop_settings_store` 调用 `GlobalDatabase::for_home(global::paths().home)` 后构造 `GatewaySettingsStore`；SDK/Iced 通过网关访问设置，不接管该后端注册表。
- `crates/vw-agent/src/agent_apps/store.rs::IconLibraryStore::new` 克隆同一 `GlobalDatabase` 构造设置仓库，固定 `agent_apps.custom_icons/default`；图标变更按提供者委托 `mutate_sqlite/mutate_postgres`。领域纯规则来自 `vw_agent_apps::icons::library`，不启动旧来源数据库。
- `crates/vw-agent/src/agent_scheduler/store/mod.rs` 的 `AgentSchedulerStore` 保存数据库引用/配置上下文；`initialize_provider` 调用 `store.database()` 后显式选择同一提供者。项目作用域与父项目存在性检查保留，不将调度表和租户设置表强合。
- `crates/vw-agent/src/gateway/state.rs::AppState` 持有配置、模型、事件和会话引擎共享引用，职责是运行时装配，不等价于来源 Tauri `AppState` 或持久化设置仓库，不应仅因同名删除。

来源 `crates/vw-agent-apps/upstream/src-tauri/src/{database,store.rs}` 仅属参考快照；正式 `crates/vw-agent-apps/src/lib.rs` 的模块树不包含 upstream，明确声明其不参与正式库构建。整树归档/移除由任务21、22处理。

## 清理结果与恢复

本域最小删除集合为空：上述路径共用正式数据库门面或承担不同状态/作用域职责，未发现已切走调用者的同职责运行副本。未新增抽象、业务接口、测试后门或数据迁移；不把多个领域仓库、SQLite连接或不同配置路径的后端视为重复所有者。没有生产代码改动，不需要数据切换/回滚，不读取真实用户库，不声称旧库数据条数、哈希或重启一致性已验证。

前置索引为 [准入与所有者](准入与所有者.md)，存储设计合同为 `docs/vibewindow/agent-apps/存储与生命周期.md`。参考基线 `699c7a9bfb382633fdd3b9962a23bbbdf425472c`；如需撤销本任务只撤销本文，不回滚其它任务或工作区改动。

## 静态检查及待验证

RustCodeGraph `status`、`explore`、上述文件 `node`、`callers for_home --limit 10` 均退出0。后者包含同名调度工厂，已结合文件源码区分，不能把名称匹配全部当作精确调用边。不存在的 `agent_apps/{custom_icons,icons}.rs` 查询已转到实际 `agent_apps/store.rs`，未凭空创建实现。

已有验证入口：`global_database_tests.rs`、`global_database/{config_tests,postgres_tests}.rs`、`gateway/settings_store_tests.rs`、`gateway/settings_store/postgres_tests.rs`、`agent_apps/{store_tests,sqlite_tests,postgres_tests}.rs`。本轮全部未运行，未领取 Cargo 槽位。以后统一验证配置路径后端复用、双提供者租户隔离、失败回滚及重启读取；不将本静态检查计为行为测试通过。
