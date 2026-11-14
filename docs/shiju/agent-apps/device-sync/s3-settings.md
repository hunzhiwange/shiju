# S3 本机连接存储合同

`device_sync::s3_settings::S3SettingsStore` 由网关传入既有 `GlobalDatabase` 与主机 SecretStore 根目录。授权层提供 `TenantScope` 和执行主机 ID；请求不能选择租户。配置使用统一 `gateway_settings`，命名空间为 `device_sync.s3.local.v1`，按租户、主机隔离，不另建数据库，不进入可迁移快照。

保存先通过 S3Config 规范化元数据，再解析显式凭据意图。Keep 仅在 bucket、region、access_key_id、endpoint、remote_root、profile 全部一致时保留旧密文；身份改变时清除绑定。Replace 拒绝空字符串，精确保留其余输入并通过已有 SecretStore 加密；Clear 删除绑定。启用配置必须具有 bucket、region、access key ID 与密文凭据，因此启用状态下清除或改变身份后未替换凭据会失败，旧配置保持不变；可同时禁用并清除。

配置与密文在同一行通过 insert-if-absent 或 compare-exchange 原子发布。保存复用租户/主机同步操作锁，跨进程陈旧写返回冲突。校验、加密、取消或数据库发布失败均不发布新配置。读取仅返回 S3SyncConfig 与 secret_present，不提供 secret access key 或密文；access_key_id 是既有读模型定义的公开账号标识。错误不回显输入、数据库载荷或底层凭据错误。

本次仅完成代码静态审阅。按用户要求未运行 Cargo、单元测试、实机测试或其它测试；未领取 Cargo 槽位，也未设置 CARGO_TARGET_DIR。后续统一验证需覆盖 Keep/Replace/Clear、规范化等价身份、身份变更、租户/主机隔离、存储冲突与失败保留旧值以及读响应不泄漏密钥。
