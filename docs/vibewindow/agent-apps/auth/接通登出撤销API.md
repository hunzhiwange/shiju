# 接通登出与撤销 API

母任务 6／批次 45。直接前置为批次 44 的 `8f086acdd`。路由沿用批次 38 的管理装配，复用批次 36–37 的主机登出、撤销和刷新代际保护。开工边界沿用批次 38 记录的已交付底座、主机合同及供应商事务。[接口契约](接口契约.md) · [管理装配](接通脱敏账号查询API.md) · [主机登出与撤销](执行登出与撤销语义.md) · [刷新竞态](阻止撤销与刷新竞态.md)

两个入口均要求 Basic 管理认证、可信主体令牌、平台及公司作用域、`host_id` 与 `service`。服务列表 `auth_logout` 和 `auth_account_revoke` 分别授予操作权限，缺省拒绝。AGY 登出及 DSH 两种操作要求受信策略登记原生槽独占所有者。授权状态仅在 macOS、DSH、具备撤销授权和槽所有权时报告 `can_revoke`。

- `POST /v1/agent-apps/auth/logout` 接受 `account`、`default`、`all` 三种选择。204 确认作用域本地登出。保留托管账号记录和绑定，清除指向目标的托管默认。Kimi 仅在具备独占所有权时检查已声明的原生 OAuth 槽，按账号地区和凭据匹配清理。Claude 全部登出等待已开始的作用域流程清理完成。
- `POST /v1/agent-apps/auth/accounts/{account_id}/revoke` 由 DSH 主机服务先清除本地共享 grant，再请求官方端点，204 表示远端确认。502 `remote_revocation_unconfirmed` 表示本地已登出而远端未确认。无可撤销 grant 返回 409 `conflict`。缺少授权返回 403 `forbidden`，已授权但服务不支持时返回 501 `unsupported`。
- 两条路由共用流程运行时。错误响应采用固定消息、稳定码、关联 ID 和 `Cache-Control: no-store`。管理子路由之外的路径无对应入口。

## macOS 验证

macOS 27.0（26A428），arm64。共享工作区的其他未提交会话存储改动曾导致编译失败（包括 `session/store.rs` 缺少 `first_response_state`）。正式验证在基线 `bd82b7285` 加本批文件的独立源码快照上运行，使用原仓库共享 Cargo 缓存。

正式红灯及绿灯动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。共享工作区的中间红灯尝试曾领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与目标目录相同。

| 阶段 | 确切命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0／0 | 通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_api_logout -- --nocapture` | 101 | 0 通过、4 失败、11795 过滤。两库的占位路由均返回 501，预期分别为本地默认登出 204 和非法输入 400。 |
| 绿灯 | `cargo test -p vw-agent --lib auth6_api_logout -- --nocapture` | 0 | 4 通过、0 失败、11795 过滤。SQLite 与 PostgreSQL 各 2 项。 |
| 原生回归 | `cargo test -p vw-agent-apps --lib auth6_logout -- --nocapture` | 0 | 4 通过、0 失败、794 过滤。 |
| 全工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |

网关测试使用真实回环 HTTP、私有凭据文件、SQLite 和 Lima PostgreSQL 独立 schema。两库均验证 Codex 登出后仍可读取账号和绑定，默认值与凭据已清除，邻接租户的同名账号保持有效。DSH 拒绝撤销非官方 issuer 的 grant，之后可单独完成本地登出并保留其他 YAML 记录。Kimi 默认登出保留另一区域凭据，全部登出清除该凭据。Claude 验证真实测试子进程退出、私有目录及隔离钥匙串条目删除，流程随后可查询为 `cancelled`。

原生回归使用回环 OAuth 服务验证 DSH 请求形状、远端确认、业务拒绝与跳转拒绝，并检查远端未确认时本地 grant 仍已删除。公网账号端到端验收归后续批次。AGY 原生回归读写真实 macOS 钥匙串的隔离条目，验证未获槽授权时拒绝，以及登出后条目删除和同作用域记录失效。

测试链接器报告与红灯相同的 `__eh_frame` 大小提示。临时 PostgreSQL 已停止，槽位锁释放后保留构建缓存。
