# AutoHand ACP 技能共存验收

任务 24 已阻塞。网关无挂载基线通过，挂载轮命中现有能力门禁。AutoHand 0.9.11 的原生读取器拒绝工作区外选集。私有预工具钩子可对夹具的 stdio MCP 请求单次审批。

## 制品与网关

Darwin arm64 的 `autohand-cli@0.9.11` 报告 `0.9.11 (46a28b16)`。执行入口 SHA-256 为 `339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb`。使用已审的直出工具参数，保留原生工作区检查及默认项目工具名单。模型夹具使用 OpenAI Chat Completions，MCP 使用原生 stdio 及仓库只读附件服务。[MCP 验收](../mcp-acceptance/autohand.md) · [审计](../mcp-policy-audit/5.md#autohand)

正式探针从 `defaults.skills` 和宿主 MCP 配置选取能力，以 `session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false` 发起请求。首次使用父目录作为网关工作目录时，AutoHand 在 ACP 握手前拒绝包含私有 HOME 的目录。以夹具项目启动网关后，握手通过。旧探针声明的 1,024 输出上限低于固定客户端请求的 `max_tokens=16000`，中继在上游调用前返回 422。复用公共运行器的供应商输出上限配置后，无挂载基线通过。生产中继的模型上限校验保持生效。

| 网关检查 | chat.done / chat.error | 有效模型请求 / MCP 调用 | 结果 |
| --- | --- | --- | --- |
| 无挂载 `ses_e06652b68ffew1pK5UjLlB14qd` | 1 / 0 | 5 / 0 | 原生正文和附件按 `probe_07bb02a73143698abadefa73`、`probe_0b11fc8af9f93546adf5cf6d` 读回 |
| 挂载 `ses_e06651bc0ffefi3dVS1zo07PNg` | 0 / 1 | 0 / 0 | 宿主返回 `autohand: no verified skill directory mount branch` |

无挂载轮回读 `autohand`、指定模型、`session_model`、`full_access=false`，原生全权限模式支持为 `unavailable`。正式探针因挂载门禁退出 1，未执行网关撤除和拒绝轮。槽位 6 的报告为 `target/acp-skill-coexistence/autohand.json`。同一探针复用更新的槽位 10 网关得到上表结果，报告为 `autohand-current.json`，制品 SHA-256 为 `bb626e51a5235a6db6037d93b0242e945e573eea8aeb0471fc17da5c49d4fdc2`。各轮的 Cargo target 和 build 路径均指向当次持有的槽位。复验未重新编译或修改 Rust。

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

## 只读替代路径复验

在独立私有项目中把源目录与扫描目录分开。私有 `AUTOHAND_HOME/skills` 里的真实扫描目录含链接的 `SKILL.md` 和附件目录。工作区内源通过原生读取。将同一布局的源移到工作区外仍能发现并激活，但 `read_file` 在正文续轮失败。原生 `FileActionManager.resolvePath` 对真实路径检查工作区和 `additionalDirs`，同一检查器也用于写入，因此普通 `--add-dir` 或目录访问授权无法表达选集只读权限。ACP 入口没有把 `read_file` 委派到宿主文件读取回调。内部 `SkillsRegistry.addLocation` 未暴露为受审的 ACP 会话注册入口。托管快照模式会过滤原有非内置目录。

固定版 `syncMcpTools` 默认把外接工具标成无需审批。私有配置中的 `pre-tool` 钩子按完整工具名匹配，只返回 `ask`，成功触发 `session/request_permission`。四个独立原生会话都返回 `end_turn`。基线、挂载、撤除、拒绝的 stdio 审计计数为 0/1/0/0。批准工具调用 `probe_eb803ab9c7d602ba407a68ec` 读回真实附件，拒绝调用 `probe_0ceebfeb6df8090fdc1218cf` 按调用 ID 回传原生跳过结果。空的私有工作区信任记录下，额外的项目 `permission-request` 钩子未覆盖宿主审批。已信任项目钩子仍可在后续权限阶段改变决定，因此生产接线需要独立核实项目钩子优先级或在宿主转发端强制审批。已有 VT Code 转发依赖适配器提供主动申请的单次凭证，AutoHand 当前协议没有提供该合同。

本地脱敏报告见 `target/acp-skill-coexistence/autohand-hook.json`、`autohand-project-hook.json` 和 `autohand-external-view.json`。下一步需要审定的供应商制品或上游接口，支持把会话选集追加到原生发现，同时按受信视图提供工作区外正文和附件的只读访问。再接入可强制执行的 MCP 单次审批后，复跑正式三轮和拒绝对照。
