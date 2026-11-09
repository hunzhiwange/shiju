# 启动授权与 Claude 回填 API

母任务 6／批次 39。直接前置为批次 38 的 `e784fb343`。开工采用已验收的管理认证、可信主机策略、受管存储及供应商事务接口。母任务 2 的领域收口、母任务 4 的客户端实机验收与母任务 5 的真实登录目录缺项仍按原验收记录跟踪。[查询 API](接通脱敏账号查询API.md) · [接口契约](接口契约.md)

## 管理路由与权限

- `POST /v1/agent-apps/auth/flows` 接收共享启动 DTO，返回 201 及一次性挑战。`POST /v1/agent-apps/auth/flows/{flow_id}/paste` 接收 `{pasted}`，仅向当前作用域中仍活跃的 Claude CLI 交付，成功返回 202。两条路由共用既有管理 Basic、租户解析和 `Cache-Control: no-store`，独立于 skey 配对和模型中继。标准管理监听和 CLI／守护进程网关装配使用同一管理路由。
- 私有策略 grant 通过 `"auth_flow_start": ["claude_official"]` 之类的服务列表显式授权启动及本流程的回填。缺省为空。主体从 `x-agent-apps-token` 摘要解析，授予范围为平台、公司、主机与服务。AGY、DSH 还要求部署显式登记对应的原生槽位所有者。离线主机返回 `execution_host_unavailable`，在线但无执行通道返回 `execution_channel_unavailable`。
- Codex、AGY、DSH 的浏览器方式需要部署在同一个 grant 设置 `"auth_browser_local": true`，并通过服务器观察到的直接回环连接访问。此字段仅适用于能保证调用端浏览器与执行端同机的部署，带转发头的连接不能用来证明该条件。回环不可达返回 `callback_unreachable`。Claude 的浏览器和复制链接方式均支持用户主动回填。
- Copilot 缺省使用 github.com。企业 authority 仅可由客户端从执行主机策略的 `github_enterprise_authorities` 选择，配置使用规范化 authority。Codex 和 Kimi 定向重登先查同作用域元数据。Kimi 新登录需要地区，定向重登的显式地区冲突返回 `invalid_input`。未知值、额外字段和不适用参数拒绝。支持矩阵与公开 DTO 沿用批次 3、4 合同。

## 执行与秘密

执行主机复用 `AuthFlows`、各服务适配和原有事务推进授权。状态机按服务间隔限制设备码兑换，浏览器回调由执行主机监听。DSH 沿用只读接口的 `en-US` 与 UTC 缺省值。Claude 从已授权的主机搜索路径定位程序，为新账号分配独立目录，回填交给原进程。CLI 退出且身份可验证后，才在同作用域发布账号元数据。确认未发布的失败尝试清理临时进程和凭据，钥匙串身份无法确认时保留其隔离项。挑战响应只包含公开 DTO 字段。回填值、流程 ID、上游请求和原始解析错误不写入应用日志。

生产账号、真实 OAuth 提供方和桌面交互留给后续验收。

## 验证

macOS 27.0 arm64。红灯、绿灯和回归阶段均动态取得槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。格式化在源码快照执行，仅回写本批文件。PostgreSQL 18 使用 Lima 的临时服务器和独立回环转发。

| 阶段 | 命令 | 退出码 | 有效结果 |
| --- | --- | ---: | --- |
| 红灯及绿灯 | `cargo fmt --all` | 0／0 | 格式化通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_api_start -- --nocapture` | 101 | 2 项预期失败，11773 过滤 |
| 绿灯 | `cargo test -p vw-agent --lib auth6_api_start -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 2 项通过，0 失败、11773 过滤 |
| 查询回归 | `cargo test -p vw-agent --lib auth6_api_read -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 2 项通过，11773 过滤 |
| Claude 回归 | `cargo test -p vw-agent-apps --lib auth6_claude_ -- --nocapture` | 0 | 14 项通过，784 过滤 |
| 钥匙串回归 | `cargo test -p vw-agent-apps --lib auth6_keychain -- --nocapture` | 0 | 隔离测试命名空间读、写、删和拒绝路径 2 项通过，796 过滤 |
| 工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 101 | 并行桌面变更使 `settings_row_layout` 成为未使用导入 |
| 隔离检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 基线 `6e88afc0e` 加本批完整 Rust 差异，全工作区通过 |
| 差异检查 | `git diff --check` | 0 | 工作区差异检查通过 |

红灯使用已注册的 501 占位处理器。合法 Claude 启动实际 501、预期 201。只有账号读取权限的主体实际 501、预期 403。第一次绿灯编译报告的夹具变量与类型错误已修复，行为结果仅采用成功编译后的测试。

绿灯经真实回环 TCP、管理认证、临时凭据目录和实际数据库执行。脚本进程替代外部 Claude 登录程序，核对原 CLI 参数和隔离目录，收到回填后写入来源形状的模拟材料。账号回读为 `ready`，账号 ID 与流程 ID 独立，凭据不出现在响应中。另一主体的回填返回 404，终态再次提交返回 409。拒绝用例覆盖只读 grant、未知服务／方式、额外字段、错误域、跨主体 Codex 目标、Kimi 地区冲突、缺少回环保证、远端执行通道缺失、离线主机和未知流程。测试进程正常退出，链接器仍给出既有 `__eh_frame` 大小提示。

临时 PostgreSQL 与 SSH 转发已停止。本批槽位锁已释放，编译缓存已保留。
