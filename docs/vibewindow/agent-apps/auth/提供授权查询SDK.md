# 提供授权查询 SDK

母任务 6／批次 46。直接前置为批次 45 的 `9ebc72366`。SDK 使用批次 38 注册的管理路由、可信主体策略及批次 4 的公开 DTO。[接口契约](接口契约.md) · [查询路由](接通脱敏账号查询API.md) · [批次 45](接通登出撤销API.md)

原生 `GatewayClient` 提供 `agent_apps_auth_accounts`、`agent_apps_auth_status` 和 `agent_apps_auth_bindings`。每次请求携带管理 Basic 凭据、独立主体令牌及端点所选的平台和公司。查询参数指定主机和服务，绑定查询可按账号筛选。客户端使用禁用重定向和重试的主机传输，请求带 `Cache-Control: no-cache, no-store`，每次读取都访问网关。网关的结构化错误保留状态、稳定码和关联 ID，非契约错误仅保留 HTTP 状态。

## macOS 验证

macOS 27.0（26A428），arm64。验证使用 `9ebc72366` 加本批文件的独立源码快照，以隔离共享工作区的并行修改。格式化只回写本批 Rust 文件。红灯动态领取槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。绿灯动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与目标目录相同。

| 阶段 | 确切命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0／0 | 通过 |
| 红灯 | `cargo test -p vw-gateway-client --lib auth6_sdk_read -- --nocapture` | 101 | 0 通过、2 失败、364 过滤。占位 SDK 返回 501，断言分别期待成功的账号列表和结构化 400。 |
| 绿灯 | `cargo test -p vw-gateway-client --lib auth6_sdk_read -- --nocapture` | 0 | 2 通过、0 失败、364 过滤 |
| 全工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |

回环 HTTP 服务器记录 GET 方法、版本化路径、查询参数及认证和租户头。验证覆盖两个公司使用各自主体令牌、切回原公司重新读取、绑定的可选账号筛选、非法参数的 URL 编码及 400、已删除账号的 404、关联 ID 保留，以及丢弃未结构化的错误响应体。无法构造 HTTP 头的主体令牌在发送前被拒绝。此处验证 SDK 线协议，生产管理路由及 SQLite、PostgreSQL 的作用域验证见批次 38。两阶段槽位锁均已释放，缓存保留。
