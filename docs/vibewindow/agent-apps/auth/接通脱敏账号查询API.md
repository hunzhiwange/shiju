# 接通脱敏账号查询 API

母任务 6／批次 38。直接前置为批次 37 的 `eefd52cca`。开工范围以已验收的正式管理路由、私有主机策略、账号存储及批次 32–37 的账号和绑定事务为准。母任务 2 的领域收口、母任务 4 的客户端实机验收、母任务 5 的真实登录目录缺项仍按原记录处理。[接口契约](接口契约.md) · [前置验证](阻止撤销与刷新竞态.md)

## 管理链与查询

- 生产管理路由只注册 `GET /v1/agent-apps/auth/accounts`、`/status`、`/bindings`。沿用 Basic 校验，管理密码未配置或为空时返回 401。主体由受保护策略中的 `x-agent-apps-token` 摘要查得，必须在选定平台、公司、主机和服务上有专属 `auth_account_read` 授权。Bindings 还要求该 grant 的 `provider_config_read`。旧 grant 缺少专属字段时保持拒绝。
- 每项查询要求 `host_id`、`service`，仅 bindings 允许可选 `account_id`。例如 Codex 的只读 grant 使用 `"auth_account_read": ["codex_oauth"]`。账号和绑定只在完整作用域内查找。外部作用域的账号 ID 与不存在 ID 均返回 404。未知、重复及额外查询字段返回 400。成功与业务拒绝使用 `Cache-Control: no-store`。
- 文件型账号复用受管账号 v2 的持久化元数据和私有凭据读取，不列举旧启动身份。签出记录保留元数据，Codex 身份错配和 xAI 的永久失效标记显示为需重登。Kimi 的原生默认仅在显式槽位所有权下解析，供应商引用来自已提交事务。AGY 和 DSH 需要显式登记原生槽位所有者。AGY 读共享锁遇到进行中的独占操作时返回 `account_in_use`。DSH 账号列表沿用固定平台的当前用户检查，绑定筛选只核对本地 grant，不发送令牌。
- 响应使用公开账号 DTO、状态 DTO 和仅含 `provider_id`、`service`、`account_id` 的绑定记录。只读 grant 不授予启动、设默认、绑定或撤销能力，因此状态中的登录方式为空，三个写能力标志为 false。绑定筛选和状态不读取其他主机或回退到客户端。

## 验证

macOS 27.0 arm64，PostgreSQL 18 使用 Lima 的临时实例和独立 SSH 转发。各验证阶段动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。格式化在源码快照运行，只回写本批文件。

红灯以正式路由的 501 占位处理器验证。SQLite 与 PostgreSQL 各有两项行为断言失败，合法作用域实际 501、预期 200，只有供应商配置权限实际 501、预期 403。此前的 Codex 夹具身份格式和 PostgreSQL 转发端口冲突已修正，未计为红灯证据。

| 阶段 | 确切命令 | 退出码 | 有效结果 |
| --- | --- | ---: | --- |
| 红灯及最终 | `cargo fmt --all` | 0／0 | 格式化通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_api_read -- --nocapture` | 101 | 0 通过、4 失败、11769 过滤 |
| 最终 | `cargo test -p vw-agent --lib auth6_api_read -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 2 项通过，0 失败、11769 过滤 |
| 钥匙串 | `cargo test -p vw-agent-apps --lib auth6_agy_read -- --nocapture` | 0 | 1 通过、0 失败、797 过滤 |
| DSH | `cargo test -p vw-agent-apps --lib auth6_dsh_ -- --nocapture` | 0 | 5 通过、0 失败、793 过滤 |
| 登出回归 | `cargo test -p vw-agent-apps --lib auth6_logout -- --nocapture` | 0 | 4 通过、0 失败、794 过滤 |
| 最终 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区通过 |
| 最终 | `git diff --check`、`git diff --cached --check` | 0／0 | 差异检查通过 |

绿灯通过真实回环 TCP 请求正式管理路由，使用私有临时策略、受管凭据文件和实际供应商绑定事务。响应逐字段核对，覆盖默认和已登出状态、绑定筛选、同租户不同主体隔离、跨公司／主机／服务拒绝、认证关闭时拒绝、只读 grant 与绑定权限分离、非法字段和路由别名缺失。SQLite 使用临时数据库，PostgreSQL 使用独立测试 schema。

AGY 使用真实 macOS 钥匙串及隔离测试项，验证默认匹配、邻接作用域隔离、拒绝未授权槽位及独占锁争用。DSH 使用真实临时文件和回环上游替身，验证本地筛选及外部 issuer 拒绝。生产账号、外部部署和桌面调用仍按后续批次验收。测试链接报告既有的 macOS `__eh_frame` 体积提示，测试进程正常退出。

临时 PostgreSQL 与 SSH 转发已停止，槽位锁已释放，编译缓存保留。
