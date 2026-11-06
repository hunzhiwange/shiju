# 任务 3：协议控制与生效确认

日期：2026-11-06，macOS / Darwin arm64。依赖提交 `f35edbf2b`（任务 2）和 `4fbab91fd`（任务 1）已具备；原生权限精确匹配允许列表仍为空，因此本任务不猜测任何供应商的权限选项，不宣称原生免审批实机通过。

## 实现与范围调整

- `client/access_policy.rs`：`confirmed_config_control` 将期望值保留在独立 pending 状态，经过真实 `session/set_config_option` 响应确认同一 config ID 与 currentValue 后才更新当前会话能力。远端拒绝、缺少选项、错误值和超时均保留可重试屏障；其他会话的响应不能覆盖当前能力。
- `confirmed_mode_control`：显式模式选择优先使用已公布的 Mode 类 config，验证支持值；配置拒绝后不回退 mode。没有 mode 配置时只发送已公布的 mode ID。`session/set_mode` 空响应只作为请求确认，不能作为权限生效回读；current_mode_id 由同会话 CurrentModeUpdate 更新。不会把模式名称或 Mode 分类当成原生完全访问证据。
- 请求超时生产值 30 秒，测试边界为 100 毫秒；测试代理在超时场景保持请求 pending，不调用真实外部模型。
- `protocol.rs` / `prompt_loop.rs`：未确认控制阻止本次 prompt，错误提示要求重试该控制；取消期间不再次恢复会话。
- 相邻必要调整：`state.rs` 保存期望值；`prompt_controls.rs` 让运行中的控制复用相同确认逻辑；`event_client.rs` 丢弃其他会话的延迟 config/mode 通知，避免通知切换 prompt 目标。原有其他类型通知的会话切换行为保留。需要这些调整是因为只修空闲 actor 路径仍会漏掉活动 prompt 与通知路径。
- 独立 `client/access_policy_tests.rs` 通过 `session_control_tests` 模块声明复用真实 ACP 双向 JSON-RPC 内存连接；测试 fixture 只替换外部进程。并未增加供应商权限映射、配置开关、安装或认证行为。

## 验证

所有测试使用默认 features；Clippy 使用全部 features。均动态原子领取共享槽位，同时设置 `CARGO_BUILD_BUILD_DIR` 与绝对路径 `CARGO_TARGET_DIR` 相同，避免仓库 build-dir 配置将中间产物写出槽位。

红测：槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。

- `cargo fmt --all`：首次受并行 desktop 修改造成的缺失 `system_settings_apps/list.rs` 影响退出非零；后续该相邻任务恢复后，全仓格式化退出 0。未修改该 desktop 文件或覆盖其改动。
- `cargo test -p vw-acp --lib acp_full_access_protocol -- --nocapture`：退出 101，有效测试 2，失败 2。断言分别复现“错误配置值被接受”和“空 mode 响应伪造当前值”，均为行为失败，不是编译失败。

最终验证：槽位 4，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。

| 命令 | 退出码 | 有效测试 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 不适用 |
| `cargo test -p vw-acp --lib acp_full_access_protocol -- --nocapture` | 0 | 10 通过 |
| `cargo test -p vw-acp --lib client::session_control_tests -- --nocapture` | 0 | 17 通过（含上述 10 条） |
| `cargo test -p vw-acp --lib client::protocol_tests -- --nocapture` | 0 | 20 通过 |
| `cargo test -p vw-acp --lib client::prompt_loop_tests -- --nocapture` | 0 | 9 通过 |
| `cargo test -p vw-acp --lib client::event_client_tests -- --nocapture` | 0 | 24 通过 |
| `cargo clippy -p vw-acp --all-targets --all-features -- -D warnings` | 0 | 不适用，无警告 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 不适用，无警告 |

最终补充回归继续动态领取槽位 4，绝对路径与上表一致。最终全仓 Clippy 前一次运行曾因并行 desktop 修改导致 `system_settings_agents/shared.rs::selected_entry` 暂时未使用而退出 101；该相邻任务自行修复后，重新运行原命令退出 0，没有修改该文件或压制告警。

脱敏协议摘要：`session/set_config_option(access, high)` 收到 `currentValue=default` 时拒绝且保留旧状态；远端 JSON-RPC error 和永不返回的请求均阻止下一次 prompt；成功重试确认 high 后解除屏障。mode 配置和传统 modes 同时公布时只发送 config 请求，拒绝后不发送 mode。传统 `session/set_mode` 空响应保持旧 currentMode，随后同会话 CurrentModeUpdate 才更新当前值；其他会话通知不切换 expected session。未知访问语义只报告 ClientOnly / UnverifiedControls，不发送猜测的原生权限请求。

审查：变更仅涉及 ACP 客户端控制、相邻运行状态/事件路径、测试 fixture 和本证据；未改支持矩阵与只读 plan.md。测试通过真实 ACP 序列化与双向连接，未启动第三方供应商 CLI，不构成第三方权限实机验收。

红测槽位 3、所有后续槽位 4 的 owner 和锁已释放，编译缓存保留。建议继续任务 4 的调用方与会话恢复集成。
