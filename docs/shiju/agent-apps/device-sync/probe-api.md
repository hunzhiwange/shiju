# 草稿连接验证

`POST /v1/agent-apps/device-sync/{transport}/test?projectId=...&targetHostId=...`

transport 为 `webdav` 或 `s3`。请求体分别复用 `UpdateWebDavSyncConfigRequest`、`UpdateS3SyncConfigRequest`，必须显式指定 secret 的 keep/replace/clear 意图。允许 disabled 草稿。Keep 仅复用当前租户、执行主机下相同规范化连接身份的密钥；身份改变或不存在已保存连接返回 400。

管理员访问策略 grant 需显式添加 `device_sync_probe: [projectId]`，默认拒绝。继承现有 token、租户、在线本地主机绑定检查，远程绑定返回 501；主机须配置绝对 credential_root。该权限允许向草稿目的地址发送凭据，应仅授予可信配置管理员。不回退桌面网络。

成功返回 204；草稿错误 400；网络或远端探测失败 502；授权沿用 401/403/404/501；存储不可用 503。响应 no-store，不回显地址、密钥或远端正文。WebDAV 探测可能创建远端目录，S3 使用签名 HEAD；均不保存本地草稿、不变更已保存凭据。

本轮仅实现并静态审阅，依用户要求未运行编译、单元测试、loopback 或实机测试；未领取 Cargo 槽位。后续需统一验证两种传输、Keep 身份边界、拒绝授权及草稿不落库。
