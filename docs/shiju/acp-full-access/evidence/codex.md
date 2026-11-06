# Codex 完全访问：固定制品验收

日期：2026-11-06（Asia/Shanghai）。平台：macOS arm64。依赖任务 4 提交 `789196df4` 已满足。

## 身份与源码确认（E2）

- 官方 npm：[`@agentclientprotocol/codex-acp@2.5.0`](https://registry.npmjs.org/@agentclientprotocol%2fcodex-acp/2.5.0)，gitHead `34e08970aa324a2be985f5080674d5aa91d7666f`。在独立临时目录安装，不升级用户 CLI。
- 原始 `dist/index.js` SHA256：`15de6b2dd5203876a48b12861378bcf87040d623ab2159e05052dc9d54c32b16`。
- 应用仓库已有 `patches/acp-model-source/codex/codex-acp-2.5.0-model-source.patch` 后 SHA256：`730bff93eddd77ad1ea1c3ff941ee5fa4fe2b9cb940449e6ee82712295846136`，与 `managed_runtime/contracts.json` 完全一致。没有修改该补丁或安装契约。
- 底层 `@openai/codex@0.165.0`，实际输出 `codex-cli 0.165.0`；Darwin arm64 引擎 SHA256：`a45099154d7fb4a5ed8a56f40966a95deb938e508bab23e6985035c709e9a85a`。Node 26.13.0。
- 固定提交的 [AgentMode.ts](https://github.com/agentclientprotocol/codex-acp/blob/34e08970aa324a2be985f5080674d5aa91d7666f/src/AgentMode.ts) 定义 `mode=agent-full-access` → `approvalPolicy=never`、`sandboxPolicy.type=dangerFullAccess`；`mode=read-only` → `on-request`、`readOnly`。`set_config_option` 回读配置，`sendPrompt` 在每个用户 turn 将模式映射传给原生引擎。模式改变不会重新选择模型或协作模式。

## 真实进程与工具结果（E3）

使用隔离项目、私有 HOME/CODEX_HOME、一次性测试凭据和 loopback Responses SSE 上游。上游只返回固定 `exec_command`（在临时项目写入 `probe-result.txt`）及结束消息；没有调用真实模型，没有读取用户凭据。代理和 Codex 引擎均为上述真实制品。

| 路径 | 新建 false | 同 ID 跨进程 load true | 同 ID 跨进程 resume false |
| --- | --- | --- | --- |
| 已核验 ACP 入口，cli_local | read-only；写入失败 | agent-full-access；写入成功；Native | read-only；写入失败 |
| 已核验 ACP 入口，session_model 启动材料 | read-only；写入失败 | agent-full-access；写入成功；Native | read-only；写入失败 |

仓库真实 `AcpClient` 执行上述两条路径：会话 ID 保持不变，每次首次 prompt 前完成配置响应确认。session_model 路径使用驱动相同的 `cli --shiju-model-source`、私有路由、模型锁及持久历史环境；本次验证权限，不重复测试 gateway 模型绑定或真实供应商鉴权。两条路径都固定 `gpt-6.1-sol`，未更换模型、协作模式或输入提示词。

另用原始和补丁后适配器各完成同进程 `read-only → agent-full-access → read-only`：写入依次失败、成功、失败；真实 `initialize` 返回协议 1、适配器名称与版本、load/resume 能力、文件和终端客户端能力。`session/new` 公布 `mode` 以及四个精确值 `read-only / workspace-write / agent / agent-full-access`。直接 load/resume 回到 `agent`，证明不能将历史模式当成当前权限。

为分别核验审批和沙箱，增加透明引擎转发探针，仅记录 `turn/start` 的权限字段和模型，不记录请求载荷或令牌。真实引擎收到的三次用户 turn 如下（额外标题生成 turn 不计入）：

```json
{"approvalPolicy":"on-request","sandboxPolicy":{"type":"readOnly","networkAccess":false},"model":"gpt-5"}
{"approvalPolicy":"never","sandboxPolicy":{"type":"dangerFullAccess"},"model":"gpt-5"}
{"approvalPolicy":"on-request","sandboxPolicy":{"type":"readOnly","networkAccess":false},"model":"gpt-5"}
```

该透明探针通过 CODEX_PATH 转发到真实固定引擎；生产策略会拒绝将自定义 CODEX_PATH 认定为已核验原生能力。此探针只提供底层协议证据；仓库客户端的 Native 验收使用默认固定引擎解析路径。

原始/补丁后适配器三次工具尝试的 `session/request_permission` 均为 0：只读执行受到 OS 沙箱拒绝，完全访问执行成功。本例由 Codex 原生终端写文件，不走 ACP `fs/write_text_file` 或 `terminal/create`，这两类客户端回调计数均为 0；不能用零回调宣称已验收客户端代执行。独立原生文件编辑工具、网络和企业托管策略未另做实机验收。

## 实现与明确边界

- `access_policy/codex.rs` 仅匹配已核验的精确配置 ID/值。`client/protocol.rs` 同时检查 macOS arm64、agentInfo 固定名称/版本、现有托管制品与引擎版本校验，以及有效 CODEX_PATH/NODE_OPTIONS/NODE_PATH 未覆盖运行时。普通启动和托管启动均可使用已核验入口；npx latest、未核验本地/自定义入口和其他平台保留 ClientOnly，不冒充 Native。未知运行时恢复后若仍公布当前完全访问而当前授权不足，因无法安全撤销而明确拒绝继续，不猜测控制。
- `client/access_policy.rs::sync_codex_access` 在 new/load/resume 及每次 prompt 前按当前授权重新同步：仅 `full_access && ApproveAll` 选择完全访问；关闭或 ApproveReads 使用只读模式。缺少公布值、远端拒绝、超时、错误回读均保留屏障，不能继续 prompt，也不回退旧 mode 接口。
- Native 必须有当前运行时身份、同会话成功确认和匹配的当前配置快照。仅公布模式或传入伪造快照不产生 Native；进程退出和重新握手撤销旧确认。
- 显式 config 或 legacy mode 的完全访问控制同样服从当前授权；DenyAll 无法覆盖，且在创建/恢复 prompt 会话前拒绝 Codex 原生执行。标准客户端审批逻辑未改写，默认客户端自动批准仍与原生免审批分别记账。
- Codex 模式仅影响后续 turn。活动 turn 切换模式时请求取消并明确返回不支持即时撤销；取消完成后重试，不将配置成功当作已经撤销活动工具权限。新进程恢复继续使用原逻辑会话。
- 没有企业授权或硬拒绝绕过：只使用原生会话控制，不添加全局配置、命令行全局绕过、登录修改或自动升级。问答、计划确认与供应商/OS 强制拒绝不属于本次取消范围。企业策略没有实机证据，不能承诺覆盖或解除。

必要相邻修改限于客户端工厂状态、会话创建/恢复、prompt 前屏障及模式控制；只增加后端匹配函数无法触达实际生产路径。测试复用现有双向 ACP JSON-RPC fixture，只替换外部代理进程与固定响应，真实供应商证据单列如上。

## 验证记录

所有 Cargo 阶段均原子动态领取共享槽位，设置 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 为同一绝对槽位路径，先运行 `cargo fmt --all`。默认测试 features；Clippy 使用全部 features。路径前缀为 `/Users/xiangmin/worker/code/shiju/target/`。最终三项检查的完整 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。

| 阶段与确切命令 | 槽位 / CARGO_TARGET_DIR 后缀 | 退出码 / 有效测试 |
| --- | --- | --- |
| 红测：`cargo test -p vw-acp --lib acp_full_access_codex -- --nocapture` | 2 / rust-slot-2 | 101；2 条行为断言失败：未授权 full mode 和 DenyAll 均到达远端 |
| 补充红测：`cargo test -p vw-acp --lib acp_full_access_codex_unverified_elevated_restore -- --nocapture` | 3 / rust-slot-3 | 101；1 条行为断言失败：未知运行时的已提权恢复继续执行 |
| 最终：`cargo test -p vw-acp --lib acp_full_access_codex -- --nocapture` | 3 / rust-slot-3 | 0；12 条通过 |
| 相邻：`cargo test -p vw-acp --lib client::session_control_tests -- --nocapture` | 3 / rust-slot-3 | 0；29 条通过（含 12 条 Codex 行为测试） |
| `cargo clippy --all-targets --all-features -- -D warnings` | 3 / rust-slot-3 | 0；无警告 |
| 真实客户端：`cargo run -p vw-acp --example codex_full_access_probe --` 和 `cargo run -p vw-acp --example codex_full_access_probe -- model-source` | 2 / rust-slot-2 | 均为 0；两条路径各 3 个有效工具结果 |

真实进程验收 harness 位于临时目录 `/tmp/shiju-full-access-codex5`；临时 Cargo example 验收后删除，不纳入产品。原始/补丁后协议探针为 `python3 .../probe.py`；权限字段探针为 `python3 .../tap-probe.py`，均退出 0。仅供应商受控 smoke 验收使用这些临时脚本；最终 harness 源码副本保留为 `/tmp/shiju-full-access-codex5/codex_full_access_probe.rs`，最终结果为 `final-replay3.log`。12 条确定性回归保存在 `access_policy/codex_tests.rs` 并经 `client/session_control_tests.rs` 声明接入。

中间失败已调查：首次全仓 fmt 因其他任务暂缺 desktop `repository_ops_tests.rs` 退出 1，后续恢复后 fmt 退出 0；新增 fixture 的借用生命周期编译错误已修复，不计红测；相邻回归发现本次能力查询锁与已有持锁测试发生重入，已将非 Codex 或未授权查询提前返回并完整重跑；首版真实恢复 harness 漏设 MODEL_PROVIDER，触发适配器默认 provider 的认证错误，补齐驱动已有环境变量后通过，未更改产品模型路由。扩展最终 resume harness 时，cli_local 测试路由增加了 env_key 却遗漏测试令牌，工具结果断言正确失败；补齐一次性令牌并重新执行两条完整路径后均通过。

审查：未修改 plan.md、安装、模型来源实现、品牌、认证、全局权限或其他任务文件。槽位占用记录在各阶段结束后删除，编译缓存保留。下一步继续任务 6；其他平台、未知入口及企业策略如需发布能力，必须独立补齐证据。
