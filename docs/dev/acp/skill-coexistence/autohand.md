# AutoHand ACP 技能共存验收

任务 24 已阻塞。AutoHand 0.9.11 的原生扫描器能发现项目技能和私有 `AUTOHAND_HOME/skills` 下的选集，但原生文件读取未返回工作区外选集中请求的正文，stdio MCP 调用也没有向宿主请求批准。现有挂载门禁继续拒绝该后端。

## 制品与网关

Darwin arm64 的 `autohand-cli@0.9.11` 报告 `0.9.11 (46a28b16)`。执行入口 SHA-256 为 `339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb`。使用已审的直出工具参数，保留原生工作区检查及默认项目工具名单。模型夹具使用 OpenAI Chat Completions，MCP 使用原生 stdio 及仓库只读附件服务。[MCP 验收](../mcp-acceptance/autohand.md) · [审计](../mcp-policy-audit/5.md#autohand)

正式探针从 `defaults.skills` 和宿主 MCP 配置选取能力，以 `session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false` 发起请求。首次使用父目录作为网关工作目录时，AutoHand 在 ACP 握手前拒绝包含私有 HOME 的目录。改用夹具项目作为网关工作目录后，握手通过，普通轮在 ACP prompt 返回错误。试过完整的上游 Chat Completions 路径，结果相同，未得到成功后的模型或权限回读。

| 网关检查 | chat.done / chat.error | 有效模型请求 / MCP 调用 | 结果 |
| --- | --- | --- | --- |
| 正式无挂载 | 0 / 1 | 0 / 0 | ACP prompt 返回错误，未完成关联读取 |
| 单独执行挂载轮 | 0 / 1 | 0 / 0 | 宿主返回 `autohand: no verified skill directory mount branch` |

正式探针退出 1，报告为 `target/acp-skill-coexistence/autohand.json`。单独挂载门禁报告为同目录的 `autohand-mount-gate.json`。复用槽位 6 的网关，SHA-256 为 `8ee6357b62005376c74ef75d26aa03e9379af73f81e9e8b3a63f5552ec768c52`。Cargo 的 target 和 build 目录均为仓库的 `target/rust-slot-6`。槽位 3 的新构建在公共编译锁排队时停止，退出 130。没有启动本任务的 Cargo 编译或 Rust 单测。

```sh
PYTHONPATH=scripts python3 -c 'import sys; from cargo_slot import run; sys.exit(run(sys.argv[1:], number=6))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent autohand --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/autohand.json'
```

## 独立原生对照

使用相同的固定入口、随机夹具、模型协议和 stdio 服务，直接运行实际 ACP。原生技能放在私有项目的 `.autohand/skills`。选集试用私有 `AUTOHAND_HOME/skills` 到测试目录的链接。首轮模型只接收目录名称。随后通过实际声明的 `skill info`、`skill activate` 和 `read_file` 读取正文及附件，并校验工具调用 ID。初次对照的工作区外选集能发现并激活，`read_file` 续轮缺少选集正文。将诊断用选集移入私有项目的非扫描目录后，取得以下结果。

| 原生对照 | ACP 终态 | MCP 调用 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `end_turn` | 0 | 原生正文及附件均读回 |
| 挂载 | `end_turn` | 1 | 两类技能正文、附件与真实 MCP 值均按 ID 读回 |
| 撤除 | `end_turn` | 0 | 原生技能可读，先前选集不可发现 |
| 拒绝对照 | 探针返回错误 | 1 | 原生客户端未收到审批请求，MCP 值已出现在续轮 |

挂载轮中原生正文、原生附件、选集正文、选集附件和 MCP 的调用 ID 依次为 `probe_b521f02433ce9ef195f525ee`、`probe_af479c49472bedc5cb9d55c0`、`probe_2cf8e8f307e7989376ae49d8`、`probe_db00573922e4dde355c04321` 和 `probe_d7ac77ac385fe7411944a378`。独立诊断的脱敏结果为 `target/acp-skill-coexistence/autohand-native-workspace.json`。

固定版的 `syncMcpTools` 给 stdio 工具注册 `requiresApproval: false`，与实际零审批请求一致。安装包中存在内部 `SkillsRegistry.addLocation` 方法，已审的 ACP 启动与会话接口没有暴露追加受信根的入口。托管快照模式会过滤原有非内置目录，无法用来保留原生发现。继续接入需要可审计的会话目录注册、受信视图文件读取，以及每次 MCP 调用的宿主批准和拒绝路径。取得这些能力后复跑正式三轮与拒绝对照。
