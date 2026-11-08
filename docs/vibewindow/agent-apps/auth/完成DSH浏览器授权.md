# 完成 DSH 浏览器授权

母任务 6／批次 31。对照前置批次 1–30 的交付提交，直接前置为 [处理 AGY 令牌刷新与外部写回](处理AGY令牌刷新与外部写回.md)（`99106d25e`）。来源是 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/dsh_account.rs` 和 `commands/dsh.rs`。

`vw_agent_apps::auth::dsh_login::DshLogin` 在已授权的 macOS 执行主机运行。主机提供可信的账号作用域、回环可达性和安装设备 ID。每个作用域保留一个待处理流程。执行主机向固定 HTTPS issuer `https://platform.deepseek.com` 发送 `auth_init`，只向客户端返回验证过的 `/dsh/authorize` 链接和独立流程 ID。回环监听绑定 `127.0.0.1` 的临时端口，只在限定时间内接受 `/oauth/callback` 上唯一且匹配的 state 和唯一授权码，请求头上限沿用共享监听的 8 KiB。回环页提示用户回到客户端查看最终结果。

`auth_exchange` 和 `auth_cancel` 发送对应流程的 PKCE verifier。三个登录请求均直连固定 issuer，禁用 HTTP 重定向。已有 DSH 账号接口使用 `x-dsh-auth-token`，本批三个 PKCE 请求使用 JSON 请求体。只有当前流程能将有效 grant 交给执行主机的作用域提交回调，提交成功才发布脱敏的 `Approved`。取消先使流程失去提交资格并关闭监听，再尽力通知平台。在途兑换收到成功响应也无法恢复已取消的流程。

批次 32 接入 DSH 共享 YAML 的设备 ID 和 grant 存储、读取与登出。公网 DeepSeek 账号验收归批次 63。

## 验证

macOS 27.0（26A428），arm64。测试保留真实回环监听、HTTP 请求和临时文件写入，以本地响应替换公网 OAuth。成功用例核对 PKCE S256 参数、设备字段、作用域、原子文件写入、脱敏 DTO 和重复轮询。拒绝用例核对异机回环、授权地址、state、重复授权码、8 KiB 请求头上限、跨作用域取消、在途兑换取消、错误 issuer，以及 307 的目标服务器未收到请求。

红灯阶段两项行为断言均收到 `Unsupported`，分别预期成功启动和 `CallbackUnreachable`。首轮实现验证中，macOS 关闭含未读请求字节的连接时重置了测试客户端的 TCP 连接。随后用原始 HTTP 将请求恰好写至缓冲区上限，并检查 431 响应。

各阶段动态领取槽位 1。`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。格式化在源码快照运行，仅将本批文件写回工作区。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、首轮实现、最终验证 | `cargo fmt --all` | 0／0／0 | 格式化成功 |
| 红灯 | `cargo test -p vw-agent-apps --lib auth6_dsh_login -- --nocapture` | 101 | 0 通过、2 失败、775 过滤 |
| 首轮实现 | `cargo test -p vw-agent-apps --lib auth6_dsh_login -- --nocapture` | 101 | 1 通过、1 失败、775 过滤 |
| 最终验证 | `cargo test -p vw-agent-apps --lib auth6_dsh_login -- --nocapture` | 0 | 2 通过、0 失败、775 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_agy_login -- --nocapture` | 0 | 2 通过、0 失败、775 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_codex_browser -- --nocapture` | 0 | 2 通过、0 失败、775 过滤 |
| 最终验证 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |
