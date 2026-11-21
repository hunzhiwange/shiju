# AutoHand ACP 技能共存验收

原版 AutoHand 0.9.11 的对照确认，网关无挂载基线通过，挂载轮命中旧能力门禁。原版读取器拒绝工作区外选集。本文记录原版对照和受管补丁的后续验证。

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

## 固定受管补丁

受管 `autohand-0.9.11-model-source-v1` 从 npm 固定包和依赖锁安装，禁用 npm 安装脚本，校验包的 registry 完整性及四个入口文件摘要。安装目录独立于本机 CLI。Darwin arm64 上已用生产安装器两次生成制品，最后一次在新目录重新安装并校验。[受管契约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [可复现补丁](../../../../patches/acp-model-source/autohand/apply.py)

ACP 初始化保留原生扫描器，在生成系统目录之前注册当次选集。宿主校验活跃技能租约，只复制所选目录，验证复制内容后发布 0700 的私有视图，资源文件权限为 0400。补丁用独立的原生 `FileActionManager` 处理视图中的 `read_file`。普通读写仍使用已有工作区管理器。路径逃逸、内部符号链接、硬链接和非普通文件不会获得视图读权限。

网关把宿主已选 stdio MCP 投影至已有审批中继。补丁在供应商工具派发点校验连接，当次工具名和精确参数通过 ACP 获取单次审批，中继在下游派发前消费授权。原生项目配置和用户钩子沿用原有支持。挂载分支要求固定受管入口、`session_model` 和已启用工具的权限模式。普通无挂载和无宿主 MCP 的会话继续使用原启动行为。

最终安装产物的独立原生 ACP 对照四轮均返回 `end_turn`，真实只读 MCP 调用计数为 0/1/0/0。挂载轮读回的选集正文、附件和 MCP 调用 ID 分别为 `probe_47c5ea26cbd283438467f027`、`probe_edd05bf4ceea2c6fbd1b3a4a`、`probe_fcdae134b27c3edf464ae9f4`。拒绝轮 `probe_dfaa50e789abef6b66582d1b` 回读原生跳过结果。记录在 `target/acp-skill-coexistence/autohand-patch-native-final.jsonl`。针对同制品的文件边界测试通过，覆盖选集可读、选集不可写、外部同级文件拒读、符号链接拒读和 MCP 连接替换拒绝。

槽位 9 的 Cargo target 与 build 路径均为仓库的 `target/rust-slot-9`。定向命令 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_autohand -- --test-threads=1` 实际通过 2 项，退出 0。两项覆盖外部选集快照、未选集隔离、文件读权限、源变更拒绝和无选集时的启动环境。

同一新版探针和受管 npm 制品复用旧网关测红。槽位 6 的网关 SHA-256 为 `09c49dfa8efa59deebb31d40ddd6f4533e7670a09f71baef33e77f10f5284453`，无挂载读回成功，挂载被旧门禁拒绝，退出 1。报告为 `target/acp-skill-coexistence/autohand-managed-red.json`。

## 正式网关验收

`cargo build --locked -j 1 -p vw-cli --bin shiju` 在槽位 9 退出 0。网关 SHA-256 为 `0991a07bf3ef6f9cf52f179979d0aee8552e611af5860a7f69b9f778d9731499`。首次执行见 `autohand-shape.json`，挂载轮的读取与审批已完成，探针把原生专家准备请求误判为主会话。按实际专家系统前缀完善探针，对辅助请求验证无私有正文、附件和 MCP 值。主会话仍校验原生目录、调用 ID、实际审批与下游审计。

复用同一网关与制品执行正式探针后退出 0。四轮均回读 `autohand`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false`。宿主仅对挂载轮批准一次，拒绝轮未执行下游。选中及非选中源放在独立私有 HOME 的 `.skills`，在项目外。撤除轮仍可读取项目原生技能，先前选集不可发现。

| 轮次 | 网关会话 ID | chat.done / chat.error | 主会话模型回合 | 下游 MCP |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_e03edae60ffeO0LMTZfUYuMlm0` | 1 / 0 | 5 | 0 |
| 挂载并批准 | `ses_e03eda1bfffeGa83ZkX5F6KJLm` | 1 / 0 | 10 | 1 |
| 撤除 | `ses_e03ed9081ffe5fbKvvKYpRGKpN` | 1 / 0 | 5 | 0 |
| 挂载并拒绝 | `ses_e03ed8497ffefitOy4Bd02s2kn` | 1 / 0 | 10 | 0 |

挂载轮的原生正文、原生附件、选集正文、选集附件和 MCP 调用 ID 分别为 `probe_6f52e86a5394f5bf200949cc`、`probe_7fe011a2d50b29caa3b2e635`、`probe_a429b5771ee2ca024c2008bf`、`probe_92210af15bcc3c5bbf528383` 和 `probe_6df5b94454cdfdd7cf5d9863`。拒绝调用 `probe_9c49308d8240c5f8b796e379` 按 ID 回读原生跳过结果。正式结构化证据在 `target/acp-skill-coexistence/autohand.json`。本次为 Darwin arm64 的请求级启动验证。选集按请求重建私有原生会话。
