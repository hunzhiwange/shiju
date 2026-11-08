# Codex 浏览器 PKCE 回调

母任务 6／子任务 15。核对了批次 1–14 的正式证据和批次 14 的设备授权提交 `45b7d81b3`。对照了冻结来源 cc-switch 4.0.25（`ca58c031`）的 `proxy/providers/codex_oauth_auth/browser.rs` 及令牌兑换实现。母任务 1–5 所需的来源快照、原生包注册、认证与执行主机合同、凭据存储和供应商事务已落地。

`vw-agent-apps/src/auth/codex_browser.rs` 在授权执行主机的 IPv4 回环端口 1455 接收 `GET /auth/callback`。客户端获得固定 HTTPS 授权页及 S256 challenge。verifier 和回调代码留在主机。异机浏览器返回 `callback_unreachable`，端口已占用返回 `conflict`。监听有效期 15 分钟，请求头限制 8 KiB、读取期限 2 秒。无效路径、错误或重复的 state，以及含重复参数的请求均不消费流程。匹配 state 的有效回调只接受一次，响应禁止缓存且不发送 referrer。

轮询关闭并等待监听退出，再将代码和该次流程的 verifier 送往固定令牌端点，由上游校验 PKCE。授权拒绝、取消及过期均阻止写入。流程注册表在提交时重查作用域与时限，`AuthAccountStore::poll_codex_browser` 复用设备授权的 Codex 身份校验和账号事务。定向重登由批次 16 接入，真实 Codex 账号验收归批次 58。

## 验证

红灯两项均在行为断言失败。可达的本机浏览器应启动监听，实际返回 `Unsupported`。异机浏览器应在绑定前返回 `CallbackUnreachable`，实际返回 `Unsupported`。

绿灯使用真实回环 HTTP 和临时凭据文件，仅替换公网令牌端点。核对了兑换表单、S256 challenge 与 verifier 对应关系、作用域内写入及脱敏，以及错误路径、重复回调、端口占用、请求上限、超时、取消和无轮询过期。

格式化、红灯和最终测试领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Clippy 领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。`CARGO_BUILD_BUILD_DIR` 同各自槽位。格式化在源码快照中执行，仅回写本任务 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_codex_browser -- --nocapture`（红灯） | 101 | 0 通过、2 失败、737 过滤 |
| `cargo fmt --all` | 0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_codex_browser -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、737 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_codex_device -- --nocapture` | 0 | 2 通过、0 失败、737 过滤 |
| `cargo test -p vw-agent --lib auth6_codex_device_store -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 1 项通过，0 失败、11739 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |

双库回归复用了同一个 Codex 账号事务。PostgreSQL 测试模式已清理，独立实例已停止。`vw-agent` 测试链接出现既有的 macOS `__eh_frame` 大小警告。两个槽位的本任务锁均已释放，编译缓存保留。
