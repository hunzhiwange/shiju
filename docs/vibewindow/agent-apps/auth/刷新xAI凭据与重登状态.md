# 刷新 xAI 凭据与重登状态

母任务 6／批次 20。直接前置见[完成 xAI 设备授权](完成xAI设备授权.md)。协议对照冻结的 cc-switch 4.0.25（`ca58c031`）中 `proxy/providers/xai_oauth_auth.rs` 的刷新路径与 `src/components/providers/forms/hooks/useManagedAuth.ts` 的重登状态读取。

`vw-agent-apps/src/auth/xai_refresh.rs` 提供执行主机内部的 `XaiRefresh::access_token`。`vw-agent/src/agent_apps/auth_accounts.rs` 的 `AuthAccountStore::xai_access_token` 先检查所属租户、成员、主机和服务的托管账号记录，再调用刷新适配器。刷新复用设备授权的可信发现、禁用重定向的 HTTP 客户端和账号私有文件租约。剩余有效期在 60 秒以内时刷新，成功后原子保存新的访问令牌、到期时间及上游可选轮换的刷新令牌。

`invalid_grant`、`invalid_token`、令牌端点 HTTP 401／403 或完整收取但无法解析的 HTTP 400 会把该账号的 `requires_reauth` 写入原有私有凭据文件。旧格式缺少此字段时按 `false` 读取。每次解析均在检查有效期前读取此字段，拒绝继续使用旧访问令牌或发送刷新请求。同一账号重新完成设备授权后写入新的就绪凭据。传输故障、限流及服务端错误保留凭据供后续重试。刷新状态由执行主机私有文件持有，账号元数据仍使用既有 SQLite／PostgreSQL 适配器。账号状态 API 归批次 38，真实 xAI 账号验收归批次 59。

## 验证

macOS 27.0 arm64。红灯接线返回 `Unsupported`。有效刷新预期返回新访问令牌，实际返回 `Unsupported`。网络断开预期返回可重试的 `UpstreamFailure`，实际返回 `Unsupported`。两项均在行为断言处失败。

回环 HTTP 夹具校验发现与刷新表单、成功轮换、连接断开、503 及 `invalid_grant`。真实临时凭据文件验证同账号持有租约时无法取得第二份租约，重新打开后仍拒绝已标记账号，其他账号保持可用。主机入口验证跨作用域拒绝、未到期的旧令牌同样被重登标记拦截，以及同一账号设备重登后恢复。

红灯和绿灯均动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。格式化在源码快照运行，仅回写本批 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯和绿灯） | 0 / 0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_xai_refresh -- --nocapture`（红灯） | 101 | 0 通过、2 失败、747 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_xai_refresh -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、747 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_xai_device -- --nocapture` | 0 | 2 通过、0 失败、747 过滤 |
| `cargo test -p vw-agent --lib auth6_xai_device_store -- --nocapture`（首次） | 101 | SQLite 通过，PostgreSQL 初始化失败，11747 过滤 |
| `cargo test -p vw-agent --lib auth6_xai_device_store -- --nocapture`（复验） | 0 | SQLite、PostgreSQL 各 1 通过、0 失败、11747 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings`（共享工作区） | 101 | 其他桌面测试改动触发 E0502 |
| `cargo clippy --all-targets --all-features -- -D warnings`（隔离源码快照） | 0 | 通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |

主机测试使用真实 SQLite 临时文件与本机 Lima 中的 PostgreSQL 18.6 临时实例，每次 PostgreSQL 测试使用独立 schema。首次 PostgreSQL 初始化要求显式 TLS 模式，复验为回环测试库指定 `sslmode=disable`。主机测试链接出现既有 macOS `__eh_frame` 大小警告。

共享工作区的 Clippy 在 `system_settings_general_tests.rs:793` 遇到其他未提交桌面改动的借用错误。以 `463104f9d` 和本批 Rust 文件组成隔离源码快照，复跑相同命令通过。各验证阶段均已释放本任务的槽位锁，保留编译缓存。临时 PostgreSQL 已停止并清理。
