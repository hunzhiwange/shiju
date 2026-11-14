# WebDAV 设置 API

GET/PUT `/v1/agent-apps/device-sync/webdav/config?projectId=…&targetHostId=…`。
沿用网关 Basic 管理认证、可信租户 Extension 和 `x-agent-apps-token` 成员认证。
管理员访问策略的对应租户/主机 grant 必须显式包含 `device_sync_config: [项目ID]`；默认拒绝，远程主机不支持。

PUT 使用 `UpdateWebDavSyncConfigRequest`，secret 必须显式选择 keep/replace/clear；不接受本地路径、租户或身份字段。配置校验先于存储操作，保存不发起网络访问。
连接复用统一数据库的租户/主机设置和宿主凭据根。

GET/PUT 返回 `WebDavSyncConfigResponse`：config（尚未配置时 null）、last_status（该完整授权目标与 WebDAV 的最近接纳任务当前状态）、active_task_id（最近未终止任务）。读取日志不触发初始化或恢复，不返回密码及密文。响应设置 no-store。
状态按项目/主体/主机隔离；连接配置沿用领域存储的租户/主机粒度。设置权限应仅授予该连接的管理员。

本轮仅代码审阅，未编译、未测试、未实机验证；后续统一验证认证拒绝、跨作用域隔离、无效配置零副作用和后台任务发现。
