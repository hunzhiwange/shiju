# Droid 技能共存验收

状态：受管网关验收通过。历史红验证中，私有网关在无挂载轮以 `requires_full_access` 拒绝启动 Droid。

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

## 受限启动续查

共享源码仍在创建技能视图前对 Droid 检查完全访问，进程启动还会根据实际审批模式二次检查。现行受管摘要与上轮相同。该断点与构建槽位、工具安装和并行改动无关，沿用原网关诊断。[请求入口](../../../../crates/libs/llm-runtime/src/acp/request.rs) · [进程入口](../../../../crates/libs/acp-client/src/client/process.rs)

Darwin arm64 私有对照使用同一已验签的 Droid 1.9.0 v2。受限进程允许 Droid 重启同一受审可执行文件，拒绝执行其他二进制。禁止派生进程时，`session/new` 报对子进程的 `posix_spawn` 操作被拒绝。允许 Droid 自身派生后，可保留原生 `auto-medium`、`Skill`、`Read` 与 `ToolSearch`。沙箱只允许向本次模型和 MCP 的指定回环端口发起连接，工作区不可写。

项目原生技能继续由 `.factory/skills` 发现。用户技能的有效扫描根是 `$FACTORY_HOME_OVERRIDE/.factory/skills`。候选把该私有根链接到与 `SessionSkillView` 同形的目录，保留视图下面链接到选中来源的第二层链接。模型续轮中的原生激活路径仍指向私有 Factory 根，并解析到选中的原始 `SKILL.md`。项目 `.factory/settings.json` 保留只读访问，被 `disabledSkills` 禁用的原生技能与未选 ShiJu 技能均未进入目录。若沙箱屏蔽已存在的项目设置，Droid 子进程在建会话时退出。

| 独立 ACP 对照 | 模型带工具请求 | 原生终态 | 下游 MCP |
| --- | ---: | --- | ---: |
| 基线 | 3 | `end_turn` | 0 |
| 挂载 | 6 | `end_turn` | 1 |
| 撤除 | 3 | `end_turn` | 0 |
| 拒绝 | 5 | `cancelled` | 0 |

挂载和拒绝都使用原生 ACP HTTP 服务登记，将真实只读执行交给宿主侧 stdio 夹具。批准由 ACP 宿主按 `_meta.factory` 的服务和工具、原始参数、会话 ID 与单次选项匹配。挂载轮 MCP 返回与批准使用相同调用 ID。拒绝轮以对应调用 ID 得到原生失败事件后结束，没有额外模型请求。报告记录三个正常会话和独立拒绝会话的 ID、正文和附件调用及审批证据。加入项目禁用配置后的首轮复测在拒绝场景建会话阶段超时，追加阶段记录后复跑四轮退出 0。

```sh
PYTHONPATH=scripts python3 -m acp_skill_coexistence.droid_sandbox \
  --output target/acp-skill-coexistence/droid-sandbox.json
```

原型验收时生产中继尚未接入 Droid。Droid 用 `_meta.factory.server/tool` 声明审批目标，标题是展示字符串。现行消费型中继按受管工具名识别批准，不能直接把 Droid 标题当作可消费的授权。生产复验需要接通并审定受管子进程沙箱，将原生服务和工具身份映射到同会话中继的单次批准，并保留项目设置的读取边界。独立 ACP 对照不含网关权限回读或 `chat.done` 证据。

## 受管入口

受限桌面会话仅在 Darwin arm64、`session_model`、`ApproveReads` 且启用工具时使用 Droid 1.9.0 v2。宿主和启动端校验固定摘要，沙箱仅允许启动该可执行文件。项目设置和原生技能目录为只读，会话选集从已验证的私有链接视图追加。未受审的启动入口仍应用原权限门禁。

网关使用请求专属的 HTTP 中继访问宿主选中的 stdio 服务。ACP 的 `_meta.factory` 提供服务与工具身份，宿主先校验当前 ACP 会话和唯一调用 ID，再沿交互权限层获取单次确认。中继只能消费匹配服务、工具和参数的本次确认；展示标题只由服务与工具名生成。受限入口用请求专属的 Droid 状态为每次请求重建原生会话，沿宿主的会话历史重放定制。历史原生会话恢复与分叉锚点不在该入口的支持范围。

用生产沙箱规则另行运行四次独立 ACP 会话，模型请求为 3/6/3/5，MCP 执行为 0/1/0/0，拒绝返回 `cancelled`。只为该对照分开模型与 MCP 监听端口。额外原生 `Read` 对照直接请求未选技能附件，模型收到访问拒绝，未取得附件标记。

### 真实网关验收

2026-11-21，使用获验共享构建的网关快照，SHA-256 为 `09c49dfa8efa59deebb31d40ddd6f4533e7670a09f71baef33e77f10f5284453`。从槽位 6 构建产物复制独立快照到持有的槽位 11，并与共享验收报告的摘要和本次接线标识比对。探针的两项 Cargo 目录均为 `target/rust-slot-11`。Droid 保持 1.9.0 v2 固定摘要和原生延迟工具。真实模型请求包含 `reasoning_effort=high`，因此为夹具模型声明对应能力，沿原有中继校验。

| 阶段 | ShiJu 会话 | 带工具的模型请求 | MCP 下游执行 |
| --- | --- | ---: | ---: |
| 基线 | `ses_e040f3d23ffeh42vKxOORmWYQc` | 3 | 0 |
| 挂载 | `ses_e040ed55dffe90FhTeHBvq00S1` | 6 | 1 |
| 撤除 | `ses_e040e715fffeKdj0OF7Rg2z4fx` | 3 | 0 |
| 拒绝 | `ses_e040e33d7ffeZFc5TLAmTKwzU3` | 5 | 0 |

四轮各一个 `chat.done`、零个 `chat.error`。网关回读 `session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017`、Droid 和未请求完全访问，结构化结果见 `target/acp-skill-coexistence/droid.json`。挂载轮原生技能激活及附件 ID 为 `call_42d96b336d858bdf_0`、`call_b45aa8c8e257eb30_0`，选中技能和附件为 `call_896f7b7b010f3d1b_0`、`call_3a1d6cb237161255_0`，MCP 为 `call_df44f6318ed2a7b5_0`。拒绝轮的 `call_0598b9152e066f19_0` 对应原生失败事件，禁止下游调用并且无后续模型请求。

无正文的拒绝对照同样拒绝并保持下游执行为零，网关对空的取消回合报告 `模型未返回内容`。正式探针在批准和拒绝的相同模型响应中先输出检查提示，再发出一次只读 MCP 调用，保留原生取消和拒绝证据。退出码为 0。

使用槽位 10 已构建的 `llm-runtime` 测试二进制运行过滤器 `acp_skill_coexistence_droid`，两个用例通过，分别覆盖仅追加选集和拒绝链接形式的项目设置。

同一槽位运行 `cargo test --locked -j 1 -p acp-client --lib acp_skill_coexistence_droid -- --test-threads=1`，退出 0，1 个用例通过，覆盖同会话的原生工具身份、单次确认及调用重放拒绝。
