# 接通默认账号 API

母任务 6／批次 43。直接前置为批次 42 的 `42730d971`，默认切换事务来自批次 34 的 `c0868fc9a`。沿用已交付的原生包、管理鉴权、主机合同、凭据与供应商事务。[接口契约](接口契约.md) · [切换默认授权账号](切换默认授权账号.md) · [底座交付映射](../底座交付映射.md)

`PUT /v1/agent-apps/auth/accounts/{account_id}/default?host_id=…&service=…` 无需请求体，成功返回空的 204。现有管理授权子路由在 `agent_apps_auth_read.rs` 装配该端点，继承 Basic 认证、可信主体令牌、租户校验和 `Cache-Control: no-store`。管理员在对应平台、公司、主机的 grant 中用 `"auth_default_write": ["github_copilot"]` 等服务列表授予写权限。状态查询同时检查该权限、服务能力和原生槽归属，再返回 `can_set_default`。

- Copilot、Codex 和 xAI 沿用受管凭据租约与租户数据库默认引用。
- AGY 和 Kimi 要求 macOS 及管理员明确登记的原生槽所有者。AGY 复用钥匙串切换。Kimi 从本机配置中匹配唯一的同作用域绑定或完全匹配的凭据，重新核验账号身份并通过现有事务发布原生模型选择。多个匹配返回 `conflict`，兼容槽缺少匹配凭据返回 `reauth_required`。
- Claude 和 DSH 返回 `unsupported`。已授权作用域内跨租户、主机或主体的账号 ID 与不存在的 ID 一样返回 `not_found`。原生错误沿用稳定码和安全响应，仅提供关联 ID。

## macOS 验证

使用真实回环 HTTP、私有凭据目录、SQLite 文件和 Lima 中临时 PostgreSQL 18.6 的独立 schema。Kimi 的文件与事务使用临时主目录，公网身份和刷新在原生回归中替换为校验请求形状的回环响应。AGY API 查询系统钥匙串中随机生成的不存在条目，写入、回读、拒绝和清理由隔离钥匙串项的原生回归覆盖。

首次红灯在 SQLite 两项行为断言中均得到占位 501，分别预期 204 和 400。PostgreSQL 测试因同步客户端在 Tokio 内初始化而中止。使用同一红灯二进制单独复现两项 SQLite 断言。绿灯将同步数据库夹具移到运行时外。

红灯及重放动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。绿灯动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 阶段的 `CARGO_BUILD_BUILD_DIR` 与对应目标目录相同。格式化在源码快照执行，只回写本批 Rust 文件。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、绿灯 | `cargo fmt --all` | 0／0 | 通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_api_default -- --nocapture` | 外层脚本 2 | SQLite 两项达到预期失败断言，PostgreSQL 夹具在行为断言前中止。外层脚本读取到运行期间的编辑，未捕获 Cargo 退出码。 |
| 红灯重放 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-9c99ea608e0487b2" auth6_api_default --skip postgres --nocapture` | 101 | 0 通过、2 失败、11790 过滤 |
| 绿灯 | `cargo test -p vw-agent --lib auth6_api_default -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 2 项通过，0 失败、11788 过滤 |
| 原生默认与钥匙串 | `cargo test -p vw-agent-apps --lib auth6_default -- --nocapture` | 0 | 2 通过、0 失败、796 过滤 |
| Kimi 原生默认 | `cargo test -p vw-agent-apps --lib auth6_kimi_account -- --nocapture` | 0 | 2 通过、0 失败、796 过滤 |
| Kimi 绑定回归 | `cargo test -p vw-agent-apps --lib auth6_binding -- --nocapture` | 0 | 2 通过、0 失败、796 过滤 |
| 授权路由回归 | `cargo test -p vw-agent --lib auth6_api_ -- --nocapture` | 0 | 22 通过、0 失败、11770 过滤 |
| 全工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |

macOS 链接器仍提示既有的 `__eh_frame` 大小。临时 PostgreSQL 已停止，各阶段槽位锁已释放，编译缓存已保留。
