# 提供授权流程 SDK

母任务 6／批次 47。前置批次 46 的查询 SDK 已由 `e5dfc10b1` 交付。复用批次 4 的公开 DTO 和批次 39–41 的管理路由。[查询 SDK](提供授权查询SDK.md) · [启动与 Claude 回填](接通启动授权API.md) · [轮询](接通授权轮询API.md) · [取消](接通取消授权API.md)

原生 `GatewayClient` 提供四个方法，接受端点租户下的执行主机及管理凭据。启动、轮询和取消显式选择服务。回填固定为 `claude_official`，敏感输入只写入原流程 `/paste` 请求体。启动返回挑战，轮询返回一次状态快照及服务端给出的下一次等待时间。回填收到 202 后继续轮询，取消收到 204 才确认成功。

沿用禁用重定向和重试的传输实例及请求的 `Cache-Control: no-cache, no-store`。SDK 将流程 ID 限于单个路径段，并在发送前校验启动 DTO 的服务组合。HTTP 错误保留状态、稳定码和关联 ID，丢弃错误正文说明。传输和解码错误使用固定错误类型。错误格式化与 SDK 日志省略原始路径、回填、响应体和关联 ID。

## macOS 验证

macOS 27.0（26A428），arm64。格式化和验证使用基线 `e5dfc10b1` 加本批 Rust 文件的隔离源码快照，排除工作区并行改动。两阶段各自动态领取槽位 2。`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同路径。

| 阶段 | 确切命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0／0 | 格式化后仅回写本批文件 |
| 红灯 | `cargo test -p vw-gateway-client --lib auth6_sdk_flow -- --nocapture` | 101 | 0 通过、2 失败、366 过滤 |
| 绿灯 | `cargo test -p vw-gateway-client --lib auth6_sdk_flow -- --nocapture` | 0 | 2 通过、0 失败、366 过滤 |
| 全工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |

红灯使用已注册的占位 SDK。合法 Claude 启动应返回挑战，实际为 `Request(Unsupported)`。终态 Claude 回填应保留 HTTP 409 和 `Conflict`，实际为 `Request(Unsupported)`。两个断言均在成功编译和注册测试后触发。

回环 HTTP 验证记录方法、版本化路径、查询、认证、租户和请求体。覆盖 Claude 回填接受、待授权与批准，Codex 设备码启动及取消确认，服务端轮询间隔，终态冲突、失效引用的 404、非结构化错误、解码失败、传输失败及取消确认缺失。通过 tracing 事件与 span 字段和错误格式化检查挑战、流程 ID、凭据、回填与上游错误的脱敏。数据库、钥匙串和真实 CLI 的主机行为由批次 39–41 的既有证据覆盖。

两阶段的槽位锁均已释放，构建缓存已保留。
