# 手工上传契约

`POST /v1/agent-apps/device-sync/{webdav|s3}/upload` 返回 HTTP 202 与 `SyncTaskStatus`（包含 taskId）。JSON 使用 camelCase，字段为 target、requestId、expectedRemoteSnapshot、selection。远端前置条件必须显式指定 `{ "kind": "empty" }` 或 `{ "kind": "snapshot", "snapshot": { "snapshotId": "…", "manifestSha256": "…", "etag": "…" } }`。摘要为完整 manifest 原始字节的 SHA-256；ETag 保持服务端原值。

selection 为数据所有者的显式选择：providers、mcp、prompts、preferences、repositories、skills、profiles、profileSkillDirectories，所有字段必须提供，空列表不代表全选。上传授权允许导出所选便携字段及资源内容，部署管理员应据此发放权限。

访问策略 grant 新增 device_sync_upload 项目 ID 列表，默认拒绝；已有备份权限不隐式授予上传权限。执行主机须绑定本地、在线，并配置绝对 credential_root 与 skills_root。请求 tenant、principal 和 host 必须匹配授权。当前仅 macOS 支持执行。

同 target 下 requestId 持久幂等：传输、方向、远端前置条件、选择集或连接修订不同均返回 409。排队后重新授权，配置及加密凭据修订变化使任务以 conflict 结束。两种传输与配置保存共享租户/主机锁。任务写入现有统一库；后台执行失败记录安全错误码，不泄露 URL、凭据或远端载荷。

先核对远端内容 ID、manifest 摘要及 ETag，再执行配置、技能、manifest 的既有发布顺序。成功回读后才记录终态与基线。显式指定已存在远端表示同意替换该版本；没有隐式强制覆盖。远端固定键仍是乐观并发协议：上传器在写入前复查内容 ID，不提供跨设备原子 CAS，元数据单独变化及复查后的并发写入仍有窗口。

本轮按指令仅实现与代码审阅，未运行 Cargo、单元测试、HTTP 测试或实机验证；未领取构建槽位，CARGO_TARGET_DIR 未设置。后续统一验证须覆盖认证拒绝、幂等、连接排队变更、两传输串行、冲突与制品顺序。任务查询/取消 HTTP 入口由对应任务交付。
