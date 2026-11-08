# 吸收 Claude 外部刷新并更新登录状态

母任务 6／批次 27。前置为[恢复 Claude 中断快照事务](恢复Claude中断快照事务.md)（`d50fe944f`）。对照 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/claude_account/snapshot/{mod,creds,switch}.rs`、`services/subscription.rs` 和 `commands/claude_account.rs`。

`auth/claude_rotation.rs` 提供主机内的 `get_claude_login_switch`。它在供应商目录锁和 Claude CLI 锁内从同一份默认槽位快照读取身份与刷新令牌到期时间，兼容秒和毫秒。刷新令牌已过期时返回需重新登录，轮换后的快照在下次查询时直接生效。配置目录被覆盖时返回明确的覆盖状态，未结算事务或其它租户的资源返回冲突。查询合同在主机内提供，脱敏账号查询网关由批次 38 接入。

切换继续使用 `providers/claude_login.rs` 的现有归档路径。主机账号存储持有目标账号租约后，归档在 Claude CLI 锁内重读目标身份和凭据，拒绝身份冲突及已知的旧凭据代际。对没有代际字段的旧目标副本，仍允许以锁内读出的同身份默认凭据为准。钥匙串写入并回读后才清理后备文件，保留同身份归档目录的资料及共享配置。

## 验证

macOS 27.0（26A428），arm64。独立子进程按 Claude CLI 锁协议更新测试命名空间的真实钥匙串。父进程观察到同一身份从需重登变为可用，供应商事务提交后核对默认身份、旧账号归档和凭据。拒绝覆盖身份变化的目录、同身份的较新代际、组织冲突和越租户读取，并验证自定义目录的阻止状态及钥匙串清理。真实 Claude 服务登录归批次 61。

红灯首次运行使用槽位 2（`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`），测试在私有目录权限检查处中止。修正夹具后，红灯预期的初始到期状态实际为 `Err(InvalidResource)`，跨身份归档预期 `Err(Conflict)`，实际为 `Ok(())`。

有效红灯和绿灯使用槽位 1。两次的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。格式化在源码快照中执行，仅回写本批 Rust 文件。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0、0 | 通过 |
| 红灯 | `cargo test -p vw-agent-apps --lib auth6_claude_rotation -- --nocapture` | 101 | 0 通过、2 失败、766 过滤 |
| 绿灯 | `cargo test -p vw-agent-apps --lib auth6_claude_rotation -- --nocapture` | 0 | 2 通过、0 失败、766 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_claude_switch -- --nocapture` | 0 | 2 通过、0 失败、766 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_claude_recover -- --nocapture` | 0 | 2 通过、0 失败、766 过滤 |
| 工作区静态检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0、0 | 通过 |

各阶段已释放本批槽位锁并保留编译缓存。
