# 授权轮询 API

母任务 6／批次 40。批次 39 由 `017a3c61e` 交付。轮询复用管理链、主机授权策略、受管存储和主机内流程状态机。[接口契约](接口契约.md) · [启动授权与回填](接通启动授权API.md)

`GET /v1/agent-apps/auth/flows/{flow_id}?host_id=…&service=…` 要求管理 Basic 认证、租户头和可信主体令牌。现有 `auth_flow_start` grant 校验平台、公司、主机和服务，再以当前可信主体查询同一启动注册表。另一个具备相同 grant 的主体、另一获准服务、未知或格式错误的流程 ID 都返回 `404 not_found`。缺失认证或授权按管理链返回 `401` 或 `403`。

成功响应为 `200` 与 `Cache-Control: no-store`。共享流程 DTO 表示 `pending`、`approved`、`expired`、`failed` 和已有的 `cancelled`。响应公开等待时间、脱敏账号或安全错误。OAuth 挑战、用户码和令牌留在原有边界。GET 只读取快照，设备码兑换仍由执行主机按间隔推进。公开取消操作由批次 41 交付。[流程到期与节流](流程到期与轮询节流.md)

## macOS 验证

macOS 27.0 arm64。格式化在源码快照执行，仅回写本批 Rust 文件。所有 Cargo 阶段动态取得槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 使用同一路径。PostgreSQL 18 通过 Lima 临时实例及回环转发运行。

占位处理器的红灯在三项行为断言失败。SQLite 和 PostgreSQL 的所有者轮询实际 501、预期 200。另一获准主体实际 501、预期 404。首次试跑的夹具编译错误已修正，行为统计来自重新编译成功后的测试。

绿灯通过真实回环 HTTP、管理策略文件、临时受管凭据目录、SQLite 和 PostgreSQL。脚本进程替代外部 Claude 登录程序，验证重复查询保持待授权，回填完成后账号已持久化并与响应一致。测试检查快照不包含启动链接、回填值或令牌，并通过既有状态机构造到期、供应商拒绝和取消，检查终态重复读取一致。真实 OAuth 账号验证由后续批次负责。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯与绿灯 | `cargo fmt --all` | 0 | 通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_api_poll -- --nocapture` | 101 | 3 项预期失败，11777 项过滤 |
| 绿灯 | `cargo test -p vw-agent --lib auth6_api_poll -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 1 项通过，0 项失败，11777 项过滤 |
| 授权 API 回归 | `cargo test -p vw-agent --lib auth6_api_ -- --nocapture` | 0 | 11 项通过，0 项失败，11769 项过滤 |
| 全工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0 | 通过 |

测试链接器仍给出既有 `__eh_frame` 大小提示。临时 PostgreSQL 已停止，槽位锁已释放，编译缓存已保留。
