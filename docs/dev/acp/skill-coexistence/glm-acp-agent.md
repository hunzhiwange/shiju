# GLM Agent ACP 技能共存验收

## v2 基线

v2 的会话技能挂载受门禁限制。GLM 1.17.1 扫描项目和用户的 `.claude/skills/<name>/SKILL.md`，把结果作为 `available_commands_update` 发给 ACP 客户端。用户输入 `/name` 时，供应商会把命令正文展开到这条用户消息。普通模型回合缺少技能目录及按需激活入口。只追加目录仍会沿用客户端命令语义，无法满足模型按需激活和同调用 ID 续轮的合同。

## 制品与直接 ACP 对照

在 Darwin arm64 用现行安装器将 npm `glm-acp-agent@1.17.1` 安装到独立私有目录 `glm-acp-agent-1.17.1-model-source-v2`。安装器校验发行包完整性和补丁前后摘要。入口 SHA-256 为 `d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98`，斜杠扫描器 SHA-256 为 `14acef1c5378785fffabf5783c46cbd517fe3c12de721684fd749ab6af8e2a96`。[受管接入](../mcp-acceptance/glm-acp-agent.md#受管接入)

```sh
PYTHONPATH=scripts python3 -m acp_skill_coexistence.glm_acp_agent \
  > target/acp-skill-coexistence/glm-acp-agent-direct.json
```

直接对照退出 0。四个独立 ACP 会话都收到原生命令目录，显式调用项目斜杠命令后，以 GLM 原生 `read_file` 读取随机附件，并在 Chat Completions 续轮按原调用 ID 核对返回值。四个 prompt 均返回 `end_turn`。

| 对照 | 带工具的模型请求 | 下游 stdio MCP 调用 | ACP 审批请求 |
| --- | ---: | ---: | ---: |
| 无挂载 | 2 | 0 | 0 |
| 仅挂 MCP | 3 | 1 | 0 |
| 撤除 MCP | 2 | 0 | 0 |
| 服务工具名单拒绝 | 3 | 0 | 0 |

允许轮通过真实 `read_probe` 按原调用 ID 回读随机内容。拒绝轮沿用受管供应商名单过滤，模型请求的目录中已排除该工具，强制试调返回未知工具，下游计数为零。静态名单对照与宿主逐次审批是两条路径。直接 ACP 对照里的允许轮没有发出 `session/request_permission`。

直接 ACP 的普通提示也返回 `end_turn`。首个 Chat Completions 请求包含原生 `read_file`，其中没有随机生成的技能名称、正文或附件。该对照与显式斜杠调用的差异发生在供应商进入模型回合前。

## 网关回环

从 `defaults.skills` 和项目 MCP 配置选择夹具，绑定 `coexistence-fixture/deepseek-v4.1-flash-1017` 与 `session_model`，请求 `full_access=false`。沿用槽位 3 的已构建网关，双缓存路径均为仓库的 `target/rust-slot-3`，网关 SHA-256 为 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。GLM 默认请求 32,768 个输出 token，首次配置的 1,024 上限触发中继 HTTP 422。复跑仅把 GLM 私有模型夹具的上限按该默认值配置。

```sh
PYTHONPATH=scripts python3 -c 'import sys; from cargo_slot import run; sys.exit(run(sys.argv[1:], number=3))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent glm-acp-agent --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/glm-acp-agent.json'
```

- 无挂载轮获得一次带工具的模型请求、一次 `chat.done`、零 `chat.error`、零审批及 MCP 执行。探针在模型请求中核实原生文件读取工具可见，三种随机技能均缺席，随后以退出 1 报告缺少模型侧技能入口。
- 在同一槽位把验收阶段临时限定为 `mounted`。退出 1，零 `chat.done`、一次 `chat.error`，模型、审批与下游 MCP 计数均为零。网关明确返回 `glm-acp-agent: no verified skill directory mount branch`。结果位于 `target/acp-skill-coexistence/glm-acp-agent-mount-gate.json`。
- Python 语法和 Git 空白检查通过。复用已有网关后取消排队中的本卡构建，未运行 Cargo 构建或 Rust 单测。成功权限回读、正式挂载、撤除及宿主审批拒绝对照仍缺证，仅在 Darwin arm64 实测。

继续接入需要可供模型按需激活的受审原生技能入口，以及能在调用下游前提交宿主审批的 MCP 路径。

## v3 受管补丁复验

固定发行包版本仍为 1.17.1。Darwin arm64 的受审 `session_model` 启动把当前 `SessionSkillView` 中的链接和目标交给 v3 供应商进程。现有斜杠扫描器的项目技能与选中技能共用模型目录。目录仅提供名称和路径，模型按需通过供应商 `read_file` 读取正文和附件，目录随当前模型请求生成。原生命令冲突沿用 GLM 优先级。选中来源必须在当前工作区内，以沿用 GLM 原生文件读取边界。工作区外的原生用户技能仍可走原有 ACP 斜杠菜单。

同一启动路径里，GLM 在调用会话 MCP 前提交 ACP 逐次审批。服务级允许和拒绝名单继续在目录发现时生效。四轮直接 ACP 对照分别取得 3、6、3、6 次模型请求，按调用 ID 核对技能正文、附件及 MCP 结果。允许轮为批准前零次、批准后一次，拒绝轮为零次。旧入口行为对照保持四轮 `end_turn`，原生斜杠命令与服务名单继续生效。

- v3 制品目录为 `glm-acp-agent-1.17.1-model-source-v3`。运行入口 SHA-256 仍为 `d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98`。修改模块及补丁摘要由 [受管合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) 固定。
- Node 受管入口服务目录回归测试实际运行 1 项并通过。直接结果记录于 `target/acp-skill-coexistence/glm-acp-agent-v3-approval-direct.json` 和 `target/acp-skill-coexistence/glm-acp-agent-v3-direct.json`。v2 证据见上文。会话技能接线限 Darwin arm64。

Rust 定向测试复用了槽位 9 已编译的 `llm-runtime` 测试程序。以 `acp_skill_coexistence_glm_acp_agent --test-threads=1` 实际运行 2 项，全部通过，覆盖空选择、仅注册受验证项目技能、拒绝项目外来源和拒绝失效视图。

正式网关首轮构建在槽位 10 遇到并行 DimCode 模块的 E0283，尚未运行 GLM 网关探针。共享工作区已移除该处多余的 `.into()`，定向重试沿用槽位 10。
