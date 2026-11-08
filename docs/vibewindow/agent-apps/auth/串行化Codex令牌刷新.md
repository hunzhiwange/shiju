# 串行化 Codex 令牌刷新

母任务 6／批次 17。前置为批次 1–16 的授权合同、主机存储和回归证据，最近一批为 [Codex 定向重新授权](校验Codex定向重新授权.md)（`a29721a66`）。刷新请求及永久拒绝规则对照 cc-switch 4.0.25（`ca58c031`）的 `proxy/providers/codex_oauth_auth.rs`。

主机入口 `AuthAccountStore::codex_access_token` 先读取受管账号元数据，再由共享的 `CodexRefresh` 按租户、主体、执行主机与账号 ID 串行处理。等待者进入后复读同一账号的凭据文件。OS 租约覆盖读取、上游兑换和原子保存，其他受管写入路径遇到占用会失败。刷新返回前复查文件代际，workspace 或 ID token 主体冲突时保留原凭据。上游省略 refresh token 或 ID token 时保留对应旧值。

进程内按文件内容摘要记录有效期或永久拒绝。永久拒绝返回 `reauth_required`，暂时性上游故障可重试。重新授权替换凭据后，旧代际的缓存自动失效。刷新只写主机文件，元数据继续记录原授权时间。正式账号实测归批次 58，CLI 外部轮换归批次 18。

## 验证

红灯运行两项，均到达行为断言。合法刷新实际返回 `Some(Unsupported)`，预期无错误。跨主机读取实际返回 `Unsupported`，预期 `StorageUnavailable`。

绿灯使用临时私有凭据文件和回环 HTTP。服务器阻塞 alpha 的响应，直到 beta 完成刷新。两个 alpha 调用都取得同一访问令牌，服务器只收到一次 alpha 请求。落盘核对两账号的代际、可选令牌保留和其他主体隔离。拒绝用例覆盖非法 ID、跨主机读取、短暂故障、主体冲突、过期响应、永久拒绝和重登后解除旧拒绝。

在 macOS arm64 上使用槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，执行格式化、红灯、绿灯及双库回归。Clippy 使用槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。格式化在源码快照中执行，只回写本任务 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo fmt --all` | 0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_codex_refresh -- --nocapture`（红灯） | 101 | 0 通过、2 失败、741 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_codex_refresh -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、741 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_codex -- --nocapture` | 0 | 8 通过、0 失败、735 过滤 |
| `cargo test -p vw-agent --lib auth6_codex -- --nocapture` | 0 | SQLite 5 项、PostgreSQL 3 项通过，0 失败、11739 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |

双库回归验证主机入口依赖同作用域的元数据，刷新只更新私有凭据。测试链接出现既有 macOS `__eh_frame` 大小警告。临时 PostgreSQL 实例已停止，两个槽位锁已释放，编译缓存保留。
