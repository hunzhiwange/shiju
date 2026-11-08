# 恢复 Claude 中断快照事务

母任务 6／批次 26。前置为[切换 Claude 凭据快照](切换Claude凭据快照.md)（`9bdddaaa5`）。来源为 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/claude_account/snapshot/{recover,state,tests}.rs`。实现沿用 `providers/claude_login.rs` 的 `LoginHost` 与现有供应商事务，启动入口在网关的 `agent_apps/access.rs`。

恢复在主机与 Claude CLI 锁内，分别将默认钥匙串、后备文件和账号资料与事务的前后快照核对。各资源可以停在不同阶段，其中任一资源已被外部改写时返回冲突并保留日志。启动先核对主机、租户、操作与资源，再允许供应商恢复处理该意图。未发布时保留原指针并恢复原凭据和资料，发布后根据已落盘的指针合同保留目标登录。同账号重登以登录日志清除为提交点。恢复后再次调用不产生写入。

## 验证

macOS 27.0（26A428），arm64。独立子进程在 14 个中断场景直接退出，父进程重新打开主机及事务并连续恢复两次。覆盖登录日志、钥匙串、后备文件、资料文件、供应商意图与发布各阶段、两份日志的清理，以及回滚本身再次中断。测试核对指针及租户、设置、账号资料、默认凭据、归档和目标副本。使用真实临时文件与 macOS 钥匙串，默认槽位映射到按临时目录派生的独立 service，验证读写删。拒绝场景覆盖外部改写资料、后备文件、钥匙串、不同租户、不同操作及无效日志，拒绝后逐项核对现场和日志。相邻回归覆盖跨主机拒绝、实际钥匙串授权拒绝与切换失败。

红灯阶段，回滚在删除钥匙串后再次中断，预期 `Ok(())`，实际为 `Err(Conflict)`。外部改写账号资料，预期 `Err(Conflict)`，实际为 `Ok(())`。

红灯和绿灯均领取槽位 2。两阶段的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。格式化在临时源码快照中运行，只回写本批 Rust 文件。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0、0 | 通过 |
| 红灯 | `cargo test -p vw-agent-apps --lib auth6_claude_recover -- --nocapture` | 101 | 0 通过、2 失败、763 过滤 |
| 绿灯 | `cargo test -p vw-agent-apps --lib auth6_claude_recover -- --nocapture` | 0 | 2 通过、0 失败、763 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib cc04_claude_login -- --nocapture` | 0 | 10 通过、0 失败、755 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_claude_switch -- --nocapture` | 0 | 2 通过、0 失败、763 过滤 |
| 工作区静态检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异检查 | `git diff --check` | 0 | 通过 |

两个阶段均已释放本批槽位锁并保留编译缓存。
