# Corust 原生技能与会话挂载

2026-11-20，Darwin arm64。验证固定 Corust 0.6.0、受管 `model-source-v4` 以及模型 `coexistence-fixture/deepseek-v4.1-flash-1017` 的 `session_model` 绑定。受管启动器 SHA-256 为 `c8100fae6241e111ef7e9d6b2db5d9374007f2c1920a0bf4f4a202ab616a6df2`，原生二进制为 `a2a644a851ee03c583ed9f113cc3acbf54d99ff9f8aaaa782535dbd576d97a2e`。启动器、原生二进制和两个桥接文件均按受管合约校验。

## 接线

项目原生技能留在 `.corust-agent/skills`。宿主在经过制品审核的请求私有 HOME 中创建 `.agents/skills`，按项投影受信视图里的技能链接。Corust 原生扫描器列出名称、描述和已规范化的文件路径，模型随后调用 Corust 的 `read_file` 读取正文和附件。固定版扫描器优先保留同名项目技能，真实进程对照确认了该顺序。

视图整目录链接无法在原有沙箱中发现，逐项链接可以。当前入口只发布沙箱已允许访问的工作区内选集，工作区外来源明确拒绝。`cli_local`、禁用工具及其它平台仍受原有策略或能力门禁约束。制品、沙箱和审批配置保持原样。

## 修复前网关基线

旧网关 SHA-256 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。无挂载会话 `ses_e0728bfe6ffeDuY7HyoFZ9B1nH` 有 3 次真实模型请求、1 个 `chat.done`、0 个 `chat.error`，模型回读 `session_model` 且完全访问请求为 false。正文和附件的调用 ID 分别为 `call_106366b4e508efbeb6b5c2d7` 和 `call_4201dd5cd7f97a9a44e7e7db`。挂载轮首个模型请求前失败，0 次模型请求、0 个 `chat.done`、1 个 `chat.error`。探针退出 1。

## 固定受管进程对照

私有隔离的 Corust ACP 和 RustCodeGraph 进程完成以下四次新会话。回环模型仅在真实目录和工具声明出现后发出读取调用，并按模型调用 ID 核对返回正文、附件和实际索引源文件中的随机值。四轮均以 `end_turn` 结束。此对照直接发 ACP 帧，并把已验证的目录形态配置到请求目录。

| 场景 | 模型请求 | 原生正文与附件 | 选中正文与附件 | MCP 审批 | 完成的图谱调用 |
| --- | ---: | --- | --- | --- | ---: |
| 无挂载 | 3 | 通过 | 不可发现 | 无 | 0 |
| 挂载 | 6 | 通过 | 通过 | 单次批准 | 1 |
| 撤除 | 3 | 通过 | 不可发现 | 无 | 0 |
| 拒绝 | 6 | 通过 | 通过 | 单次拒绝 | 0 |

挂载轮的原生正文与附件调用为 `call_d4e2e5e0d45e7aa2c8e29ac2`、`call_ecbb6c23ee13fddc14787fc4`，选中技能为 `call_a6d9bcd1526c3a14f57f9cf2`、`call_501e20bdf6777c2c17905047`，图谱为 `call_115528ab907d7a4badd14cf1`。拒绝轮的 `call_1adf9a4a530bccb09ac5205e` 收到带相同模型调用 ID 的权限拒绝。MCP 计数由受管桥接的配对工具事件和真实模型续轮结果核对。

## 宿主验证与复跑

复用同一工作区已编译的槽位 10 测试产物，经 `scripts/cargo_slot.py` 领取该槽位后运行 `debug/deps/llm_runtime-85f23fc8970681f8 acp_skill_coexistence_corust_agent --test-threads=1`。两项通过、零项失败，退出 0。两个 Cargo 路径均为仓库下 `target/rust-slot-10` 的绝对路径。定向 Cargo 命令在等共享编译锁时取消，槽位正常释放。Python 语法、修改文件的格式和差异空白检查均通过。

同工作区同槽位的共享 `vw-cli --bin shiju` 构建退出 101，`task-scheduler` 在 `src/store/postgres/task_plan.rs` 第 247、249 行访问 `PlanMember.id` 私有字段并触发 `E0616`。源码仍存在这两处访问。保留任务卡，待公共构建修复后复跑正式网关的挂载、撤除和拒绝验证。启动探针时以 `ACP_CORUST_REVIEWED_DIR` 指向包含摘要匹配 `corust-agent-native` 的受审目录。基线报告位于 `target/acp-skill-coexistence/corust-agent-before.json`，当前状态位于 `target/acp-skill-coexistence/corust-agent.json`。
