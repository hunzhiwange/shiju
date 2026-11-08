# 启动与完成 Codex 设备授权

母任务 6／子任务 14。已核对批次 1–13 的交付记录，以及母任务所需的 cc-switch 冻结源快照、原生包注册、认证与主机合同、主机凭据文件和供应商事务。对照 cc-switch 4.0.25（`ca58c031`）的 `proxy/providers/codex_oauth_auth.rs` 和 `commands/auth.rs`。实现位于 `vw-agent-apps` 的 `auth/codex_device.rs`，主机存储适配位于 `vw-agent` 的 `agent_apps/auth_accounts.rs`。

- 启动请求仅发往固定的 OpenAI HTTPS 端点。公开挑战包含短用户码、固定验证 URL、随机流程 ID 和轮询时间。上游设备标识由执行主机持有。轮询间隔接受来源的数字和字符串格式，并加 3 秒安全余量。未提供有效期时采用 900 秒。
- 轮询通过主机流程状态机校验作用域、取消、到期和节流。OpenAI 的 403、404 通常表示等待，410 表示过期，显式 `access_denied` 转为授权拒绝。轮询返回授权码与 verifier 后，按来源约定提交表单兑换。全部请求禁用重定向。错误只投影稳定领域码。[主机流程状态机](主机内授权流程状态机.md)
- ID token 必须含可用的用户 subject。workspace ID 优先取自 ID token，缺失时从 access token 的 claims 读取。完整作用域、workspace 和 subject 派生不透明账号 ID。此阶段显示固定的 `ChatGPT` 标签，公开状态与账号元数据不携带上游邮箱、原始身份和令牌。新登录发现重复身份时报错，原凭据继续可用。
- 主机在私有凭据文件中保存原生 Codex `auth.json` 结构中的 access、refresh、ID token、workspace ID 和取令牌时的 UTC `last_refresh`。同一账号在独占租约内写入文件和登记脱敏元数据，元数据写入确认失败时清理未发布的文件。两者完成后流程才批准。浏览器、定向重登及真实 OpenAI 登录分别由后续任务覆盖。[私有凭据文件](执行主机凭据文件存储.md)

## macOS 验证

macOS 27.0（26A428），arm64。回环 HTTP 夹具校验 JSON、兑换表单和请求路径。使用真实临时私有目录回读整束令牌，覆盖早轮询、等待、拒绝、到期、跨主机访问、缺少刷新令牌或 subject，以及取消后的迟到兑换。主机存储夹具使用 SQLite 临时库和 Lima 中的隔离 PostgreSQL，验证重开后的账号与凭据隔离、重复拒绝和 SQLite 写入失败回滚。

红灯退出 101，两项在行为断言失败。有效挑战应能启动，实际返回 `Unsupported`。空设备标识应返回 `UpstreamFailure`，实际返回 `Unsupported`。

格式化、聚焦测试和双库测试动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Clippy 动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段将 `CARGO_BUILD_BUILD_DIR` 设为对应槽位目录。格式化在源码快照执行，只回写本任务文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_codex_device -- --nocapture`（红灯） | 101 | 0 通过、2 失败、735 过滤 |
| `cargo fmt --all` | 0 | 最终格式化通过 |
| `cargo test -p vw-agent-apps --lib auth6_codex_device -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、735 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_copilot -- --nocapture` | 0 | 6 通过、0 失败、731 过滤 |
| `cargo test -p vw-agent --lib auth6_codex_device_store -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 1 项通过，0 失败、11739 过滤 |
| `cargo test -p vw-agent --lib auth6_copilot_store -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 1 项通过，0 失败、11739 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |

`vw-agent` 测试链接仍报告既有的 macOS `__eh_frame` 大小警告。隔离 PostgreSQL 测试 schema 已清理，实例已停止。槽位 1、2 的本任务锁均已释放，缓存保留。
