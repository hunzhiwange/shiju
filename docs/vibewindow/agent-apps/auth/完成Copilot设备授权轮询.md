# 完成 Copilot 设备授权轮询

母任务 6／子任务 12。批次 1–11 已交付，复用[启动挑战](启动Copilot设备授权.md)、[轮询节流](流程到期与轮询节流.md)、[受管元数据](持久化作用域账号元数据.md)和[执行主机凭据文件](执行主机凭据文件存储.md)。按维护者更新的开工条件核对实际依赖后继续。供应商实机验收剩余项目独立记录在[供应商验收报告](../../../shiju/agent-apps/provider-macos-acceptance.md)。行为对照冻结的 cc-switch 4.0.25（`ca58c031`）的 `commands/copilot.rs` 与 `proxy/providers/copilot_auth.rs`。

- 执行主机通过 `AuthAccountStore::poll_copilot` 推进原流程。兑换使用启动时绑定的 HTTPS authority。公开 GitHub 的身份与订阅校验请求只发往 `api.github.com`。Enterprise 的身份请求只发往原 authority 的 `/api/v3/user`，沿用来源的 OAuth 令牌策略。请求禁止重定向，错误只返回安全状态码，账号和流程响应只携带脱敏字段。
- `authorization_pending` 保持等待，`slow_down` 经流程状态机延长间隔，`expired_token` 转到期，`access_denied` 转授权拒绝。取消和到期后的成功响应失去提交资格。进入终态后清除执行主机持有的设备挑战。
- 受管账号 ID 由完整作用域及已验证的 GitHub authority、数字用户 ID 派生。同一作用域内重复登录复用账号 ID 并替换 OAuth 令牌，原始注册元数据保留。不同 authority 的同一数字 ID 分属不同账号。元数据记录身份摘要、首次认证时间、GitHub 域和登录名，OAuth 令牌留在主机私有文件。批准快照的认证时间对应本次登录。登录过程无默认账号写入。
- 凭据写入沿用原子发布和独占租约。新增账号先保存私有凭据，再登记元数据。已确认的元数据失败清理本次写入的凭据。如果数据库写入和读回均不可用，保留私有文件避免删掉可能已提交的凭据。重试按同一账号和租约重新核对。运行时只有凭据及元数据提交成功才返回批准。
- 公开域的短期会话令牌仅用于订阅验证。账号专属会话令牌的读取和缓存归批次 13，HTTP 路由及真实 GitHub 登录另行验收。

## macOS 验证

macOS 27.0（26A428），arm64。回环 HTTP 夹具校验实际表单编码、GitHub 与 Enterprise 请求地址及认证头。测试用单调时钟推进等待与降速，验证拒绝、上游到期、无效响应、跨主机访问和取消后的迟到响应。用真实私有临时目录回读 OAuth 令牌，检查同一身份重复提交、域与主体隔离、流程序列化脱敏及终态重复查询。

红灯在两个行为断言失败。第一次轮询本应返回等待，实际返回 `Unsupported`。跨主机访问本应返回 `NotFound`，实际返回 `Unsupported`。

红灯、最终格式化和验证动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。一次中途格式化使用槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与相应目标目录一致。格式化在源码快照执行，仅回写本任务的 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_copilot_poll -- --nocapture`（红灯） | 101 | 0 通过、2 失败、731 过滤 |
| `cargo fmt --all` | 0 | 最终格式化成功 |
| `cargo test -p vw-agent-apps --lib auth6_copilot_poll -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、731 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_copilot_start -- --nocapture` | 0 | 2 通过、731 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_file_store -- --nocapture` | 0 | 2 通过、731 过滤 |
| `cargo test -p vw-agent-apps --lib cc_apps_account_launch_isolation -- --nocapture` | 0 | 12 通过、721 过滤 |
| `cargo test -p vw-agent-apps --lib cc_apps_credential_lease -- --nocapture` | 0 | 1 通过、732 过滤 |
| `cargo test -p vw-agent --lib auth6_copilot_store -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 1 项通过，0 失败、11734 过滤 |
| `cargo test -p vw-agent --lib auth6_store -- --nocapture` | 0 | 两库各 2 项通过，11733 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |

存储测试用 SQLite 临时文件和隔离 Lima 内的 PostgreSQL 18.6。两库均从 Tokio 运行时调用账号提交并重新打开适配器，回读域、登录名和身份摘要，验证令牌轮换、域冲突保护、跨作用域不可见及默认记录为空。SQLite 使用真实触发器拒绝元数据插入，检查无账号记录、凭据文件已清除，解除故障后重试成功。测试链接报告既有 macOS `__eh_frame` 大小警告，测试与 Clippy 均通过。PostgreSQL 测试 schema 已清理，临时实例已停止。各阶段本任务的槽位锁均已释放，缓存保留。
