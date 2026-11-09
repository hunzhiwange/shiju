# macOS 真实网关授权链路验收

母任务 6／批次 56。沿用批次 55 提交 `d46b48eed` 和双库结果，核对了来源快照、原生包、认证及执行主机、双库存储、供应商事务和模型目录底座。母任务 2 按已验收的底座子计划交接，完整领域迁移另行收口。[批次 55 验收](回归凭据脱敏与失败诊断.md) · [来源及前置映射](来源映射.md) · [底座验收](../底座交付映射.md) · [供应商验收](../../../shiju/agent-apps/provider-macos-acceptance.md) · [模型目录验收](../../../shiju/model-catalog/macos-acceptance.md)

## SDK 到执行主机

- macOS 27.0（26A428），arm64。使用生产 SDK、真实回环 HTTP、正式网关路由及 Basic／主体／租户策略、SQLite 临时数据库和 Lima 中的 PostgreSQL 18 临时实例。唯一替代的业务 I/O 是 Codex 公网 OAuth。夹具逐项核对设备码申请、轮询及表单兑换的请求，并记录调用顺序。网关仅在测试构建中选择回环 OAuth 地址，适配器限制为回环且关闭代理与重定向。挑战链接保持官方 HTTPS 地址，测试未打开浏览器。
- 启动返回公开挑战，后台轮询及兑换使账号进入 `ready`，从主机回读受管 Codex 凭据文件，确认写入的合成 access 和 refresh token 与夹具一致。公开启动及轮询响应不含设备私有 ID、兑换凭据或令牌。经 SDK 绑定官方供应商并设置默认账号。
- 关闭 HTTP 服务器，重新打开数据库并建立新路由运行时。SDK 回读到同一账号、默认及绑定。旧流程返回 404 `not_found`。删除被引用账号返回 409 `account_in_use`，凭据仍存在。默认登出后账号保留为 `signed_out`、不再标为默认、绑定保留、受管凭据已移除。解除绑定并删除后，SDK 查询不到账号和绑定。
- 远端主机无执行通道返回 501 `execution_channel_unavailable`，未触发 OAuth 或创建主机文件。另一个获准主体取消本主体的流程返回 404 `not_found`。所有者取消成功后，过轮询间隔查询仍为 `cancelled`，上游无轮询或兑换，账号、绑定和凭据根均为空。
- 真实钥匙串回归使用随机测试命名空间，验证不同账号隔离、创建、覆盖、读回、删除及重复删除，检查非法条目标识和超长载荷拒绝后保留原值。真实供应商登录由批次 57–63 分别验收。[钥匙串适配与 ACL 证据](macOS钥匙串存储.md)

## 验证

在 `42689b4a0` 的隔离源码快照上执行。沿用工作区已有的 PostgreSQL 会话测试签名修正（为 `load_session_meta` 补传 `false`），该修正属于独立任务。格式化回写限于批次 56 的 Rust 文件。红灯仅在快照中让获授权的 Codex 设备流启动显式返回 `unsupported`。四个用例均执行并在 SDK 应收到公开挑战的断言处失败，实际为 HTTP 501。绿灯恢复正式接线。

各阶段均动态领取槽位 1。`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 使用同一绝对路径。结束后释放任务锁并保留缓存。

| 阶段 | 确切命令 | 退出码 | 有效结果 |
| --- | --- | ---: | --- |
| 红绿源码格式化 | `cargo fmt --all` | 0／0 | 通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_integration -- --nocapture` | 101 | SQLite 与 PostgreSQL 各 2 项在启动断言处失败，共 0 通过、4 失败、11825 过滤 |
| 绿灯 | `cargo test -p vw-agent --lib auth6_integration -- --nocapture` | 0 | SQLite 与 PostgreSQL 各 2 项通过，共 4 通过、0 失败、11825 过滤 |
| 真钥匙串 | `cargo test -p vw-agent-apps --lib auth6_keychain -- --nocapture` | 0 | 2 通过、0 失败、797 过滤 |
| 设备适配回归 | `cargo test -p vw-agent-apps --lib auth6_codex_device -- --nocapture` | 0 | 2 通过、0 失败、797 过滤 |
| 全工作区 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |

网关测试链接阶段出现既有的 macOS `__eh_frame` 大小提示。
