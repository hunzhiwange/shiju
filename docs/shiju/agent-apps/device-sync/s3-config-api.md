# S3 设置 API

GET/PUT `/v1/agent-apps/device-sync/s3/config` 复用设备同步认证链。
查询参数为 `projectId`、`targetHostId`；租户来自认证中间件，项目成员及本机执行权限由访问策略校验。
配置沿用租户与主机隔离的 S3SettingsStore，状态使用完整授权目标并按 S3 过滤。

PUT 使用 UpdateS3SyncConfigRequest，凭据必须显式选择 keep、replace 或 clear。
Keep 只在同一连接身份保留已有凭据；身份改变则清除。启用配置必须具备完整元数据和凭据，Clear 仅可与禁用配置一起保存。
非法元数据、空替换值和无效启用组合返回 422；存储失败返回 503，不返回内部错误或秘密。
响应为 S3SyncConfigResponse：config、last_status、active_task_id，凭据仅通过 secret_present 表示，所有响应设置 Cache-Control: no-store。

实现已进行源码差异审阅；按本轮指令未运行 Cargo、单元测试、HTTP fixture 或实机验证，编译和行为均未验证，需统一回归。
