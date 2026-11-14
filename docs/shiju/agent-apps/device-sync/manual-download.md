# 手工下载 API（任务 47，未验证）

`POST /v1/agent-apps/device-sync/{webdav|s3}/download` 使用 camelCase JSON：
`target`、`requestId`、`selection`、`expectedRemoteSnapshot`（snapshotId、manifestSha256、etag）、
`expectedLocalSnapshot`（本机归档内容标识）、`connectionRevision`（已保存连接版本）。

需要独立的项目授权 `device_sync_download`，不会继承上传授权。
执行主机须配置绝对 credential_root、skills_root 和 data_backup_root。
返回 202 和既有 SyncTaskStatus；轮询任务接口获取结果。
相同 requestId 与不同预览、选择或连接版本冲突；任务进入执行锁后重新授权并校验连接、本机和远端版本。
远端兼容性或摘要不符时不应用。下载器完成归档校验和远端复查后，先持久化本机完整配置前像，再经配置所有者的事务检查原值并应用。

当前 API 仅接通配置下载：非空技能选择返回 501；远端携带技能也会在发布前拒绝。
技能资源的完整前像、保存绑定与安装状态事务尚未接通，不可视为完整任务验收。
配置导入不主动发布 native 文件；涉及 provider/MCP/prompt 时返回 ProjectionFailed 警告，需后续所有者重协调。
非 macOS 返回 501。尚未运行编译、单元测试、真实 HTTP 或实机验证，无 Cargo 槽位及缓存路径。

待统一验证：两传输正常下载、过期预览、协议不兼容、HTTP 中断与原数据不变、回退材料、重启幂等性及取消边界。
