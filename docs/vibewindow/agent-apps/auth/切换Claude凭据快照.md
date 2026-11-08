# 切换 Claude 凭据快照

母任务 6／批次 25。来源为 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/claude_account/snapshot/{switch,creds,files}.rs`。批次 24 的回填入口见[提交 Claude 手工授权回填](提交Claude手工授权回填.md)。

快照复用 `providers/claude_login.rs` 中的 `LoginHost`。供应商切换沿用网关已取得账号租约的 `LoginHost::begin` 入口及现有事务发布点。同身份的重新登录通过 `LoginHost::replace_live` 更新主机默认槽位，保持供应商指针，调用方须持有目标账号租约。不同身份的重新登录保留当前槽位。两条路径均在现有主机、刷新和配置锁内检查资料与凭据的组织是否一致。归档回调后重读默认和目标快照，任一份发生变化即拒绝写入。

凭据与资料只替换账号字段，清除与旧账号关联的缓存，保留当前槽位的 MCP 授权和其他共享配置。托管目录沿用现有可独立启动的凭据副本。钥匙串写入并回读匹配后才更新配置。写入拒绝时，若当前快照仍是原样，清理本次私有日志。检测到部分写入时，沿用现有回滚和日志机制。恢复规则与外部轮换分别归批次 26、27。

## 验证

macOS 27.0（26A428），arm64。测试使用真实临时目录、现有供应商事务和实际 macOS 钥匙串读写删。测试将默认槽位映射到临时目录派生的独立 service，保留用户当前的 Claude 钥匙串条目。测试包含归档原账号、切换并提交配置、同身份重登、不同身份保持原状、归档时目标凭据轮换、钥匙串写入拒绝、失败后的配置及凭据原样回读。拒绝场景使用分别可写但合并后超过适配器输入上限的合成条目。相邻回归覆盖部分写入后回滚、跨主机拒绝、CLI 锁、文件后备和已有发布恢复。真实 Claude 账号端到端验收归批次 61。

红灯阶段已进入两个行为断言。重登用例预期 `Ok(true)`，实际为 `Err(InvalidResource)`。过时快照用例预期 `Some(Conflict)`，实际为 `None`。

红灯和绿灯均领取槽位 2。`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。格式化在源码快照中执行，只回写本批 Rust 文件。
阶段结束后已释放本批槽位锁并保留缓存。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0、0 | 通过 |
| 红灯 | `cargo test -p vw-agent-apps --lib auth6_claude_switch -- --nocapture` | 101 | 0 通过、2 失败、761 过滤 |
| 绿灯 | `cargo test -p vw-agent-apps --lib auth6_claude_switch -- --nocapture` | 0 | 2 通过、0 失败、761 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib cc04_claude_login -- --nocapture` | 0 | 10 通过、0 失败、753 过滤 |
| 工作区静态检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异检查 | `git diff --check` | 0 | 通过 |
