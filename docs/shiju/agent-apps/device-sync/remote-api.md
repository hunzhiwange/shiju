# 已保存配置的远端预览

`GET /v1/agent-apps/device-sync/{transport}/remote?projectId=...&targetHostId=...`

transport 为 `webdav` 或 `s3`，不接受连接草稿。复用显式 `device_sync_probe: [projectId]` 网络访问授权、可信 TenantScope 和本地主机绑定；授权在配置、凭据读取和网络请求之前完成。跨租户请求不能读取其他租户配置；远程执行主机沿用现有拒绝语义，不回退桌面。

读取统一存储中当前租户、主机下的配置，在现有同步操作锁内解析凭据并完成一次有界 manifest 查询。兼容版本取自共享快照所有者。不会下载归档、变更配置或执行恢复。

成功响应使用 `DeviceSyncRemoteResponse`：

- `{"status":"empty"}`：仅 manifest 的 HTTP 404 表示空远端。
- `status: "snapshot"`：包含 `device_name`、`created_at`、`snapshot_id`、`protocol_version`、`data_compat_version` 和 `compatibility`。后者为 `compatible`、`unsupported_protocol` 或 `unsupported_data_version`，不兼容元数据仍可展示。

所有响应 no-store。不包含远端地址、manifest 路径、ETag、凭据及远端错误正文。错误体为固定代码：未配置 404 `not_configured`，禁用 409 `disabled`，配置或密钥不可用 503 `connection_unavailable`，远端传输/鉴权失败 502 `remote_unavailable`，无效 manifest 502 `invalid_manifest`。授权及参数拒绝沿用网关状态码，不将远端鉴权失败误报为空。

本轮仅实现并审阅代码，依用户要求未运行 Cargo、编译、单元测试或实机测试，未领取槽位或设置 CARGO_TARGET_DIR。后续统一验证两种传输的 empty/兼容/不兼容/无效 manifest、远端失败、禁用和未配置、主机与租户拒绝及响应脱敏。
