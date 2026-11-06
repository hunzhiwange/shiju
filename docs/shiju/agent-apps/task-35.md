# 任务35：应用设置API

状态：已完成。2026-11-06，macOS。

## 依赖与范围

核对任务34生命周期API和任务27设置归一化的真实实现，复用可信管理员授权文件、TenantScope、GlobalDatabase、GatewaySettingsStore及vw-agent-apps::apps::preferences。不修改只读plan.md与来源快照；未重建设置JSON文件、执行主机RPC或设置规则。RustCodeGraph status可用，explore长时间无返回后终止该查询，使用实际文件核对新增/未覆盖实现。

## 接口和安全边界

生产handler_router→agent_apps::router合并GET/PATCH /agent-apps/preferences?targetHostId=…（及既有/v1前缀）。沿用外层Basic/TenantScope和x-agent-apps-token的可信管理员文件授权；目标必须已授权、在线且绑定本地，远程仍明确501。每次重新读取授权文件。

公开请求DTO为vw_api_types::agent_apps::AppPreferencesPatch，响应为已有AppPreferencesResult。PATCH每次替换一个完整域，其他域保留；不接受客户端用户ID、命令、文件路径或任意字段。选择单域替换是为复用现有单行原子upsert，避免两域部分写入，不新增事务层或第二套存储。

```json
{"scope":"user","value":{"appOrder":["codex","claude"],"visibleApps":["codex","claude"]}}
```

```json
{"scope":"host","value":{"preferredTerminal":"terminal","wslShells":[]}}
```

用户数据在既有gateway_settings中使用agent_apps_user命名空间，键为管理员文件principal.id；不使用客户端声明的身份，不以token作为持久化键。主机数据使用agent_apps_host命名空间和已授权host.id。两域均继续由TenantScope隔离平台和公司。用户偏好跨该用户已授权主机共享，主机终端设置在同一租户主机的用户间共享。稳定principal.id应由管理员保持；令牌轮换不改变设置归属。

持有既有读取grant的主体可保存自身偏好；修改共享主机配置额外要求该grant显式preferences_write=true，省略为false，既有actions权限不隐式获得配置写权限。此字段只由现有可信管理员文件设置，无HTTP认领入口。不执行重启、终端或其他子进程。restartRequired仅在该次主机配置发生规范化后实际变化时为true，GET和重复保存为false；它是即时建议，不是跨重启的待办标记。

归一化复用任务27规则，真实本地主机决定平台，拒绝无效终端/非Windows WSL设置和空可见集，过滤过期顺序ID与重复项。不存在设置时返回归一化默认值，不写入；读取损坏的JSON或数据库故障失败关闭。无UI变更，无需渲染证据；macOS用例不冒充其他平台实机证据。

## 验证

红灯：cargo test -p vw-agent --lib cc_apps_preferences_api_scope -- --nocapture，退出101，2个有效测试失败（503对比期望200/401），不是编译失败或零测试。未实现处理器骨架先直接返回503，随后启用真实存储和授权行为。


所有Cargo阶段动态mkdir领取slot-2，记录任务/会话/PID；生产验证阶段以EXIT trap清理自身记录和锁，全部Cargo及测试子进程结束后释放，缓存保留。读取.cargo/config.toml发现build.build-dir="target"，显式覆盖两个环境变量：

- 实际槽位：slot-2（红灯、通过与最终回归阶段均动态获得此槽）。
- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

Rust修改后先执行cargo fmt --all，退出0，审查无无关格式化差异。

| 确切命令 | 退出码 | 有效测试数 |
| --- | --- | --- |
| `cargo test -p vw-agent --lib cc_apps_preferences_api_scope -- --nocapture`（红灯） | 101 | 2失败，11460过滤 |
| `cargo test -p vw-agent --lib cc_apps_preferences_api_scope -- --nocapture`（通过） | 0 | 2通过，11460过滤 |
| `cargo test -p vw-agent --lib cc_apps_read_api_authorization -- --nocapture` | 0 | 7通过，11455过滤；另1个生产路由子进程不重复计数 |
| `cargo test -p vw-agent --lib cc_apps_lifecycle_api_scope -- --nocapture` | 0 | 5通过，11457过滤 |
| `cargo test -p vw-agent-apps --lib cc_apps_preferences_normalization -- --nocapture` | 0 | 4通过，156过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓通过，无Clippy诊断 |
| `git diff --check`、`git diff --cached --check` | 0 | 无空白错误 |

18个顶层有效通过用例。当前API两用例使用真实临时SQLite和可信授权文件，覆盖保存后新建路由/重开数据库读取、用户偏好跨已授权主机共享、不同主体偏好隔离、同主机配置共享、主机配置隔离、已授权另一租户默认值隔离、重复/过期排序ID归一化、主机变化与重复保存重启提示、错误令牌/跨租户/未知主机/缺主机写权限/空可见集/无效终端/伪造用户ID拒绝，拒绝后原默认设置不变及授权文件损坏失败关闭。

仅既有macOS链接器__eh_frame提示仍出现，测试真实退出0；未改变链接设置，不将该历史提示当作新增Clippy问题。没有运行无过滤全量cargo test、fmt检查、wasm检查或Makefile。

差异只包含设置处理器、独立测试、最小模块路由、公开PATCH DTO、授权grant字段及本文档；现有任务1–34的编号文件删除不纳入提交。任务35编号文件完成后删除，不提交单纯状态变更。后续建议任务36/SDK复用此接口和DTO，主机终端启动应读取相同命名空间，而非再建立设置文件。
