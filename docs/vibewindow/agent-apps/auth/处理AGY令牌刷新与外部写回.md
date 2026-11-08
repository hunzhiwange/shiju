# 处理 AGY 令牌刷新与外部写回

母任务 6／批次 30。前置批次 1–29 已对照交付提交核验。直接前置为[切换 AGY 钥匙串账号](切换AGY钥匙串账号.md)（`d6259aa40`）。来源为 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/agy_account/{mod,oauth}.rs`。

`vw_agent_apps::auth::agy_refresh::refresh_account` 由已授权的 macOS 执行主机调用。它从 `shiju.agy.oauth` 读取作用域账号，向 AGY 浏览器登录共用的 Google 端点发送刷新请求。网络请求期间释放钥匙串锁。提交前重新读取记录并核对身份与内容摘要，记录变化时返回 `conflict`。Google 返回 HTTP 400 和 `invalid_grant` 时，将当前版本原子更新为 `reauth_required`。刷新保留省略的刷新令牌、ID 令牌及凭据内的未知字段。返回的 access token 仅供主机使用。

`reconcile_native_account` 要求主机提供独占槽所有权及已绑定账号 ID。通过校验后读取 `gemini`／`antigravity`，只有身份和刷新授权相符且到期时间较新时才收录。提交前再次读取原生项。旧进程写回其他账号时返回 `conflict`，托管记录保持原值。刷新和收录只写托管记录。额度查询和 CLI usage 回退归母任务 7，真实 AGY 账号验收归批次 62。

## 验证

macOS 27.0（26A428），arm64。公网 OAuth 使用回环 HTTP 服务器，检查 Google 刷新表单形状。测试使用系统钥匙串唯一命名项读、写、回读并删除；第二个真实进程在切号前读取旧账号，在切号后向同一原生测试项写回。测试还覆盖无独占授权、外部主体、较旧凭据、OAuth 响应身份错误、临时上游失败，以及并发更新后拒绝过期的 `invalid_grant`。

红灯阶段，同账号写回收录断言预期 `Ok(())`，缺少独占授权断言预期 `Forbidden`，两者实际均收到 `Unsupported`。

红灯与绿灯均动态领取槽位 2。两阶段的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。格式化在源码快照运行，只回写本任务文件。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0／0 | 格式化成功 |
| 红灯 | `cargo test -p vw-agent-apps --lib auth6_agy_refresh -- --nocapture` | 101 | 0 通过、2 失败、773 过滤 |
| 绿灯 | `cargo test -p vw-agent-apps --lib auth6_agy_refresh -- --nocapture` | 0 | 2 通过、0 失败、773 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_agy_switch -- --nocapture` | 0 | 2 通过、0 失败、773 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_agy_login -- --nocapture` | 0 | 2 通过、0 失败、773 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_keychain -- --nocapture` | 0 | 2 通过、0 失败、773 过滤 |
| 绿灯 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |
