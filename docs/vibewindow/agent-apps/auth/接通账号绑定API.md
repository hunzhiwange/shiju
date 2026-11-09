# 接通账号绑定 API

母任务 6／批次 42。直接前置为批次 41 的 `ad12b4449`。API 沿用批次 38–41 的管理路由、可信策略和私有执行主机，供应商事务沿用批次 33 的 Kimi 绑定及 `aeff6b430` 交付的 Codex、Claude 和 AGY 官方卡绑定。[接口契约](接口契约.md) · [供应商事务](../../../shiju/agent-apps/provider-contracts.md#官方卡账号引用与删除事务交接)

`PUT /v1/agent-apps/auth/bindings/{provider_id}?host_id=…&service=…` 接收 `{"account_id":"…"}` 或 `{"account_id":null}`，成功返回空的 204。省略字段返回 `invalid_input`。管理 Basic、可信主体令牌与租户头沿用授权链。受保护策略的同一作用域 grant 必须同时声明 `provider_config_write: true` 和包含目标服务的 `auth_binding_write`。AGY 与 Kimi 还须显式登记原生槽所有者。跨作用域账号及由其他作用域持有的供应商绑定返回 404，响应携带 `Cache-Control: no-store`。已知的业务错误返回稳定码和关联 ID。

Codex、Claude 与 AGY 调用现有目录事务的比较写入。Kimi 绑定继续核验原生 grant 并提交模型选择，解绑在原事务锁内原子删除引用，保留 CLI 当前模型与凭据。Claude 当前激活卡沿用既有切离要求。DSH 的共享 grant 没有供应商绑定。Copilot 和 xAI 当前缺少生产 `ProviderRouteRuntime` 的受管账号消费者，这三个服务在本执行端返回 `unsupported`。

## macOS 验证

macOS 27.0（26A428），arm64。测试通过真实回环 HTTP 调用管理路由。SQLite 使用临时数据库，PostgreSQL 18 使用 Lima 隔离实例和独立测试 schema。两库均确认 Codex 绑定、替换、重复请求和解绑，Kimi 绑定、选择模型和解绑后保留原生配置与 grant。拒绝用例覆盖跨公司、主机和主体账号，凭据缺失时保留原绑定，配置写权限与绑定权限分离，原生槽所有权，非官方卡，错误请求体、在线远端无执行通道及离线主机。AGY 邻接测试使用真实 macOS 钥匙串，验证测试条目的读写、清除与无槽位授权时拒绝。

红灯的 SQLite 两项行为断言分别预期 204 和 404，实际均为占位路由的 501。首次编译时修正了夹具的错误类型转换。红灯中的 PostgreSQL 两项在连接串缺少 `sslmode` 时未进入行为断言。绿灯首次执行时，两库的 Kimi 夹具均因缺少原生 OAuth 锁目录而返回 503。补齐目录后，四项聚焦测试通过。

红灯动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。绿灯动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。两阶段的 `CARGO_BUILD_BUILD_DIR` 分别与其目标目录相同。格式化在源码快照中执行，只回写本批 Rust 文件。

| 阶段 | 确切命令 | 退出码 | 有效结果 |
| --- | --- | ---: | --- |
| 红灯及绿灯 | `cargo fmt --all` | 0 | 格式化通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_api_binding -- --nocapture` | 101 | SQLite 2 项预期失败，PostgreSQL 2 项未进入断言，11783 项过滤 |
| 绿灯 | `cargo test -p vw-agent --lib auth6_api_binding -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 2 项通过，0 项失败，11783 项过滤 |
| 授权路由回归 | `cargo test -p vw-agent --lib auth6_api_ -- --nocapture` | 0 | 18 项通过，0 项失败，11769 项过滤 |
| 真实钥匙串 | `cargo test -p vw-agent-apps --lib provider_agy_binding -- --nocapture` | 0 | 1 项通过，0 项失败，797 项过滤 |
| 全工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0 | 通过 |

测试链接器仍提示既有的 `__eh_frame` 体积。临时 PostgreSQL 已停止，各阶段槽位锁已释放，编译缓存已保留。
