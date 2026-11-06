# 任务 2：权限意图与能力判定

日期：2026-11-06，macOS / Darwin arm64。依赖提交 `4fbab91fd` 已提供 22 项静态支持矩阵；没有原生权限 E2/E3，不能把候选 mode/config 值当成原生支持。

## 实现与审查

- `crates/vw-acp/src/access_policy.rs`：`AccessPolicy` 将显式 `full_access_requested`、`PermissionMode`、`FullAccessSupport` 和非敏感 `AccessPolicyReason` 分开。宿主 `DenyAll` 优先于会话选择；没有完全访问选择时保留 `ApproveAll` 默认审批语义，不自动扩展文件边界。
- `crates/vw-acp/src/client/builder.rs`：`AcpClient::access_policy` 接收当前会话已解析的 ACP `SessionModeState` / `SessionConfigOption`，提供只读判定。没有供应商能力开关，也不修改启动参数或全局配置。
- 原生精确匹配允许列表目前为空：本任务不猜测 `auto`、显示名、描述、布尔配置或自定义命令。只有客户端自动批准机制有证据；`Native` 不会被本任务返回。后续供应商任务须以固定版本证据实现精确匹配及协议回读，任务 3 负责同步控制。
- `permissions.rs`：`ApproveAll` 仅选择 `AllowAlways` / `AllowOnce`；缺失允许选项时返回 ACP `Cancelled`，删除 `options[0]` 回退。`DenyAll` 保持选择拒绝项、否则取消的语义。
- 独立测试模块 `access_policy_tests.rs` 已接入；`permissions_tests.rs` 覆盖实际请求解析、选项选择和结果分类。未改安装、认证、模型来源、注册表和其他任务文件。`plan.md` 保持只读。

## 验证证据

所有 Cargo 命令均使用默认 features（Clippy 明确使用所有 features）。按原子 mkdir 动态领取槽位，记录持有 PID，阶段结束删除自己的 owner 和锁目录，保留缓存。

初次红测和初次绿测：槽位 4，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。发现仓库 `.cargo/config.toml` 的 `build-dir = "target"` 会令测试中间产物落到槽位外，因此最终阶段额外设置 `CARGO_BUILD_BUILD_DIR` 与 `CARGO_TARGET_DIR` 相同，不修改仓库配置；最终验证以该阶段为准。

红测：`cargo fmt --all` 退出 0；`cargo test -p vw-acp --lib acp_full_access_policy -- --nocapture` 退出 101，有效测试 1，失败 1。`acp_full_access_policy_approve_all_cancels_without_allow_option` 在断言 ACP `Cancelled` 处失败，原实现选中拒绝选项，并非编译失败或零测试。

最终阶段：动态领取槽位 3；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，`CARGO_BUILD_BUILD_DIR` 同路径。

| 命令 | 退出码 | 有效测试 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 不适用 |
| `cargo test -p vw-acp --lib acp_full_access_policy -- --nocapture` | 0 | 10 通过，0 失败 |
| `cargo test -p vw-acp --lib permissions::permissions_tests -- --nocapture` | 0 | 29 通过，0 失败 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 不适用；无警告 |

协议摘要：通过 ACP schema 构造、解析权限请求，验证空选项、只有拒绝选项、允许项位于拒绝项之后和宿主拒绝。策略测试覆盖缺省意图、构造顺序、仅读、未知后端、自定义命令、未知 mode/config 和只有拒绝 mode。未启动供应商 CLI，未声明原生免审批或外部版本实机验收通过。

最终阶段退出 0；槽位 3 的 owner 与锁已释放，槽位 4 的先前阶段锁也已释放。格式化未引入本任务范围之外的新增文件差异；提交只暂存上述策略、权限测试及本证据。建议下一步执行任务 3 协议控制与生效确认。
