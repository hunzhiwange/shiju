# Claude 原生技能与 ShiJu 会话技能共存

2026-11-20 在 Darwin arm64 复验 `@agentclientprotocol/claude-agent-acp` 0.26.0、SDK 0.2.96 和 Claude Code 2.1.96。适配器入口 `dist/index.js` 的 SHA-256 为 `2859ccaa5ba91ed27b80614eae47399370015acbcbc7d01d1408c71f7964565d`，`dist/acp-agent.js` 为 `00e66a056cc39ba087dffe63da3b62d27fcff6694e6eee5fe3b25961e6218ddd`，SDK `cli.js` 为 `d4d06b148b05d3d21ad250a9900b7c8c47780088712299acd9892a8777f76ab4`。启动前逐项重验历史回执的四个制品摘要和嵌入引擎版本。[固定制品回执](../../../vibewindow/acp-skill-mounts/evidence/claude.json)

网关通过 `session_model` 绑定 `coexistence-fixture/claude-sonnet-4-6`，仅将上游原生 Anthropic Messages 响应替换为本地夹具。模型配置、实际 Claude 进程、`Skill` 与 `Read`、stdio MCP 执行和宿主审批均走生产路径。私有项目的 `.claude/skills` 沿 SDK 原生 project 来源发现，`.shiju/skills` 只有 `defaults.skills` 选中项通过 `SessionSkillView` 注册为 `shiju-session-skills` 插件。挂载轮使用插件限定名激活，`Read` 路径从原生展开取得，并核对路径确实由会话视图解析到选中源。MCP 来自项目 `.vwacprc.json` 和 `defaults.mcp`。四轮保持 `full_access=false`，运行期间读回同版访问状态 `requested=false`、`support=unavailable`，此值是完全访问能力的状态。结束后读回网关绑定和选模。

| 独立会话 | 原生正文及附件 | 选中正文及附件 | MCP 审批与续轮 | 下游执行 | `chat.done` / `chat.error` |
| --- | --- | --- | --- | --- | ---: | ---: |
| 基线 | 匹配调用 ID | 未发现 | 未发现 | 0 | 1 / 0 |
| 挂载 | 匹配调用 ID | 匹配调用 ID | 单次允许，同 ID 返回随机值 | 1 | 1 / 0 |
| 撤除 | 匹配调用 ID | 未发现 | 未发现 | 0 | 1 / 0 |
| 拒绝对照 | 匹配调用 ID | 匹配调用 ID | 单次拒绝，同 ID 返回拒绝错误 | 0 | 1 / 0 |

首次 Messages 请求逐轮检查目录及工具声明，并确认随机正文、附件和 MCP 值均未出现。各轮首个目录均没有未选中技能。原生 Skill 结果的调用 ID 和随后的用户展开关联，附件和 MCP 输出由同 ID 的工具结果确认。首次使用现行网关的完整行为基线已通过，随后用更严格的路径和拒绝文本断言复跑通过。现行启动与挂载实现已满足此路径，Rust 和发布制品保持原有版本。此次只覆盖原生 Messages。

从仓库根执行，使用已有槽位 3 的网关二进制。两个 Cargo 目录均为仓库下的绝对路径 `target/rust-slot-3`。网关 SHA-256 为 `c0d3d7f86e543dd41244388e4f388844090d431c6147a4026b260d8c51b49622`。四轮回环退出 0，三项 Python 定向单测退出 0。本次复用网关，未重新编译或运行 Rust 单测。脱敏报告包含每轮会话 ID、调用 ID、读回及终态，保存于 `target/acp-skill-coexistence/claude.json`。探针退出时清理私有网关、ACP 子进程、模型监听及临时项目。

```sh
python3 -m unittest discover -s scripts/tests -p test_verify_acp_skill_coexistence.py
python3 scripts/cargo_slot.py sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent claude --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/claude.json'
```

## 供应商探针接口

在 `scripts/acp_skill_coexistence/<下划线 key>.py` 提供 `launch(env)`，返回网关 ACP 启动配置和脱敏的固定制品身份。声明 `MODEL`、`ADAPTER`、`API_PATH`、`TRANSPORT` 和项目原生技能目录 `NATIVE_DIR`。当前共享入口接入已审 stdio，使用 `acp_mount_integration.py` 的只读服务与执行审计。

`Handler` 处理该供应商的真实模型请求协议，通过 `server.key` 校验私有模型凭据，并将请求交给 `server.probe`。`Probe(fixtures, mounted, rejected)` 按实际观察到的工具结构和目录发出只读调用。它公开模型请求计数 `requests`、已验证的语义动作与调用 ID 映射 `completed`、首个脱敏失败 `failure`，以及 `approves(metadata)` 来识别当前唯一等待审批的 MCP 动作。运行器维护私有网关、配置、独立会话、一次性审批、审计计数和报告。使用共享 `correlated_result` 验证原生结果关联，完成固定版验证后将注册 key 添加到入口的 `AGENTS`。
