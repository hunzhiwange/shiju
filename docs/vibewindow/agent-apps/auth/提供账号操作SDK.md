# 提供账号操作 SDK

母任务 6／批次 48。批次 47 已由 `025d0cfe5` 交付。本批复用已接通的管理路由和执行主机事务。[账号绑定](接通账号绑定API.md) · [默认账号](接通默认账号API.md) · [删除](接通删除账号API.md) · [登出与撤销](接通登出撤销API.md)

| `GatewayClient` 方法 | 行为 |
| --- | --- |
| `agent_apps_auth_bind` | 设置供应商账号引用。传入 `None` 时发送显式 JSON `null` 解绑。 |
| `agent_apps_auth_set_default` | 选定作用域内的默认账号。 |
| `agent_apps_auth_delete` | 删除指定账号。网关可用 `AccountInUse` 拒绝仍被供应商引用的账号。 |
| `agent_apps_auth_logout` | 按指定账号、默认账号或全部账号执行本地登出，保留账号记录和绑定。 |
| `agent_apps_auth_revoke` | 请求上游撤销。`RemoteRevocationUnconfirmed` 表示本地登出已完成，上游未确认。 |

五个操作使用端点租户、显式主机和服务、Basic 管理凭据及可信主体令牌，仅以 HTTP 204 确认成功。Kimi 供应商标识支持 Unicode 与空格，作为单个 URL 路径段编码。SDK 复用授权流程的无重试、无重定向传输。错误保留 HTTP 状态、`ReauthRequired`、`Unsupported` 等稳定码及关联 ID，错误格式化省略关联 ID 和原始正文。`AgentAppsAuthFlowError` 仍可作为共享 `AgentAppsAuthError` 的兼容导出使用。

## macOS 验证

macOS 27.0（26A428），arm64。两阶段在基线 `025d0cfe5` 加本批 Rust 文件的隔离源码快照中运行，格式化后只回写本批文件。每阶段独立领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。

回环 HTTP 固件记录请求方法、路径、作用域、认证头和 JSON 正文。覆盖绑定和显式解绑、Kimi 标识编码、默认切换、三种登出选择、删除及独立上游撤销。拒绝用例覆盖需重登、过期引用、账号仍被引用、服务不支持、上游撤销未确认、非 204、无效标识和令牌、传输失败及错误输出脱敏。数据库、真实进程和钥匙串证据见上述网关批次。

红灯使用能编译并注册测试的占位 SDK。成功用例期待绑定确认，实际为 `Request(Unsupported)`。拒绝用例期待保留 `ReauthRequired`，实际为 `Request(Unsupported)`。红灯夹具原用 HTTP 400，绿灯按网关 `FlowError` 的映射校正为 409。

| 阶段 | 确切命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0／0 | 通过 |
| 红灯 | `cargo test -p vw-gateway-client --lib auth6_sdk_accounts -- --nocapture` | 101 | 0 通过、2 失败、368 过滤 |
| 绿灯 | `cargo test -p vw-gateway-client --lib auth6_sdk_accounts -- --nocapture` | 0 | 2 通过、0 失败、368 过滤 |
| 授权 SDK 回归 | `cargo test -p vw-gateway-client --lib auth6_sdk_ -- --nocapture` | 0 | 6 通过、0 失败、364 过滤 |
| 全工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |

两阶段均已释放所持槽位锁并保留编译缓存。
