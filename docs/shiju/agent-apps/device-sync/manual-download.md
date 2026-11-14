# 手工下载 API（任务 47，未验证）

`POST /v1/agent-apps/device-sync/{webdav|s3}/download` 使用 camelCase JSON：
`target`、`requestId`、`selection`、`expectedRemoteSnapshot`（snapshotId、manifestSha256、etag）、
`expectedLocalSnapshot`（本机归档内容标识）、`connectionRevision`（已保存连接版本）。

需要独立的项目授权 `device_sync_download`，不会继承上传授权。
执行主机须配置绝对 credential_root、skills_root 和 data_backup_root。
返回 202 和既有 SyncTaskStatus；轮询任务接口获取结果。
相同 requestId 与不同预览、选择或连接版本冲突；执行锁内重新授权并校验连接、本机和远端版本。

两传输均先校验 manifest 兼容性、两个归档摘要、技能目录与声明，拒绝归档夹带内容。
技能必须在本机已有安装身份，使用本机保存的投影绑定，不接受远端路径。
批量准备共享物理根目录锁，冲突直接失败；完整前像包括配置原值、技能安装状态、资源内容与链接。
发布前重新检查远端完整 manifest/ETag、授权根、连接版本和本机原始记录版本。
先持久化保护快照，再发布技能，通过同一配置事务提交导入与安装元数据；失败则回滚技能并保留恢复材料。
取消仅在提交边界前接受；回滚失败标记 committed，不能报告成功。

提交后在同一任务内等待 provider、MCP、prompt 所有者的原生刷新；技能保存绑定随资源提交发布。
刷新读取当前保存状态并使用各领域现有授权，失败保留已提交结果并添加 ProjectionFailed 警告。
不自动激活未选中的供应商或提示词，不扩大 credential 使用权限。非 macOS 返回 501。

本轮只做源码审阅，未运行 Cargo、单元测试、真实 HTTP 或实机验证，无 Cargo 槽位及缓存路径。
后续统一验证两传输正常下载、过期预览、协议不兼容、HTTP 中断与原数据不变、多技能锁、
保护快照、回滚失败、原生投影、重启幂等性及取消边界。
