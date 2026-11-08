# Codex 定向重新授权

母任务 6／子任务 16。以批次 1–15 的正式证据和批次 15 的 PKCE 提交 `77a27918b` 为前置。对照 cc-switch 4.0.25（`ca58c031`）中 Codex 定向重登、重复账号和邮箱变更用例。

设备与浏览器授权都在启动时绑定 `target_account_id`。提交沿用流程登记的租户、主体、主机、服务和目标 ID，并重新检查取消与到期。身份由 ChatGPT workspace 和 ID token 的 `sub` 共同确定，邮箱变更允许继续使用原 ID。授权响应缺少 ID token 时返回 `reauth_required`。

主机侧的 `AuthAccountStore` 在凭据租约内读取原记录并检查同作用域的重复身份。不同主体或 workspace 返回 `identity_mismatch`，身份已归同作用域另一条记录则返回 `codex_oauth_duplicate_account`。目标不存在返回 `not_found`，早于已提交认证的结果返回 `conflict`。成功时仅替换目标的凭据并刷新认证时间，保留原受管 ID 和其他账号。元数据更新确认失败时恢复旧文件。数据库无法确认提交结果时保留当前文件并返回存储错误。

共享身份解析仍在 `vw-agent-apps/src/auth/codex_device.rs`，浏览器入口在同目录的 `codex_browser.rs`。SQLite、PostgreSQL 与私有凭据文件通过 `vw-agent/src/agent_apps/auth_accounts.rs` 的现有适配器写入。独立测试复用原有模块注册。真实 Codex 账号验收归批次 58。

## 验证

红灯时两项测试均到达行为断言。设备与浏览器携带合法目标 ID 启动，实际分别得到 `Some(Unsupported)`，期望无错误。绿灯使用真实回环 HTTP 和临时私有凭据文件，仅替代公网 OAuth。双库验证检查同一目标更新、跨主机隔离、身份不符、重复身份、陈旧结果和 SQLite 写入失败回滚。

格式化、红灯及最终测试领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Clippy 领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。格式化在源码快照中执行，仅回写本任务 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_codex_reauth -- --nocapture`（红灯） | 101 | 0 通过、2 失败、739 过滤 |
| `cargo fmt --all` | 0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_codex_reauth -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、739 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_codex_device -- --nocapture` | 0 | 2 通过、0 失败、739 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_codex_browser -- --nocapture` | 0 | 2 通过、0 失败、739 过滤 |
| `cargo test -p vw-agent --lib auth6_codex -- --nocapture` | 0 | SQLite 4 项、PostgreSQL 2 项通过，0 失败、11739 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |

双库测试包含既有设备授权回归。测试链接出现既有 macOS `__eh_frame` 大小警告。临时 PostgreSQL 实例已停止，两个槽位锁已释放，编译缓存保留。
