# Cortex Code 原生技能与 ShiJu 技能共存

## 固定制品

Darwin arm64，Cortex Code `1.2.46+175654.6f8561fd271f`，受管安装修订 v5。从已校验的供应商归档经仓库安装入口生成，`codesign --verify` 退出 0。网关绑定 `session_model` 和 `deepseek-v4.1-flash-1017`，仅模型响应替换为私有回环夹具。

| 文件 | SHA-256 |
| --- | --- |
| `cortex` | `f92a402ec97d4a3d3fff7c7b34c835654d5d0e8eb658340b176ee7fe6e082dfa` |
| `cortex-native` | `f9753a0da7a733d4e782dfe633c7bd4fd5faca7b4960352d0aa464b62441178a` |
| `cortex-local` | `9d2395d46b6497d398c928f6d328b521f141584a5dc041eeee305ebc222b2cf9` |
| `mcp_bridge.py` | `3d64a35787047eaf3210ae6187434209e585ce223875200bd4a5e452bfdb6ece` |

## 接入路径

受管 Chat Completions 使用 Cortex 原生注册表和目录格式化器。首次模型请求只提供技能名称、描述和文件路径，正文及附件均通过原生 `read` 回传。禁用模型调用的技能从目录中排除。原生项目扫描保留，选中技能经宿主验证后追加到当前启动目录的 `session-skills`，遵循 Cortex 会话目录优先级。撤除选择后，新进程沿用原生扫描。挂载要求工具已启用的受管 `session_model`，技能源须在当前工作区的沙箱读取范围内。

显式 stdio MCP 沿用已审 `clientTools` 注册、逐次审批和只读执行。探针通过只读管道向真实 MCP 服务提供随机附件，由沙箱外的探针父进程记录服务取值。原生工具保留 `read`、`write`、`edit` 的已审目录。本轮只调用 `read` 与指定的 MCP 工具。

## 修复前

受管 v4 的网关探针退出 1，基线到达首个实际 Chat 请求，原生工具已声明，`<available_skills>` 缺失。失败报告保存于 `target/acp-skill-coexistence/cortex-code-before.json`。同制品的无挂载 ACP 对照使用已知项目路径调用原生读取，模型续轮通过 `call_da5d21e1cbdafeda4d4ed9e7` 读回随机正文，提示以 `end_turn` 结束，下游 MCP 调用为零。

## 固定进程对照

v5 独立 ACP 对照四轮均以 `end_turn` 结束。基线、挂载、撤除、拒绝依次收到 3、6、3、6 个模型请求，实际 MCP 调用为 0、1、0、0。四轮均读回原生正文和附件。挂载和拒绝轮经会话目录读回选中正文和附件，三轮及拒绝对照均排除未选择和禁用技能。挂载 MCP 调用 `call_490db4c0ba80fbfc5f448461` 读回随机附件；拒绝调用 `call_ab169b133a03bda0139f3616` 的续轮收到审批拒绝。

## 定向单测

`python3 scripts/cargo_slot.py cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_cortex_code -- --test-threads=1` 的等价命令在共享编译锁内运行。槽位 6 的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库绝对路径下的 `target/rust-slot-6`。退出 0，2 项通过，0 项失败。测试覆盖无选择时保留启动配置、仅发布选中目录以及拒绝工作区外来源。

## 网关复验待恢复

同工作区的同目标构建在共享槽位 3 退出 101。编译器在 `crates/libs/task-scheduler/src/store/postgres/task_plan.rs` 第 247、249 行报告 E0616，两处访问了私有字段 `PlanMember.id`。已核对该包相对于记录失败的提交 `06a31ab1ba` 无差异，失败文件 Git blob 为 `5ed7fbe7255fc1f2a8a40c0243d1f2ba4bdd4eb6`。取消本卡的重复构建排队并释放槽位 6，槽位进程退出 130。当前网关四轮均待运行，尚无对应的 `chat.done`、模型与权限回读。状态记录见 `target/acp-skill-coexistence/cortex-code.json`。 [同工作区构建回执](deepagents.md#网关复验)

共享构建恢复后将 `ACP_CORTEX_REVIEWED_DIR` 指向摘要匹配的 v5 目录，在仓库根运行下列命令。编译阶段按计划获取共享编译锁，探针复用同一槽位。

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent cortex-code --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/cortex-code.json'
```
