# Kimi 技能共存验收

## 固定入口

Darwin arm64 原生 Kimi Code CLI 2.5.0，入口为绝对路径加 `acp`。下载后核对发布清单中的原始二进制 SHA-256 `d4dd02d40f18f3a784699fc37ae0c55d676a4704c37a3b91ab96541b0db673a1`，并读取 `--version`。探针把该制品复制到私有目录运行。模型绑定为 `session_model` 和 `coexistence-fixture/deepseek-v4.1-flash-1017`，回环协议为 OpenAI Chat Completions。MCP 沿 `session/new.mcpServers` 的原生 stdio 路径，保持 `full_access=false`。[固定版源代码](https://github.com/MoonshotAI/kimi-code/tree/refs/tags/%40moonshot-ai/kimi-code%402.5.0) · [发布清单](https://code.kimi.com/kimi-code/binaries/2.5.0/manifest.json) · [MCP 基线](../mcp-acceptance/kimi.md)

Kimi 的 `ExtraFileSkillSource` 从私有 `KIMI_CODE_HOME/config.toml` 读取 `extra_skill_dirs`，按现有原生优先级合并技能。项目和用户技能优先于追加目录，追加目录优先于内置来源。宿主在校验制品和会话租约后只追加租约根。原生 `Skill` 返回关联的调用结果，并在同一续轮递送 `<skill-loaded>` 正文和目录，`Read` 按该目录读取附件。无选集时驱动生成的配置省略该字段。宿主拒绝 `cli_local`、禁用工具、未经审核的平台和制品的挂载请求。旧版独占目录回执仍适用于 `--skills-dir` 的历史行为。[原生技能发现](https://github.com/MoonshotAI/kimi-code/blob/main/docs/en/customization/skills.md#skill-locations) · [历史回执](../../../vibewindow/acp-skill-mounts/evidence/kimi.json)

## 修复前证据

仓库槽位 10 的缓存网关 SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。修复前探针退出 1。无挂载会话 `ses_e076d79c2ffecrMVL4e18FIMur` 收到一次 `chat.done`，零错误，三次模型请求，技能调用 `call_c76f3bba03f02f8214a4baad` 和附件读取 `call_2b201c770c6a5aeff7668540` 均进入续轮。网关回读后端 `kimi`、指定模型和 `full_access=false`。挂载会话 `ses_e076d67ddffeNsc4osIlaA7ZmX` 在宿主能力门禁收到一次 `chat.error`，零模型请求和 MCP 调用。脱敏证据位于 `target/acp-skill-coexistence/kimi-before.json`。

## 固定版原生诊断

同一制品直连 ACP，四轮均保留原生 `mode=default`，各自新建私有会话。挂载与拒绝两轮的配置只追加一个包含选中技能软链接的私有试验目录。首次模型请求仅见符合轮次的技能元数据，正文和附件在原生调用后进入续轮。非选中技能一直不可见。

| 轮次 | 原生终态 | 模型请求 | MCP 审批 | MCP 执行 |
| --- | --- | ---: | --- | ---: |
| 基线 | `end_turn` | 3 | 无 | 0 |
| 挂载 | `end_turn` | 6 | 允许一次 | 1 |
| 撤除 | `end_turn` | 3 | 无 | 0 |
| 拒绝 | `end_turn` | 6 | 拒绝一次 | 0 |

挂载轮依次读回原生技能正文 `call_73e2a13e409fc40dbbdf45b3`、原生附件 `call_f4837aaea2eb97fa68c728af`、选中技能正文 `call_32d4327365da7872d135f2de`、选中附件 `call_66a9c2b595945c737eedebfb` 和 MCP 结果 `call_8841cd0db75f1e4b86d2c818`。拒绝后的原生错误续轮关联 `call_a8632a67296c2ba7b027ac0c`。原生诊断日志位于 `target/acp-skill-coexistence/kimi-direct-*.log`。

## 网关复验状态

槽位 9 的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库内的 `target/rust-slot-9`。本次网关构建已通过 `llm-runtime` 编译，随后在并行开发的 `task-scheduler` 退出 101。错误包括 `E0432`（构建时 API 缺少 `TaskPlanSubTask`、`WorkPlanPublication`）和 `E0616`（PostgreSQL 存储访问私有 `PlanMember.id`）。后续共享构建越过 API 导出错误，仍在同一私有字段失败。网关挂载、撤除和拒绝对照尚待复验。

槽位 10 的旧网关另通过 MCP 隔离对照。四个会话均有一次 `chat.done`、零错误，并回读受限模式与指定模型。MCP 实际调用为 0/1/0/0，挂载轮允许一次，拒绝轮拒绝一次。此对照使用原生技能与 MCP，选中技能挂载由新网关复验。证据位于 `target/acp-skill-coexistence/kimi-mcp-only.json`。

定向单测复用槽位 3 已编译的 `llm_runtime-85f23fc8970681f8`，在持有槽位期间按 `acp_skill_coexistence_kimi --test-threads=1` 过滤，退出 0，实际执行 2 项且全部通过。两项缓存路径均为仓库内的 `target/rust-slot-3`。槽位 9 的重复单测编译随后停止，槽位租约已释放。

## 复验入口

从发布清单对应的 [Darwin arm64 压缩制品](https://code.kimi.com/kimi-code/binaries/2.5.0/kimi-code-darwin-arm64.zst) 解出可执行文件，放入 `target/acp-skill-coexistence/kimi-artifact/kimi`。探针启动前会复制并校验其版本和摘要。从仓库根运行，Cargo 构建须持有计划中的共享构建锁。

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent kimi --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/kimi.json'
python3 scripts/cargo_slot.py cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_kimi -- --test-threads=1
```
