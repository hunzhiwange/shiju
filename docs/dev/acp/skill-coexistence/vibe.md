# Vibe 原生技能与会话挂载共存

受管入口为 Darwin arm64 的 Mistral Vibe `vibe-acp 3.0.4`。v4 使用固定 wheel，SHA-256 为 `16ee71ef4481dd1b93bc47279b2c76124f6e654ad24d14c9415eea9095f0b0e9`。私有运行时入口 SHA-256 为 `80547aa0b83be7cd1d63536ff041daf93cae8b43fd7c6ee645ff1da67927b7b1`。从摘要匹配的原始文件重放安装补丁，预演和应用均成功，八项文件摘要匹配受管契约。已有 v3 安装需升级到 v4。[受管契约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)

`SkillManager` 保留 Vibe 原有本地根及去重、过滤、项目授权和插件投影，将 ShiJu 当前会话视图追加到本地搜索路径。原有本地根中的同名项按原生顺序优先，`disable-model-invocation` 继续生效。选中技能激活返回会话视图中的路径。MCP 和原生文件读取的审批携带实际工具名、参数及原回调 ID，仍由原生回调处理一次批准或拒绝。

v3 基线使用相同私有 HOME 技能夹具和旧网关，首次真实模型请求缺少原生技能，探针退出 1。旧网关 SHA-256 为 `92a1bcb04e026d7595e756afd2fba3f4f4b01929fad223d13d782cc59e8d10c3`，脱敏报告位于 `target/acp-skill-coexistence/vibe-baseline.json`。

默认路径通过 `search_tool_functions` 查找已配置的 stdio 工具，再从 `run_typescript` 调用。Vibe 原生 ACP 旁路对照在私有副本中仅切换 MCP 暴露类型为 `direct`，同时保留程序化搜索和执行目录。模型实际收到直列 `read_probe`，读取两类技能后批准轮下游执行一次，拒绝轮零次，两轮均结束为 `end_turn`。默认路径和私有直列路径各完成无挂载、批准、拒绝三个原生回合，MCP 调用各为 0/1/0。调用和审批 ID 见 `target/acp-skill-coexistence/vibe-native-controls.jsonl`。现行受管安装继续使用默认程序化路径。[v3 默认路径记录](../mcp-policy-audit/coexistence/vibe.md)

同名对照中，SkillManager 保留了原生技能及其正文。原生配置禁用该名称后，两个来源都未出现在目录。跨进程 `session/load` 对照中，原会话仅保存了目录检查回合。新进程载入原会话时，原生用户技能或已授权项目技能继续可见，已撤除的挂载项从目录消失。

## 网关验收

复用共享槽位 6 的 `cargo build --locked -j 1 -p vw-cli --bin shiju` 成功产物。两项 Cargo 路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`，网关 SHA-256 为 `8ee6357b62005376c74ef75d26aa03e9379af73f81e9e8b3a63f5552ec768c52`。槽位内执行 `python3 scripts/verify_acp_skill_coexistence.py --agent vibe --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/vibe.json`，退出 0。

| 回合 | 模型请求 | 原生技能读取 | 选中技能读取 | MCP 决策 | 下游次数 |
| --- | ---: | --- | --- | --- | ---: |
| 基线 | 4 | 正文及附件 | 未发现 | 无 | 0 |
| 挂载 | 7 | 正文及附件 | 正文及附件 | 批准一次 | 1 |
| 撤除 | 4 | 正文及附件 | 未发现 | 无 | 0 |
| 拒绝 | 7 | 正文及附件 | 正文及附件 | 拒绝 | 0 |

四个独立会话各有一个 `chat.done`、零 `chat.error`。网关回读均为 `vibe / session_model / coexistence-fixture/deepseek-v4.1-flash-1017`，`full_access=false` 且 `access_status.requested=false`。每轮批准一次工作区外的原生附件读取，选中附件从会话视图读取。未选中和显式禁用技能始终不在首轮目录。首次请求未包含随机正文、附件或 MCP 返回值。挂载轮的原生激活、选中激活及 MCP 调用 ID 分别为 `call_331c61ddf44cfe88d38c7c24`、`call_835f77f7839a6bd7e9395c0d`、`call_dd2ff80d6518d741050088db`。拒绝轮的 MCP 调用 `call_37b487cf4ab65b8b9eb33d2b` 回传拒绝，未触发下游。完整 ID 和脱敏回读见结构化报告。

技能挂载探针 Python 回归运行 46 项，退出 0。本次 Rust 宿主挂载配置和权限策略沿用现有实现，制品接线以真实网关验收验证。
