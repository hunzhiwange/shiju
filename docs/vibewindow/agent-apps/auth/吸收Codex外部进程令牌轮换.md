# 吸收 Codex 外部进程令牌轮换

母任务 6／批次 18。批次 1–17 的授权合同、主机凭据及刷新适配均已交付，直接前置为[串行化 Codex 令牌刷新](串行化Codex令牌刷新.md)（`d0b0efbff`）。轮换身份和代际规则对照 cc-switch 4.0.25（`ca58c031`）的 `read_managed_live_auth_refresh_for_account` 和 `adopt_account_refresh_token`。

`AuthAccountLaunch` 复用已授权账号的凭据租约。CLI 退出后，`AccountLaunch` 从该次启动的私有 `CODEX_HOME/auth.json` 回读凭据，比较启动时和回读时的 workspace 与 ID token 主体。可用的 workspace 声明必须和文件的 workspace 一致。令牌材料有变化时，两个代际都须有有效时间，且回读时间在毫秒精度上严格较新。相同材料可以补齐时间。CLI 退出码非零时仍处理有效轮换，并清理临时目录。

CLI 写回和 OAuth 刷新共用 `CredentialLease::save_codex_generation`，在原有私有文件暂存和原子发布路径中复读并比较初始凭据。旧 OAuth 响应被丢弃，已写入的凭据保留。临时 CLI 目录由原有启动边界创建，主机默认 `.codex` 仍由既有供应商事务管理。账号绑定归批次 33，真实服务登录归批次 58。

## 验证

红灯使用旧的 workspace 检查及写回路径。同 workspace 的另一用户被错误接受，实际没有错误，预期为 `rotated official account identity changed`。

绿灯通过两个独立行为测试。真实子进程将轮换凭据写入临时 CLI 文件，验证同账号写回、其他主体隔离和临时目录清理。拒绝场景覆盖主体冲突、workspace 声明冲突、旧时间、等时间、缺时间和已被替换的启动代际。受控回环 HTTP 等待外部进程写入新代后再释放旧刷新响应，确认返回 `Conflict` 且新代仍在磁盘。

macOS arm64 使用槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。仓库另设了 Cargo 中间产物目录，本轮将 `CARGO_BUILD_BUILD_DIR` 也设为同一槽位。格式化在源码快照中执行，只回写本任务的 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo fmt --all` | 0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_codex_rotation -- --nocapture`（红灯） | 101 | 1 通过、1 失败、743 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_codex_rotation -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、743 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_codex -- --nocapture` | 0 | 10 通过、0 失败、735 过滤 |
| `cargo test -p vw-agent-apps --lib cc_apps_account_launch_isolation -- --nocapture` | 0 | 主测试进程 12 通过、0 失败、733 过滤 |
| `cargo test -p vw-agent --lib cc_apps_account_launch_isolation_owner_persists_rotation_and_holds_lease_until_reaped -- --nocapture` | 0 | 1 通过、0 失败、11746 过滤 |
| `cargo test -p vw-agent --lib auth6_codex -- --nocapture` | 0 | SQLite 5 项、PostgreSQL 3 项通过，0 失败、11739 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |

主机测试链接仍有既有 macOS `__eh_frame` 大小警告。临时 PostgreSQL 实例已停止，槽位锁已释放，编译缓存保留。
