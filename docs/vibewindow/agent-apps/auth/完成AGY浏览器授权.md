# AGY 浏览器授权

母任务 6／批次 28。前置批次 1–27 已核对，直接前置为[吸收 Claude 外部刷新并更新登录状态](吸收Claude外部刷新并更新登录状态.md)（`ed4d879ba`）。开工依赖包括冻结来源、原生业务包、认证及执行主机合同、主机存储和供应商事务。来源为 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/agy_account/{oauth,mod,tests}.rs`。

`vw_agent_apps::auth::agy_login::AgyLogin` 在授权的 macOS 执行主机启动、轮询 AGY 登录，取消使用 `AuthFlows::cancel`。Google 授权请求沿用 AGY 安装型客户端、原七项 scopes、offline 和选账号／同意提示。回调只监听 `127.0.0.1` 的随机端口，路径为 `/oauth-callback`，有效期十分钟。异机浏览器在启动前收到 `callback_unreachable`。同一作用域的新尝试取消旧流程。其他作用域各自持有流程。

回环处理复用从 Codex 提取的监听和校验。错误路径、错误 state 和重复参数均拒绝。有效回调只接收一次，轮询关闭监听后通过固定 HTTPS 令牌端点兑换。PKCE verifier 只在主机生成、保存并随兑换发送。取消和到期使迟到的兑换结果失去提交资格。授权拒绝和缺少长期凭据返回脱敏状态。

托管账号使用由平台、公司、执行主机、调用者、服务和 Google subject 派生的不透明 ID。在执行用户默认钥匙串的 `shiju.agy.oauth` 命名空间内，单个条目原子保存账号投影和来源兼容的凭据 JSON，回读后批准流程。使用已锁定版本的 macOS Security Framework 接口承载完整令牌包，规避现有命令行适配器的交互行长度限制。原生单槽切换由批次 29 实现，公开流程接线由批次 39–41 实现，真实 Google 账号由批次 62 验收。

## 验证

macOS 27.0（26A428），arm64。测试使用真实回环 HTTP 和 macOS 钥匙串，仅以本机 HTTP 替代 Google 令牌端点。成功用例核对授权 URL、兑换表单、来源客户端及七项 scopes、S256 challenge 与 verifier 对应关系、回调重放拒绝、作用域隔离、2 KiB 以上凭据写入／回读／删除和脱敏结果。失败用例覆盖异机、错误路径／state、重复参数、拒绝授权、缺少刷新令牌、同作用域替换、取消、在途兑换丢弃和到期。取消后和缺少刷新令牌时测试命名空间中无对应条目。

红灯时，本机启动应返回监听句柄，实际返回 `Unsupported`。异机应返回 `CallbackUnreachable`，实际返回 `Unsupported`。扩充用例后的中间检查中，两条测试命令和 Clippy 均因测试摘要格式化的 `E0277` 退出 101，行为测试未运行。已改用 Base64URL 比较摘要。

| 阶段 | 槽位 | CARGO_TARGET_DIR |
| --- | ---: | --- |
| 红灯、初轮绿灯 | 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` |
| 中间检查、最终验证 | 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` |

`CARGO_BUILD_BUILD_DIR` 与各阶段的 `CARGO_TARGET_DIR` 一致。格式化在源码快照执行，仅回写本批 Rust 文件。各阶段释放了本任务的槽位锁并保留缓存。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、初轮绿灯、中间检查、最终验证 | `cargo fmt --all` | 0／0／0／0 | 通过 |
| 红灯 | `cargo test -p vw-agent-apps --lib auth6_agy_login -- --nocapture` | 101 | 0 通过、2 失败、768 过滤 |
| 初轮绿灯、最终验证 | `cargo test -p vw-agent-apps --lib auth6_agy_login -- --nocapture` | 0／0 | 每轮 2 通过、0 失败、768 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_codex_browser -- --nocapture` | 0／0 | 每轮 2 通过、0 失败、768 过滤 |
| 初轮绿灯、最终验证 | `cargo clippy --all-targets --all-features -- -D warnings` | 0／0 | 全工作区通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |
