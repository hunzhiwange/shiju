# Droid 技能共存验收

状态：已阻塞。私有网关在无挂载轮以 `requires_full_access` 拒绝启动 Droid。Droid 的技能能力门禁保持关闭。

## 受审制品

Darwin arm64，Factory Droid 1.9.0，安装修订 `droid-1.9.0-model-source-v2`，入口 `droid exec --output-format acp`。私有副本通过 `codesign --verify` 及版本检查，SHA-256 为 `0858a4f7411b0159780246598fc6170a17d695556e4da66ab939bff82b58050b`。沿用驱动的 Airgap、私有状态、Chat Completions 回环和 stdio MCP 入口。[受管合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [Droid MCP 审计](../mcp-policy-audit/coexistence/droid.md)

## 网关复验

从仓库根执行。复用槽位 10 的已验网关，目标与构建目录均为仓库下的 `target/rust-slot-10`，网关 SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。本次只改探针与回执，未重新构建或运行 Rust 单测。

```sh
python3 -c 'from scripts.cargo_slot import run; import sys; sys.exit(run(sys.argv[1:], number=10))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent droid --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/droid.json'
```

本次从已有受审安装复制制品，若缺少本地缓存，探针使用仓库受管安装器。私有预设从 `defaults.skills` 和宿主 MCP 列表选择资源，请求使用 `session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017`、`full_access=false`。退出码为 1。无挂载会话 `ses_e077a9522ffeG7NEDTpmibyHIH` 收到 0 个 `chat.done`、1 个 `chat.error`，模型请求和下游 MCP 执行分别为 0。网关在会话技能视图交给供应商前要求当前完全访问授权，尚未取得模型与权限回读。挂载、撤除和拒绝对照未进入。结构化证据为 `target/acp-skill-coexistence/droid.json`。[宿主门禁](../../../../crates/libs/acp-client/src/client/process.rs#L33)

## 独立原生诊断

同一受审制品直连 ACP，在私有项目的 `.factory/skills` 放置随机原生技能，保留 `auto-medium`。首个带工具的模型请求通过原生 system reminder 列出技能，正文和附件均未出现，`ToolSearch` 仍在工具目录。随后 `Skill` 返回正文与绝对 `SKILL.md` 路径，`Read` 从该路径读取附件，两个值均以实际调用 ID 进入模型续轮。三次带工具的模型请求以 `end_turn` 结束，调用分别为 `call_9fb67fc44814acff_0` 和 `call_1031d4d144a6a2bf_0`，没有 MCP 服务或 ACP 审批请求。脱敏证据为 `target/acp-skill-coexistence/droid-native.json`。

继续条件是可在当前网关授权边界使用的受审 Droid 入口。后续需核实会话视图追加方式，并重跑三轮与拒绝对照。
