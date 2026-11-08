# 获取账号专属 Copilot 会话令牌

母任务 6／子任务 13。批次 1–12 的正式证据已交付，复用[账号轮询与提交](完成Copilot设备授权轮询.md)、[受管元数据](持久化作用域账号元数据.md)和[执行主机凭据文件](执行主机凭据文件存储.md)。本项沿用批次 12 确认的开工条件。源快照、原生业务包注册、主机与账号作用域、受管存储和供应商事务合同均可用。本项不消费供应商桌面或模型列表的实机交付。

- 主机内部入口为 `AuthAccountStore::copilot_session`，输入已授权作用域和显式账号 ID。先读取相同作用域的账号元数据，再通过同账号的私有文件租约读取 OAuth 凭据。端点和敏感 Authorization 值成对返回。Gateway、SDK 和桌面没有新增令牌读取入口。
- 对照 cc-switch 4.0.25 冻结提交 `ca58c031` 的 `get_valid_token_for_account`、`fetch_copilot_token_with_github_token` 和 `fetch_and_cache_endpoint`。公开 GitHub 通过 `api.github.com/copilot_internal/v2/token` 兑换短期令牌，Enterprise 在受管域下查找端点并沿用同账号 OAuth 令牌。两种路径的发现请求均使用该账号的 OAuth 凭据。模型协议夹具 `copilot-endpoint-cases.json` 描述模型调用路径，归母任务 5。
- 缓存由租户、主体、执行主机、账号和 GitHub 域共同分隔。同键并发只执行一次刷新。缓存命中仍须重新读取租约内的凭据，OAuth 轮换后重新兑换和发现。公开令牌提前 60 秒刷新，发现端点后再次检查是否到期。Enterprise 的端点缓存 5 分钟。刷新失败清除本次失效的缓存。
- 发现结果仅接受 `api.githubcopilot.com`、`api.{individual,business,enterprise}.githubcopilot.com` 或该 Enterprise 域推导的 `copilot-api.<域>` HTTPS origin。响应未提供端点时使用同域默认值。非法端点及发现请求失败返回脱敏错误。兑换和发现均禁止重定向，401 返回 `ReauthRequired`，403 返回 `AuthorizationDenied`。响应正文和令牌不进入错误或日志。

## macOS 验证

macOS 27.0（26A428），arm64。回环 HTTP 使用来源的请求路径、身份头和响应形状。测试以真实主机私有临时目录存取凭据，验证账号、主体和域隔离、并发合并、轮换、到期刷新、Enterprise 路径和非法端点拒绝。SQLite 临时库及隔离 Lima 内的 PostgreSQL 18.6 验证主机入口对孤立凭据、跨主体元数据及非法受管域的拒绝行为。真实 GitHub 登录与推理请求由后续验收覆盖。

红灯在两个行为断言失败。有效账号本应返回会话，实际返回 `Unsupported`。含用户信息的非法 authority 本应返回 `InvalidInput`，实际返回 `Unsupported`。

格式化和测试动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。全工作区 Clippy 动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与对应目标目录一致。格式化在源码快照中执行，回写本任务涉及的 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_copilot_token -- --nocapture`（红灯） | 101 | 0 通过、2 失败、733 过滤 |
| `cargo fmt --all` | 0 | 红灯和绿灯阶段均成功 |
| `cargo test -p vw-agent-apps --lib auth6_copilot_token -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、733 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_copilot -- --nocapture` | 0 | 6 通过、0 失败、729 过滤 |
| `cargo test -p vw-agent --lib auth6_copilot_session_scope -- --nocapture` | 0 | SQLite、PostgreSQL 各 1 项通过，0 失败、11737 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |

首次双库检查退出 101，SQLite 通过，PostgreSQL 在夹具连接前拒绝缺少 `sslmode` 的测试 URL。给隔离回环实例显式设置 `sslmode=disable` 后重跑，两库通过。测试链接阶段报告已有的 macOS `__eh_frame` 大小警告。PostgreSQL 测试 schema 已清理，两次临时实例均已停止。槽位 1、2 的本任务锁均已释放，缓存保留。
