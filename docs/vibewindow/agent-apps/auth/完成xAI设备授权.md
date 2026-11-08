# 完成 xAI 设备授权

母任务 6／批次 19。批次 1–18 的合同和执行主机适配已交付，直接前置见[吸收 Codex 外部进程令牌轮换](吸收Codex外部进程令牌轮换.md)。协议对照 cc-switch 4.0.25（`ca58c031`）的 `proxy/providers/xai_oauth_auth.rs` 与 `commands/auth.rs`，并核对 xAI 的 [OpenID 发现文档](https://auth.x.ai/.well-known/openid-configuration)。

`vw-agent-apps/src/auth/xai.rs` 的 `XaiDeviceAuth::start` 和 `poll` 复用主机授权状态机。发现文档、设备请求和令牌交换均由执行主机发起。发现响应的发行方与 HTTPS 端点固定在 `auth.x.ai`，HTTP 客户端禁用重定向。公开挑战仅包含不透明流程 ID、浏览器验证链接、用户码和时间信息。浏览器链接只接受 `auth.x.ai` 或 `accounts.x.ai` 的 HTTPS 地址。

`authorization_pending` 保持待授权，`slow_down` 将间隔增加 5 秒并封顶于 63 秒，`expired_token` 进入过期，`access_denied` 返回脱敏的拒绝。流程按租户、成员、执行主机和服务隔离，取消或过期后到达的令牌响应无法提交。

`vw-agent/src/agent_apps/auth_accounts.rs` 的 `AuthAccountStore::poll_xai_device` 使用同作用域租约与既有 OAuth 发布事务。账号 ID 由作用域和受信令牌响应中的稳定主体派生，账号库只保留脱敏元数据。访问令牌、刷新令牌和访问令牌到期时间使用 `CredentialFileStore::lease_xai` 写入执行主机的私有文件。同主体重新完成设备流程可更新已有凭据，旧代及不同主体无法覆盖已有账号。定向重登和浏览器登录按 DTO 合同拒绝。令牌刷新归批次 20，默认账号归批次 34，网关接线归批次 39–41，真实账号登录归批次 59。

## 验证

红灯接线返回 `Unsupported`。有效发现和设备挑战预期成功，实际返回 `Unsupported`。不可信令牌端点预期返回 `UpstreamFailure`，实际也返回 `Unsupported`。两项均在行为断言失败。

绿灯使用回环 HTTP、受控单调时间和真实私有文件，核对固定请求、待授权、降频、过期、拒绝、迟到响应及脱敏结果。首次绿灯运行发现临时目录权限不满足凭据存储要求，夹具显式设为 `0700` 后两项通过。

macOS 27.0 arm64。槽位由原子目录锁动态领取。`CARGO_BUILD_BUILD_DIR` 与各阶段的 `CARGO_TARGET_DIR` 相同。格式化在源码快照执行，仅回写本批 Rust 文件。

| 命令 | 槽位 | 退出码 | 结果 |
| --- | ---: | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_xai_device -- --nocapture`（红灯） | 2 | 101 | 0 通过、2 失败、745 过滤 |
| `cargo fmt --all` | 2、3 | 0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_xai_device -- --nocapture`（修正夹具后） | 3 | 0 | 2 通过、0 失败、745 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_copilot -- --nocapture` | 1 | 0 | 6 通过、0 失败、741 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_expiry -- --nocapture` | 1 | 0 | 2 通过、0 失败、745 过滤 |
| `cargo test -p vw-agent --lib auth6_xai_device_store -- --nocapture` | 2 | 0 | SQLite、PostgreSQL 各 1 通过，0 失败，11747 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 2 | 0 | 通过 |
| `git diff --check`、`git diff --cached --check` | — | 0 / 0 | 通过 |

实际 `CARGO_TARGET_DIR` 如下。

- 槽位 1 使用 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 槽位 2 使用 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- 槽位 3 使用 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。

数据库测试重开真实 SQLite 文件，并在本机 Lima PostgreSQL 实例中使用独立 schema。两库均验证了元数据脱敏、私有凭据读取、同主体更新以及旧代和跨主体拒绝。宿主测试链接出现既有 macOS `__eh_frame` 大小警告。PostgreSQL 实例已停止，本任务的槽位锁均已释放，编译缓存保留。
