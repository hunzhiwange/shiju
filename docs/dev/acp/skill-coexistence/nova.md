# Nova 会话技能共存验收

固定 npm `@compass-ai/nova@1.1.65`，平台为 Darwin arm64。受管安装修订为 v3，入口仍为 `scripts/bin-wrapper.mjs acp`。原始 `dist/cli.js` 的 SHA-256 为 `ee9d26976cda1beda9b86a0f05aea0e04a77665ae89d8cfbe5395a11ca6e9935`，v3 补丁后为 `ee96b92927b6c1e524b6c20a25f00428b5579692ba298b254725aa66f5dabf5f`。六项摘要均按受管契约核对。测试复用此前为人工夹具授权的最小同意记录，隔离用户目录与项目目录，模型请求仅发往本地回环。

Nova 使用原生 `tool_capabilities` 激活 `invoke_skill`，随后通过 `read_file` 读取附件。宿主传递 `defaults.skills` 解析所得的私有目录视图。选集依附受管连接、Nova 会话、项目和活动回合，取消或结束回合撤回目录上下文。检测到与原生技能同名时沿用原生技能。所选技能使用内联上下文，内置 shell 替换保持禁用。会话目录权限为 `0700`。

MCP 使用受审 stdio 只读沙箱、独占连接及 owner approval 单次审批。回环模型走 Anthropic Messages，网关将 `coexistence-fixture/selected-model` 绑定为 `session_model`。验收使用 `full_access=false`。既有 MCP 接线见 [Nova MCP 验收](../mcp-acceptance/nova.md)。

## 修复前

原网关及 v2 制品的无挂载轮完成原生激活和附件读取，收到一个 `chat.done`、零个 `chat.error`，原生调用 ID 为 `toolu_9d983e163f04527fa86ec8ca` 和 `toolu_9c90895b85e0902cbecd6aff`。挂载轮在宿主技能门禁停止，零模型请求与零 MCP 执行，收到一个 `chat.error`。结构化记录位于 `target/acp-skill-coexistence/nova-before.json`。

## 补丁后原生 ACP 核验

真实 v3 制品的四个独立原生连接均完成回合。挂载和拒绝连接均拒绝在同一连接上再建会话。首次模型请求只含目录元数据，未选与禁用技能始终不可见。所选技能中的 shell 替换表达式未产生执行输出。两个对照保留原生技能激活与读取，且 MCP 执行均为零。

| 轮次 | 原生激活与附件 | ShiJu 激活与附件 | MCP 调用 ID、审批及执行 |
| --- | --- | --- | --- |
| 无挂载 | `toolu_71cf72f8cde269a8b88de91d`、`toolu_1eebc6264ad5df1056aecdb0` | — | 零次 |
| 挂载 | `toolu_8a1b8b11b04bd1910ca7a439`、`toolu_56786268909ed3a546f6f178` | `toolu_58a11d371908f1fcf6cd9d76`、`toolu_c64bc398a7edea396fa76b4e` | `toolu_bff5394f3fcbf07b96e2a98a`，单次批准，一次执行 |
| 撤除 | `toolu_343cce3278cfbc60f4dab14c`、`toolu_739be7e61842564b9078be06` | — | 零次 |
| 拒绝 | `toolu_9af2fd25aedb2ae353b7ffee`、`toolu_ffc17364d0f316ad40880906` | `toolu_76d70b0851050376a5674db0`、`toolu_8e602d6cda8cf45c33332355` | `toolu_15d32ee9a09fa40d62db1b3a`，拒绝，零次执行 |

恢复探针在三个独立连接中创建并两次载入 Nova 会话 `3f8a5bae-0ab2-4804-80c9-334beec0811c`。带选集恢复后再次读取两类技能，MCP 调用 `toolu_3076de7a98caac5ea79e05f7` 获单次批准并执行。空选集恢复保留历史和原生技能，当前模型目录没有旧挂载，MCP 零次执行。

## 正式网关回归

`cargo build --locked -j 1 -p vw-cli --bin shiju` 经共享锁和槽位入口在槽位 10 退出 101。两个 Cargo 缓存路径均为仓库的 `target/rust-slot-10`。本任务的 `agent-skills`、`acp-agent-registry` 和 `llm-runtime` 已随构建编译。共享 `task-scheduler` 在 `store/postgres/task_plan.rs:201` 和 `:203` 访问私有字段 `PlanMember.id`，两处均报 `E0616`。该次构建在运行探针前停止。


共享调度器修正字段访问后，复用槽位 10 已完成的网关构建，确认二进制包含 v3 合约和 Nova 宿主门禁。网关 SHA-256 为 `bb626e51a5235a6db6037d93b0242e945e573eea8aeb0471fc17da5c49d4fdc2`。探针退出 0，四个独立网关会话各有一个 `chat.done` 和零个 `chat.error`。各轮回读 `acp-coexistence / nova`、`session_model`、`coexistence-fixture/selected-model` 和 `full_access=false`。挂载轮与拒绝轮均收到真实单次审批。完整结构化证据位于 `target/acp-skill-coexistence/nova.json`。

| 轮次 | 网关会话 | 原生激活与附件 | ShiJu 激活与附件 | MCP 调用、审批及执行 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_e06bed420ffe9tcrzPGn3qbtHb` | `toolu_97aca294a70bd68a5b46e920`、`toolu_2af087431d1c1f40872ccc94` | — | 零次 |
| 挂载 | `ses_e06bec8d0ffe21yslX7SbV49QP` | `toolu_a9fe9b7a429f9c405d068059`、`toolu_c9aa0a6de48594a680862155` | `toolu_61a92281efa09ac5728d5577`、`toolu_a354db93ec13c8efaad4f5e5` | `toolu_19973eb4c4a38404cdeebdc3`，单次批准，一次执行 |
| 撤除 | `ses_e06beb976ffeC3D0HpZsYlnwkK` | `toolu_240c72a30cda4931e4319ac0`、`toolu_2d0e82e766a50442f8199187` | — | 零次 |
| 拒绝 | `ses_e06beaf38ffeKl2Y0jYWEyZp1G` | `toolu_63dc68bfd8ba426645121245`、`toolu_d4cc19e5c00de344626222e6` | `toolu_e5af9b94038a6e397616af3f`、`toolu_636230102117c494cb61b5bd` | `toolu_3ab036002cd53b0db6d60833`，拒绝，零次执行 |

复验时使用 `ACP_NOVA_REVIEWED_PREFIX` 指定受管 v3 安装前缀，使用 `ACP_NOVA_TEST_CONSENT` 指定已有授权的测试同意记录。按计划在同一 Cargo 槽位先构建网关，再运行 `python3 scripts/verify_acp_skill_coexistence.py --agent nova --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/nova.json`。

## 定向 Rust 单测

复用共享 Cargo 构建产物，独占槽位 6 直接执行同一 `llm-runtime --lib` 测试二进制。两个缓存路径均为仓库的 `target/rust-slot-6`。

```sh
"$CARGO_TARGET_DIR/debug/deps/llm_runtime-85f23fc8970681f8" \
  --exact acp::skill_mounts::nova::tests::acp_skill_coexistence_nova_gates_only_selected_skills \
  --test-threads=1
```

退出 0，1 项通过，0 项失败，522 项过滤。随后撤销仍在等待共享 Cargo 锁的重复编译。
