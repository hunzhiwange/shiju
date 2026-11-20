# Auggie 技能共存验收

状态：已阻塞。网关在无挂载轮、首个模型请求前返回 `requires_full_access`。技能挂载能力门禁继续拒绝 Auggie。

## 制品与网关

- Darwin arm64，受管 `@augmentcode/auggie` 0.37.1，供应商提交 `e8d38687`，安装修订 `auggie-0.37.1-model-source-v1`。用仓库安装器核验固定包，隔离启动的 `--version` 与预期相同。
- `augment.mjs` 的 SHA-256 为 `1e39bfa85bb3ebe44afb6d627bdd3202550508d5224c837dc4370576f555cb18`，`session-model.mjs` 为 `d50926db8682babc44174201b0591e7b309e00e5f7f5989c924f610e7b1a7081`。
- 复用 Cargo 槽位 10 的已验网关，核对二进制包含相同受管入口摘要。构建和目标目录均为仓库根下的 `target/rust-slot-10`。网关 SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。本次变更仅涉及探针和回执，复用网关产物，未运行 Rust 构建或单测。

探针从仓库根执行，使用私有配置、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false`。

```sh
PYTHONPATH=scripts python3 -c 'from cargo_slot import run; import sys; sys.exit(run(sys.argv[1:], number=10))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent auggie --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/auggie.json'
```

退出码为 1。无挂载轮 0 个 `chat.done`、1 个 `chat.error`、0 个模型请求、0 次下游 MCP 调用。宿主对 Auggie 要求已有完全访问授权，因此本次没有模型或权限回读。挂载、撤除和拒绝对照均未进入。结构化证据位于 `target/acp-skill-coexistence/auggie.json`。此前 [Auggie MCP 验收](../mcp-acceptance/auggie.md) 同样记录为 `client_only`，其完全访问会话属于另一项验收。

## 独立 ACP 诊断

同一制品直连真实 ACP，在独立私有项目的 `.augment/skills` 放置随机原生技能。回环模型首次看到原生技能名称及 `SKILL.md` 路径，正文和附件均由原生 `view` 读取。3 个模型请求完成 `end_turn`，调用 ID `call_08d073a03051df2b9229eba2` 和 `call_caadc1a5ee987f9df0cf4e0e` 分别关联正文和附件。无 MCP 调用，没有 ACP 审批请求。另将目标技能放入独立的本地插件或本地市场目录，分别用 `--plugin-dir` 启动 ACP。两种目录都在首个模型请求中缺少选中技能，夹具拒绝继续。

独立原生 stdio 对照将只读服务通过 `session/new.mcpServers` 注入。实际模型声明为 `read_probe_coexistence`，参数对象为空。调用 `call_3` 的读取结果以相同的 `tool_call_id` 进入 Chat Completions 续轮。默认启动未向宿主发出 `session/request_permission`。在另一独立进程仅追加 `--permission read_probe_coexistence:ask-user`，同一测试调用向 ACP 宿主发送 `session/request_permission`，标题和空参数与模型声明匹配。返回 `reject_once` 后，模型续轮收到关联拒绝结果，下游审计计数为 0。此选项只覆盖指定 MCP 工具。上述直连观测保留供应商的原生工具，未改写项目设置，也未授予网关完全访问。

继续条件是审定能约束原生工具的启动路径，将实测的 MCP 审批接入网关，并验证会话技能目录追加。供应商入口参考 [Auggie 0.x 技能](https://docs.augmentcode.com/cli/v0/skills.md)、[插件和市场](https://docs.augmentcode.com/cli/v0/plugins.md)及[工具权限](https://docs.augmentcode.com/cli/v0/permissions.md)。

## 续查受限启动

复核了当前共享源码。网关在准备技能视图前检查 Auggie 的完全访问授权，ACP 进程入口也独立检查实际审批模式。相邻适配器拥有各自的受限分支，Auggie 的门禁和 v1 制品合约仍相同。首个断点由这两处宿主检查决定，沿用原网关报告，未重复构建或运行网关探针。[请求入口](../../../../crates/libs/llm-runtime/src/acp/request.rs) · [进程入口](../../../../crates/libs/acp-client/src/client/process.rs)

Darwin arm64 私有副本用仓库合约校验 0.37.1 的输入 SHA-256，将会话根追加到供应商原生扫描器。候选入口 SHA-256 为 `69e8b7e0833201c84e2772ae5dc5b051ab2a1b91645b4dfe9b975285c3ecad9f`，Node v24.22.3 的入口 SHA-256 为 `ba7073582822078d8c90bca3facacc46690bc5366fab4b815aeef9148cd6e854`。模型只替换响应，原生 `view` 通过同一个工具协议读取正文和附件。项目 `removedTools` 与用户技能禁用配置继续生效。原生 ACP 接收宿主 HTTP 中继，实际读取由独立 stdio 服务执行。

| 独立 ACP 会话 | 终态 | 模型请求 | 审批结果 | 下游 MCP |
| --- | --- | ---: | --- | ---: |
| 基线 | `end_turn` | 3 | 无 | 0 |
| 挂载 | `end_turn` | 6 | 单次批准 | 1 |
| 撤除 | `end_turn` | 3 | 无 | 0 |
| 拒绝 | `end_turn` | 6 | 单次拒绝 | 0 |

四轮均取得原生技能正文和附件的调用 ID，挂载和拒绝轮另取得选中技能的调用 ID。模型首轮只看到相应的目录元数据。每轮另验证沙箱阻止读取其他私有目录、写入工作区、启动 shell、再次执行 Node 以及连接未放行的端口。探针退出 0，报告位于 `target/acp-skill-coexistence/auggie-sandbox.json`。

```sh
PYTHONPATH=scripts python3 -m acp_skill_coexistence.auggie_sandbox \
  --package /path/to/verified/node_modules/@augmentcode/auggie \
  --output target/acp-skill-coexistence/auggie-sandbox.json
```

第一项生产差异来自项目自带 MCP。将同一个只读 stdio 服务写入私有项目 `.augment/settings.json`，固定版普通 ACP 启动能发现该工具，单次批准后下游计数为 1。同设置和同入口放进只读沙箱后，模型仍正常结束，原生进程无法启动 stdio，工具未进入目录，下游计数为 0。受限分支若要保留这类项目原生 MCP，需要明确允许哪些项目服务经宿主登记，以及如何为它们绑定工作区、工具目录和权限。若产品在受限分支只承诺宿主已选择的 MCP，也需要确认该范围。

候选在独立 ACP 验证审批回调。正式网关的消费型中继当前以 `mcp__服务__工具` 匹配批准，Auggie 已实测回调标题是 `工具_服务`。接线时需要从受信服务目录建立无歧义映射，把批准绑定到同一会话的具体服务、工具和参数，再由中继消费。生产仍使用原入口、权限门禁和挂载能力，网关三轮、回读及拒绝对照未验收。继续条件为确认项目原生 MCP 的受限投影范围，并评审候选入口与宿主中继的权限接线。
