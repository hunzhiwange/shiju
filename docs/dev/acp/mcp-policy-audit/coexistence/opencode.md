# OpenCode 1.19.5 原生工具与 MCP 共存回执

2026-11-19，Darwin arm64。固定原包和受管重签件均通过独立进程的无挂载、挂载、撤除三轮。保持同一个进程时，两版的空列表新建、加载和恢复均保留旧服务，还能实际读取更换后的附件。任务 29 保留待回归。

## 制品、入口与授权

- 注册 key 为 `opencode`，本地入口为 `opencode acp`，未找到本地二进制时才回退 `npx -y opencode-ai@latest acp`。本次从本机受管缓存复制两份固定二进制到私有目录，各自直接执行 `<绝对路径>/opencode acp`。原始 `opencode-darwin-arm64/bin/opencode` SHA-256 为 `feaafead8cd563af0c2dcb386b0757e4355e327ebb6bd1aaad5440fe2f83596c`，受管 v1 `reviewed/opencode` SHA-256 为 `1a1f10028d3edebddd6df10a465cfa79b58d710875173dbd32137a8b115daa2c`，均匹配合约，重签件通过 `codesign --verify --strict`。两版自报 1.19.5、ACP v1、load/resume 与 HTTP/SSE MCP 能力。冷装在私有 npm 缓存内尝试后停止，本次证据来自已校验的缓存副本。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L583) · [制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L285) · [受审补丁](../../../../../patches/acp-model-source/opencode/apply-skills.py)
- 原包由私有 `opencode.json` 选模。受管版按现有驱动形态使用 `OPENCODE_CONFIG_CONTENT`、`OPENCODE_DISABLE_PROJECT_CONFIG=1`、私有 XDG 及 `OPENCODE_TEST_HOME`，令牌通过环境引用。两版都隔离 HOME、配置和状态，关闭模型拉取及自更新，主、辅助和内置 agent 都指向同一回环 Chat Completions 模型。ACP 回读 `session-provider/fixture` 和原生 `build` 模式。未申请全访问或免审批。正向轮沿用默认权限，拒绝和取消轮只给目标工具配置 `ask`，由探针客户端作一次性响应。受管版经 `SHIJU_OPENCODE_SKILL_ROOT` 露出合成技能，原包经私有原生配置目录扫描同一技能，三轮均观察到 `fixture-skill` 及官方 `skill` 工具。[受管选模驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/opencode.rs) · [原生选模验收入口](../../../../../patches/acp-model-source/opencode/recipe.json)
- 两版使用同一个私有工作目录。官方 `read` 由 OpenCode 直接读取项目内只读文件。目标服务由 `session/new.params.mcpServers` 原生 stdio 挂载，使用仓库 `acp_mount_integration.py` 的真实只读 RPC 实现读取项目外附件，薄包装只记入站方法和 RPC ID。两份文件的随机标记逐轮生成，只从实际工具结果进入模型请求。没有关闭官方目录或裁剪 MCP 目录。真实模型、ShiJu 网关、浮动入口和其他平台未执行。历史真实网关证据单列，历史权限 `client_only` 未用于本次探针。[公共口径](../../ACP验收.md) · [历史回执](../../mcp-acceptance/opencode.md)

## 独立进程三轮

表内工具 ID 在模型和 ACP 一致。官方名为 `read`，MCP 的模型及 ACP 名为 `coexist_probe_read_probe`，服务内名为 `read_probe`。

| 入口 | 轮次 | ACP 会话 ID | 官方读取 ID | MCP 工具 ID / 服务 RPC ID | 服务调用增量 | ACP / 进程退出 |
| --- | --- | --- | --- | --- | --- | --- |
| 原包 | 无挂载 | `ses_e0d36bce8ffeJZwbz4EygdlrjI` | `call_local_before_read` | 无 | 0 | `end_turn` / 0 |
| 原包 | 挂载 | `ses_e0d36b6e9ffeB6ztL5XTURzAd9` | `call_local_mounted_read` | `call_local_mounted_mcp` / 2 | 1 | `end_turn` / 0 |
| 原包 | 撤除 | `ses_e0d36b091ffe5zSi92JJ86uSWs` | `call_local_removed_read` | 无 | 0 | `end_turn` / 0 |
| 重签件 | 无挂载 | `ses_e0d366f49ffejME2KVTYzl7Bmp` | `call_reviewed_before_read` | 无 | 0 | `end_turn` / 0 |
| 重签件 | 挂载 | `ses_e0d36682effeivJ9mnFy22Cxw1` | `call_reviewed_mounted_read` | `call_reviewed_mounted_mcp` / 2 | 1 | `end_turn` / 0 |
| 重签件 | 撤除 | `ses_e0d365fe1ffeJylq5STDtgWX3f` | `call_reviewed_removed_read` | 无 | 0 | `end_turn` / 0 |

两版的模型目录均为 10 / 11 / 10 项，新增项仅为目标 MCP 工具。挂载轮服务实收 `initialize`（0）、`tools/list`（1）、`tools/call`（2）。所有表内调用均有 ACP `pending`、`in_progress`、`completed`，文件标记按原 `tool_call_id` 进入后续模型请求。挂载末轮同时收到两个标记。撤除轮沿用同一服务名与附件位置，目标服务无启动、声明或调用。

## 拒绝、取消和恢复

- 拒绝轮 `call_local_deny_mcp` 和 `call_reviewed_deny_mcp` 各在 ACP 审批请求 ID 0 收到 `reject_once`。ACP 同 ID 报 `failed` 并返回拒绝原因，服务只完成握手与列目录，`tools/call` 为 0。提示以 `end_turn` 结束，进程退出 0。回环未收到该拒绝结果的后续模型请求。
- 取消轮 `call_local_cancel_mcp` 和 `call_reviewed_cancel_mcp` 各在审批请求 ID 0 等待时收到 `session/cancel`，宿主答复审批取消。ACP 同 ID 报 `failed`，提示终态 `cancelled`，服务调用为 0，进程退出 0。本轮覆盖执行前取消。
- 新进程沿用各自挂载轮的原 ACP 会话 ID 和私有状态。带挂载列表的 `session/load`、`session/resume` 各实际调用一次 MCP（服务 RPC ID 2），两类读取均按 ID 续轮。传空列表的两种恢复均有新的官方读取，目标工具零声明、零调用。四类恢复均 `end_turn` / 0。旧历史的工具结果未计入新执行。

## 同进程撤除断点

两版分别启动独立挂载进程。首次读取成功后，在同一进程内更换附件，并以 `mcpServers: []` 调用下表所列方法。首个断点在空列表请求后的首次模型目录，仍声明 `coexist_probe_read_probe`。所有样本的官方 `read` 均完成，工具 ID 为表中 MCP ID 的 `_mcp` 后缀改为 `_read`。残留 MCP 调用均由原 stdio 服务执行，新增服务 RPC ID 3。ACP 同 ID 完成，模型续轮收到更换后的附件值，六轮均 `end_turn` / 0。

| 入口 | 空列表方法 | 之后的 ACP 会话 ID | 残留 MCP 工具 ID |
| --- | --- | --- | --- |
| 原包 | `session/new` | `ses_e0d358345ffemklV60vSKMnCAE` | `call_local_same-new_empty_mcp` |
| 原包 | `session/load` | `ses_e0d357e17ffecv92zl4GGyhuAN` | `call_local_same-load_empty_mcp` |
| 原包 | `session/resume` | `ses_e0d357639ffeF0QuPeWhcdonkW` | `call_local_same-resume_empty_mcp` |
| 重签件 | `session/new` | `ses_e0d356b72ffemHvTA8V9fppyL2` | `call_reviewed_same-new_empty_mcp` |
| 重签件 | `session/load` | `ses_e0d356654ffer1UGHdBpoS1426` | `call_reviewed_same-load_empty_mcp` |
| 重签件 | `session/resume` | `ses_e0d355ff4ffeCJYi1iMdFNRM3O` | `call_reviewed_same-resume_empty_mcp` |

原包复现相同问题，挂载正向未见受管适配器裁剪官方目录。宿主缓存键包含 `mcpServers`，空表和非空表应取得不同的键。实际网关是否切换并退役旧进程仍需确认。静态核对还显示，共享默认服务投影对 OpenCode 缺少逐服务 `allowedTools` / `deniedTools` 的下游表达或显式拒绝，本轮未发送带名单的网关请求。任务 43 负责按当前宿主路径复验撤权与名单。此次只提交回执，未修改运行时或执行 Cargo。[宿主缓存键](../../../../../crates/libs/llm-runtime/src/acp/session.rs#L115) · [默认服务投影](../../../../../crates/libs/session-engine/src/acp_defaults.rs#L110) · [任务 43](../../../../../.plans/2026-11-19-acp-工具共存/43-集成与证据收口.md)

原始记录、两版私有副本及探针保存在本机 0700 目录 `shiju-opencode-task29.T5gbDv`。从仓库根目录用 `PYTHONDONTWRITEBYTECODE=1 python3 <私有目录>/final/probe.py <local|reviewed> <轮次>` 运行，复跑时在新私有目录布置同名制品与探针。依次使用 `before mounted removed deny cancel load resume load-mounted resume-mounted`。同进程样本改用 `rotate/probe.py` 并指定 `same-new`、`same-load` 或 `same-resume`。两探针 SHA-256 分别为 `9e085dd02948c93be67159a6829f75d32f30e2b88fd1077843a9e268ffbfd8cb`、`8158572e3d169ae9c5ce75b273ed85bd3409a9ccf1e6586f9f9d92041d0c69c0`。专属进程和回环服务均已退出。
