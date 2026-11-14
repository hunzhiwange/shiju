# 提示词条目持久化契约

来源：cc-switch `src-tauri/src/database/dao/prompts.rs` 的条目 CRUD；本实现复用母任务提供的 GatewaySettingsStore，不迁移旧数据库。

入口：`vw_agent::agent_apps::prompts_store::PromptsStore`，提供 list/get/create/update/delete。作用域由已授权执行端提供 ProjectScope、host 和 AppId；租户由数据库平台与公司键隔离，项目、主机和应用使用 JSON 元组编码。调用者仍须完成资源授权，保存身份不是授权凭据。

PromptEntry 保存 id、name、content、description；正文和中文原样序列化，不含激活字段，不写 live 文件。Claude Desktop 明确拒绝。列表保留创建顺序，更新保留位置；后续排序任务在此基础上扩展。

每次写入携带 list 返回的 catalog revision；不存在的目录 revision 为 0。新增拒绝重复 ID，更新与删除拒绝缺失 ID。SQLite/PostgreSQL 共用 insert_if_absent/compare_exchange 防止并发覆盖；冲突要求调用者重新读取。删除最后一项仍保留递增 revision，防止旧请求复活目录。损坏载荷明确失败，不重置为空，不输出正文。

API、SDK、UI 接线由后续任务交付。本轮按用户要求仅实现代码，未运行 Cargo、单元测试、其它测试或 macOS 实测；无验证通过声明。后续统一验证需覆盖双后端中文 CRUD、跨租户/项目/主机/应用隔离、并发冲突及损坏记录拒绝。
