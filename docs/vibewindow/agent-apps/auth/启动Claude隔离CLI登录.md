# 启动 Claude 隔离 CLI 登录

母任务 6／批次 23，依赖[切换与刷新 Kimi 账号凭据](切换与刷新Kimi账号凭据.md)以及[主机凭据文件存储](执行主机凭据文件存储.md)。来源为 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/claude_account/{login,layout,identity}.rs`。

macOS 主机入口为 `vw_agent_apps::auth::claude_login::ClaudeLogin::start`。调用者提供主机发现的绝对 Claude 可执行路径、已授权的完整账号作用域和已有的托管目录账号 ID。入口复用 `CredentialFileStore::for_owner` 和凭据租约，命令固定为 `claude auth login --claudeai`，工作目录和 `CLAUDE_CONFIG_DIR` 同指该账号目录。子进程仅接收运行所需的环境变量，浏览器调用写入本次登录的私有临时目录。非 ASCII 配置路径沿用现有 macOS 钥匙串适配的拒绝规则。

优先返回截获的可信 HTTPS 授权链接，终端输出用作回退。两种来源共用主机、授权路径和回调目标检查。进程输出只在主机侧短暂缓冲，错误仅返回分类。取消、超时、丢弃句柄或 CLI 退出均终止进程组并回收 CLI，随后清理临时文件并释放租约。手工回填、账号身份读取与发布及真实 Claude 登录分别留给后续批次。

## 验证

macOS 27.0（26A428），arm64。聚焦测试使用真实临时账号目录、凭据租约、Claude 替代脚本及其后代进程。成功场景断言工作目录、环境隔离、链接优先级和退出后的账号文件；失败场景覆盖另一主体、伪造授权主机、错误回调、非零退出、取消、超时和丢弃句柄。测试以进程状态确认 CLI 已回收、辅助进程已停止，并检查临时目录删除及租约可重新取得。

红灯使用最小接线。成功场景在要求可信链接处收到 `Unavailable`；不可信链接后退出场景预期 `Exited`，实际收到 `Unavailable`。

格式化使用仓库源码快照，仅回写本批文件。构建阶段的 `CARGO_BUILD_BUILD_DIR` 与 `CARGO_TARGET_DIR` 一致。各阶段结束后释放本批锁并保留缓存。

| 命令 | 槽位 | 退出码 | 结果 |
| --- | ---: | ---: | --- |
| `cargo fmt --all`（红灯／最终） | 2／1 | 0／0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_claude_login -- --nocapture`（红灯） | 2 | 101 | 0 通过、2 失败、753 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_claude_login -- --nocapture`（最终） | 1 | 0 | 6 通过、0 失败、753 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings`（最终） | 1 | 0 | 通过 |
| `git diff --check` 与 `git diff --cached --check` | — | 0／0 | 通过 |

- 槽位 1 的 `CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 槽位 2 的 `CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
