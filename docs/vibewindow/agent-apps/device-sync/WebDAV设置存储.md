# WebDAV 设置存储合同

任务18实现 `device_sync::webdav_settings::WebDavSettingsStore`。仅保存与读取连接，不注册网络路由或执行同步。

- 调用者必须从授权上下文提供 TenantScope、执行 host 与既有主机 SecretStore 根目录；不得采用请求自选身份或路径。存储使用统一 GatewaySettingsStore，命名空间 `device_sync.webdav.local.v1`，键为 host 与固定 default 槽，租户隔离由统一库提供。
- 写入复用 WebDavConfig::normalize；读取拒绝非规范化或不一致记录，不返回未校验 URL。全部配置属于设备数据，不进入可迁移快照。
- 密码通过既有 security::SecretStore 的 enc2 加密机制管理；不复用仅支持官方登录服务的 AccountCredential 枚举，不创建第二数据库。读模型只含 secret_present，内部记录无 Debug，错误不含输入及底层载荷。
- Keep 仅在规范化 base_url、username、remote_root、profile 全相同时保留密文；首次保存或身份变化时置空。Replace 保留非空密码原文，空串明确拒绝并提示使用 Clear；Clear 删除密文。enabled 不强制密码存在，后续连接验证负责认证要求。
- 保存持有既有租户/host 操作锁；调用者须与同步执行统一使用 JSON 编码的 `(platform_id, company_id)` 作为锁租户键。数据库通过 compare_exchange/insert_if_absent 原子发布配置和密文，跨进程竞争返回冲突。加密、序列化、数据库写入失败不会改变旧记录；首次加密可能创建主机密钥，数据库失败不删除共享密钥。

依赖审阅：配置 DTO 与规范化入口已交付；统一存储及 SecretStore 已存在。快照/恢复不参与本保存入口，不以其缺口伪造同步闭环。

按用户最新指令，本轮仅人工源码及差异审阅，未运行 Cargo、脚本验证、单元测试、实机测试或最终回归；未领取构建槽位，无 CARGO_TARGET_DIR。后续统一验证须覆盖 SQLite/PostgreSQL 隔离与失败回滚、Keep/Replace/Clear、身份变化、并发冲突、读取脱敏和操作锁互斥。当前未验证编译及行为。
