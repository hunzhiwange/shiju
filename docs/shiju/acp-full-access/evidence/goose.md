# Goose ACP 权限接入

日期：2026-11-06；依据 Goose `v1.57.0` 官方源码，tag tree `30bba95105e59bc3e7df624cf6b4d433690cfe12`。本次按用户最新要求不运行测试、不申请账号或模型凭据、不进行真实协议与工具执行验收。

## 源码确认（E2）

- [GooseMode](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose-provider-types/src/goose_mode.rs) 公布 `auto`、`approve`、`smart_approve`、`chat`；默认是 `auto`。
- [response_builder](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/acp/response_builder.rs) 生成会话 modes 和 `mode` 配置；[dispatch](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/acp/server/dispatch.rs) 实现 `session/set_config_option(mode=...)`，设置后返回当前配置；也支持 `session/set_mode` 通知。
- [server::on_set_mode](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/acp/server.rs) 调用 [Agent::update_goose_mode](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/agent.rs)，仅更新目标会话，并向支持更新的模型 provider 传递模式。客户端不修改 provider、model 或 thinking 配置，配置响应改变这些字段时拒绝继续 prompt。
- [PermissionInspector](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/permission/permission_inspector.rs) 的 `auto` 直接返回 Allow，跳过用户 `NeverAllow`；`approve` 才检查用户 allow/ask/deny。因此不自动选择 `auto`，也不允许通过通用会话控制切入 `auto/smart_approve`。权限开关不应取消用户拒绝规则。
- server 的权限请求先提供 `allow_always` 再提供 `allow_once`。客户端过滤永久允许项，避免把可撤销的 host 授权写成持久工具规则。

关键文件 SHA256：

| 文件 | SHA256 |
| --- | --- |
| `goose-provider-types/src/goose_mode.rs` | `d5b0ae31b4882085b572ca7b595f9153f595c417f797d4ef6a50bcba1e7ebdd7` |
| `goose/src/acp/response_builder.rs` | `9618d5b0594c0b766fa6265b9e35f1ce5cccbc2d5a5727fa02f8faa9eb9a1075` |
| `goose/src/acp/server/dispatch.rs` | `81b6d2d749920a992c1467e76289966580d4576f65a8cc08c8bdb55718ff9817` |
| `goose/src/permission/permission_inspector.rs` | `9a19a4ee725c1abfcb298a620214f89c6f6f4a8cd1f2b276d6caa3d10ee7db8c` |

## 生产行为（E1）

分类：**客户端自动批准**，不宣称已验证原生免审批。

- 普通/托管入口均保持 `goose acp`；CLI 自治模式的环境配置不当作 ACP server 权限接口，Goose 内部 ACP provider 也不当成当前 server。
- new/load/resume 后及每次 prompt 前，优先使用实际公布的 `mode=approve` 配置并确认响应；仅公布 legacy modes 时走现有 `confirmed_mode_control`。缺少权限控制的旧/自定义入口保留客户端审批；只有已知高权限无法恢复时明确拒绝。
- false→true→false 始终保留原生 `approve`，由当前 host `permission_mode` 控制每次客户端工具批准，仅选择 `allow_once`；不会留下本次生成的永久允许规则。既有用户显式 allow/deny 规则仍由 Goose 处理，不修改全局配置。
- 保留 `chat`，不会因权限开关悄然启用工具或更换模型。活动 turn 的手动模式变更先取消并要求完成后重试；同步失败保留 prompt 屏障。
- host `DenyAll` 在 prompt 前拒绝原生工具执行；客户端文件与终端沿用当前权限处理器。关闭完全访问不代表撤销用户独立配置的权限。
- cli_local/session_model 沿用原有驱动与版本门控，不新增认证、安装或平台能力声明。供应商硬拒绝和原生沙箱不由客户端批准覆盖。

## 验证记录

用户明确跳过测试驱动、自动化测试及账号实机验收；新增行为测试数 0，未运行 `cargo test`。无 initialize/new/load/resume/config 或工具结果的真实协议记录，macOS/Linux/Windows 工具效果均未实测。

- 实际槽位：2；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo fmt --all`：退出码 0，仅保留本任务必要差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0；全目标、全 features 静态检查通过，不执行测试。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 阶段进程已退出；本任务锁已释放，槽位缓存保留。
