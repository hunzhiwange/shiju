# OpenCode ACP 技能共存验收

2026-11-20，Darwin arm64，状态为已阻塞。受管 `opencode-ai` 1.19.5 v1 的 `reviewed/opencode` 通过严格码签校验，自报 1.19.5，SHA-256 为 `1a1f10028d3edebddd6df10a465cfa79b58d710875173dbd32137a8b115daa2c`。沿用受审 ACP 启动参数 `acp`、原生 stdio MCP 和私有回环 Chat Completions。[原 MCP 验收](../mcp-acceptance/opencode.md)

## 生产网关基线

探针从 `defaults.skills`、`defaults.mcp` 和宿主 `.vwacprc.json` 解析选集，发送 `full_access=false`，指定 `session_model` 和 `coexistence-fixture/deepseek-v4.1-flash-1017`。复用槽位 10 中已验的网关，SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。两个 Cargo 缓存路径均指向该槽位。基线和复跑均退出 1，证据位于 `target/acp-skill-coexistence/opencode-before.json` 和 `target/acp-skill-coexistence/opencode.json`。

基线会话在启动 OpenCode 前触发 `requires_full_access`。收到一个 `chat.error`、零个 `chat.done`、零条模型请求和零次 MCP 下游调用。正式网关未进入挂载、撤除或拒绝对照，未取得模型及权限回读。既有桌面门禁要求有效的全访问授权，因为宿主审批无法约束 OpenCode 的本机原生工具。本次隔离会话以受限权限发起。[桌面权限门禁](../../../../crates/libs/acp-client/src/client/process.rs)

## 原生 ACP 对照

同一 v1 二进制在私有目录执行 `acp --pure`，空 `SHIJU_OPENCODE_SKILL_ROOT` 使模型首轮目录缺失项目原生技能。这个断点与现行扫描补丁一致，早于技能激活。原生证据位于 `target/acp-skill-coexistence/opencode-production-v1.json`。

上游原版 1.19.5 的 SHA-256 为 `feaafead8cd563af0c2dcb386b0757e4355e327ebb6bd1aaad5440fe2f83596c`，与固定安装合约相符，严格码签校验通过。私有 ACP 对照保留 `.agents/skills` 的原生发现，使用原生 `skills.paths` 追加独立的会话视图，以 `coexistence_*=ask` 请求 MCP 单次审批。探针逐个验证原生 `skill` 的正文、返回路径规范化后的身份、原生 `read` 的随机附件、未选技能缺席和首次模型请求无正文。MCP 放行通过工具调用 ID 核对实际续轮结果。MCP 拒绝通过同 ID 的原生失败事件核对，OpenCode 随即结束提示。

| 轮次 | ACP 终态 | 模型请求 | MCP 审批 | 下游调用 |
| --- | --- | ---: | --- | ---: |
| 基线 | `end_turn` | 3 | 无 | 0 |
| 挂载 | `end_turn` | 6 | 单次批准 | 1 |
| 撤除 | `end_turn` | 3 | 无 | 0 |
| 拒绝 | `end_turn` | 5 | 拒绝 | 0 |

四轮独立会话及关联调用 ID 位于 `target/acp-skill-coexistence/opencode-upstream.json`。私有追加扫描原型也通过四轮，另用 `--pure debug skill` 确认原生同名技能在原型中优先。原型采用固定输入摘要和等长替换，重签后通过严格码签验签，入口 SHA-256 为 `bf892198c2ca6f7d698215b4924c7e3cfec7abe76cf79825bbe39811b0a2c88d`，原型回执位于 `target/acp-skill-coexistence/opencode-prototype.json`。私有模型配置省略 `ask` 时，OpenCode 直接执行该 stdio 调用，未向客户端发起审批。

当前生产路径仍需可约束原生工具的受审启动方式和与既有策略兼容的逐次 MCP 审批。保留现有受管制品和门禁。检查通过共享 Python 判定用例 5 项及探针语法检查。初次构建在串行锁上排队，取消后改用已有槽位网关复验。未修改 Rust，未执行 Rust 单测或桌面构建。

取得受审受限启动路径后，从仓库根按计划持有共享编译锁及槽位复跑生产探针：

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent opencode --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/opencode.json'
```
