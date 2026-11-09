# 接通删除账号 API

母任务 6／批次 44。直接前置为批次 43 的 `6f3dbf54e`，删除事务由批次 35 的 `f0ec26aea` 交付。管理路由沿用批次 38 在 `agent_apps_auth_read.rs` 中建立的授权子路由。[接口契约](接口契约.md) · [删除事务](删除账号并处理绑定引用.md)

`DELETE /v1/agent-apps/auth/accounts/{account_id}?host_id=…&service=…` 无需请求体，成功返回空的 204。请求使用现有管理 Basic 认证、可信主体令牌和租户头。管理员在对应平台、公司、主机的 grant 中用 `"auth_account_delete": ["codex_oauth"]` 等服务列表授予权限，缺省拒绝。AGY 和 DSH 还要求受信策略登记原生槽的独占所有者。

- 复用 `AppProviderStore::remove_auth_account` 处理引用、主机凭据和默认值。供应商仍引用目标时返回 409 `account_in_use`，响应不包含供应商标识。清理默认值后保持未选定。
- 已授权作用域内，账号不存在或归属其他租户、主体、执行主机时返回相同的 404 `not_found`。响应包含稳定码、固定消息及关联 ID，并携带 `Cache-Control: no-store`。
- macOS 执行主机在阻塞线程运行删除事务，请求断开后事务仍可完成。其他平台返回 `unsupported`，远端缺可信执行通道时由管理鉴权链返回 `execution_channel_unavailable`。

## macOS 验证

macOS 27.0（26A428），arm64。红灯、绿灯均动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。格式化在源码快照执行，仅回写本批 Rust 文件。

HTTP 测试通过真实回环网关访问私有凭据文件、SQLite 文件和 Lima 中的 PostgreSQL 独立 schema。两库分别验证删除有绑定的 Codex 账号返回 409，显式解绑后返回 204，凭据与数据库默认记录均移除，邻接公司的同名账号与默认值仍可读。跨公司、主机、主体与不存在的 ID 返回相同的 404 消息和稳定码。测试也覆盖非法 ID、独立删除授权、缺少 AGY／DSH 槽所有权、远端无通道、主机离线及非管理路由。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0／0 | 通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_api_remove -- --nocapture` | 101 | 0 通过、4 失败、11792 过滤。SQLite 与 PostgreSQL 均收到占位 501，分别预期引用冲突 409 与账号缺失 404。 |
| 绿灯 | `cargo test -p vw-agent --lib auth6_api_remove -- --nocapture` | 0 | 4 通过、0 失败、11792 过滤。SQLite、PostgreSQL 各 2 项。 |
| 原生回归 | `cargo test -p vw-agent-apps --lib auth6_remove -- --nocapture` | 0 | 5 通过、0 失败、793 过滤。含真实 macOS 钥匙串隔离项的读取、拒绝和删除，及 DSH 文件共享 grant 清理。 |
| 全工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |

macOS 测试链接器提示既有的 `__eh_frame` 大小。临时 PostgreSQL 已停止，槽位锁已释放，构建缓存已保留。
