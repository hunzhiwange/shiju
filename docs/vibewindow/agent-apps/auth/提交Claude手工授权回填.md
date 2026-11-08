# 提交 Claude 手工授权回填

母任务 6／批次 24。基于[启动 Claude 隔离 CLI 登录](启动Claude隔离CLI登录.md)的 macOS 主机句柄。来源为 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `commands/claude_account.rs` 和 `services/claude_account/{mod,login}.rs`。

`vw_agent_apps::auth::claude_login::ClaudeLogin::submit_paste` 接收复制链接后的用户输入。调用者通过授权作用域和流程 ID 找到本次登录的专属句柄，再调用此入口。状态取自该句柄已校验的授权链接。与状态匹配的 `code#state` 作为一行写入该 CLI 的 stdin。完整回环 URL 必须匹配原回调端口、路径和 state，并包含单个非空 code。请求使用原回调地址，`localhost` 仅解析到 IPv4 或 IPv6 回环地址，禁用代理、重定向及自动重试。

CLI 监督循环串行投递，并在派发前检查进程和期限。取消、到期或退出终止投递与清理进程。回填结果只确认投递，授权结果由后续轮询读取。格式错误和非活动流程使用固定错误分类，分类不含原始输入、URL 或传输错误。

## 验证

macOS 27.0（26A428），arm64。测试使用真实临时账号目录、凭据租约、替代 Claude 子进程及真实回环监听。覆盖账号目录隔离、stdin 输入、回调地址归一化、禁止跟随 HTTP 302、非法格式、另一流程的 state、错误端口和路径、取消期间关闭连接、到期后拒绝及租约释放。真实 Claude 登录由批次 61 验收。

红灯使用最小入口。成功用例预期 `Ok(())`，实际为 `Err(Unavailable)`。拒绝用例预期 `Err(InvalidPaste)`，实际为 `Err(Unavailable)`。

格式化使用仓库源码快照，仅回写本批 Rust 文件。各阶段的 `CARGO_BUILD_BUILD_DIR` 与 `CARGO_TARGET_DIR` 相同。阶段结束后释放本批锁并保留缓存。

| 命令 | 槽位 | 退出码 | 结果 |
| --- | ---: | ---: | --- |
| `cargo fmt --all`（红灯／首次绿灯／最终） | 2／3／3 | 0／0／0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_claude_paste -- --nocapture`（红灯） | 2 | 101 | 0 通过、2 失败、759 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_claude_paste -- --nocapture`（首次绿灯／最终） | 3／3 | 0／0 | 每次 2 通过、0 失败、759 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_claude_login -- --nocapture`（首次绿灯／最终） | 3／3 | 0／0 | 每次 6 通过、0 失败、755 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings`（首次绿灯／最终） | 3／3 | 0／0 | 通过 |
| `git diff --check` 与 `git diff --cached --check` | — | 0／0 | 通过 |

- 槽位 2 的 `CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- 槽位 3 的 `CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
