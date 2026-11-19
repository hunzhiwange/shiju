# iFlow 官方工具与 MCP 共存复验

2026-11-19 在 Darwin arm64 直连原包及受管 iFlow 0.5.19。两版均通过无挂载、挂载、撤除三轮。挂载轮的官方读文件和目标 stdio MCP 工具按原调用 ID 回传，撤除后的新会话对目标服务零访问。原生单次拒绝及逐服务排除均阻止调用。宿主逐服务名单的投影缺口待任务 43 回归。

## 固定入口与权限

- 注册 key `iflow`，预设 `acp-iflow`。原包 `@iflow-ai/iflow-cli@0.5.19` 经固定 npm lock 安装，归档 SHA-256 为 `4ac4ec1d6e52362c5794bb07213886bafb554d4036752b8af8c8ca08462ab365`，与合约 SRI 对应。用 Node 24.22.3 执行私有 `node_modules/.bin/iflow --experimental-acp`；链接指向 `bundle/entry.js`。原包 `entry.js` 和 `iflow.js` SHA-256 分别为 `81de70778e7276b6410ae91a84e1481f88e4087984b95f882873e8e3024bc2c1`、`3bc5c8873d4e5691322de7fac1b5cd530c2f797107c3c9beb11afd8e99cedddc`，与受审补丁输入一致。[基线审计](../1.md#iflow) · [补丁](../../../../../patches/acp-model-source/iflow/apply.py)
- 受管 v1 用仓库 `install.py`、固定 lock、私有 npm 缓存执行安装，退出 0。入口追加 `--shiju-model-source`，两项 SHA-256 分别为 `55710b7cfea2e5aefebfe0acec4eaf3e647d34538ee5835a3d724a49853d1fa6`、`3f485f9ca61edd2d1594cc6b5374a8108002ab0ee3488f03bdbcfc4bef89ef66`，与受管合约一致。两入口 `--version` 均为 0.5.19。[安装脚本](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/install.py) · [制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 原包通过原生用户配置选择 OpenAI 兼容模型。受审入口沿用 `IFLOW_HOME`、`IFLOW_*`、模型锁及私有历史，空技能根禁用技能。两版实际模型均为私有回环 `fixture`，协议是 `/v1/chat/completions`。合成凭据和两种随机读取标记均在私有目录，提示和模型响应未预置标记。沙箱仅允许回环联网，阻止读写真实用户的 iFlow 配置。
- 握手为 ACP v1、`iflow-agent 0.5.19`，声明 `loadSession=true`，未声明 HTTP 或 SSE MCP。通用注册入口和受审选模仅在本机平台取证。每次新建及加载后主动发送 `session/set_mode`，回执均为 `success=true`、`currentModeId=default`。官方工作区读取由 iFlow 原生执行，MCP 挂载轮在 `session/request_permission` 使用 `allow_once`，保持原审批路径。[权限同步](../../../../../crates/libs/acp-client/src/client/iflow_access.rs) · [选模驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/iflow.rs)

## 三轮与结果关联

受管版较原包少一个 `Skill`，与空技能根的隔离设置一致。

| 入口 | 轮次 | ACP 会话 | 模型目录数／请求数 | 工具、执行方及 ACP／模型调用 ID | 目标服务请求 | 提示终态／进程退出 |
| --- | --- | --- | --- | --- | --- | --- |
| 原包 | 无挂载 | `61108048-526e-40fc-addc-60627902402b` | 22／2 | iFlow `read_file` / `c-native` 完成 | 0 | `end_turn`／143 |
| 原包 | 挂载 | `46364100-e239-4bad-8ee5-efc21130c91c` | 23／3 | iFlow `read_file` / `c-native`、`coexist_probe.read_probe` / `c-mcp` 均完成 | 初始化、列目录、执行各 1 | `end_turn`／143 |
| 原包 | 撤除 | `86fc22ff-4dfc-446d-b43e-957d6b3a6867` | 22／2 | iFlow `read_file` / `c-native` 完成 | 0 | `end_turn`／143 |
| 受管 | 无挂载 | `09369d5f-b669-4d86-9446-321bb0e1e38b` | 21／2 | iFlow `read_file` / `c-native` 完成 | 0 | `end_turn`／143 |
| 受管 | 挂载 | `46dc2661-7eb6-4476-92ef-7d0475273c73` | 22／3 | iFlow `read_file` / `c-native`、`coexist_probe.read_probe` / `c-mcp` 均完成 | 初始化、列目录、执行各 1 | `end_turn`／143 |
| 受管 | 撤除 | `b2477bef-607f-490a-a564-0edef7fcdc17` | 21／2 | iFlow `read_file` / `c-native` 完成 | 0 | `end_turn`／143 |

每个挂载轮的 MCP stdio 请求分别为 `initialize` ID 0、`tools/list` ID 1、`tools/call` ID 2。两类 ACP 工具更新走完 `pending`、`in_progress`、`completed`。第三次模型请求按 `tool_call_id` 同时读回当轮工作区文件和项目外附件的独立标记。ACP 提示结束后探针关闭 stdin，再发 SIGTERM 清理进程组，表中 143 是启动器退出状态。

额外静态配置对照会话 `7fd44e11…`（原包）与 `f628fb13…`（受管）均以 `end_turn` 结束。两版均在模型目录保留静态 `static_probe.read_static` 和官方 `read_file`，并采用会话 `coexist_probe.read_probe`。初始配置阶段连接了原静态同名服务并列目录，覆盖后模型目录没有它提供的 `read_replaced`，原服务零执行。实际执行归会话服务，MCP ID 2，与 `c-mcp` 的回传结果匹配。保留原生合并顺序。

## 拒绝、取消与恢复

下表每对会话按原包、受管顺序，进程均在 ACP 终态后由探针关闭，退出 143。

| 样本 | ACP 会话前缀 | 处理结果 |
| --- | --- | --- |
| 单次拒绝 | `682cdfa6`／`a5b4f2a7` | iFlow 审批 ID `c-mcp` 选择 `reject_once`。服务完成发现但零 `tools/call`。下次模型请求按 `c-mcp` 收到用户取消工具的错误，提示 `end_turn`。官方读文件完成。 |
| 原生逐服务排除 | `0bcf31b3`／`f2c1000b` | `session/new.mcpServers[].excludeTools` 排除 `read_probe`。服务完成发现，模型目录省略该工具。夹具发出待拒调用 `c-mcp`，在 iFlow 注册表被拒，零服务执行。模型收到同 ID 的工具未注册错误并正常续轮。显式同时包含及排除的另轮 `9e79f877`／`aac1284c` 结果相同。 |
| 取消提示 | `ae6e5122`／`2bcb5671` | 回环收到模型请求后、回包前发送 `session/cancel`，返回 `cancelled`，零工具更新、零服务请求。 |
| 重连加载 | 挂载原 ID | 另启进程对挂载轮原 ACP ID 发送 `session/load`。重传服务时 `read_file` 和 `read_probe` 再次实际执行并按 `c-native`、`c-mcp` 回传新值。清空服务再加载同 ID，仅官方读文件执行，原服务零访问，两次均 `end_turn`。 |

## 宿主投影缺口

仓库当前配置解析将 `allowedTools`／`deniedTools` 保存为宿主管理字段。共享 `acp_defaults` 对部分后端生成过滤策略，并对部分后端显式拒绝带过滤的挂载。iFlow 分支仍将所选服务对象直接传给 ACP，缺少原生 `includeTools`／`excludeTools` 的投影或对应拒绝。宿主会话名单及逐服务名单的实际网关行为仍待复跑。[解析](../../../../../crates/libs/acp-config/src/mcp_servers.rs) · [宿主投影](../../../../../crates/libs/session-engine/src/acp_defaults.rs)

以当前宿主投影形状重放直连 ACP（`df508781`／`07305b7c`），以及直接附加宿主管理字段 `deniedTools`（`5620e76d`／`a2891125`），均仍声明 `read_probe`。在独立本地夹具单次允许后，调用实际抵达服务 ID 2，回传附件标记并以 `end_turn` 结束。拒绝的首个断点是宿主策略没有进入 iFlow 读取的逐服务字段。该差异在原包和受管版一致。任务 43 负责共享接线或显式拒绝，并用当前网关重放带 `allowedTools`／`deniedTools` 的项目配置，核对拒绝时目标服务零执行。

## 验证边界与复验

当前回执来自真实 iFlow 入口、真实 stdio 执行和确定性回环模型。历史网关三轮和真实模型的 0／70／0 次目标调用另见原验收。当前未运行网关、真实供应商、跨平台、独立 `session/resume` 或 Cargo。原包和受管源码差异及运行目录表明，本样本的原生工具与 MCP 路径无需适配器补丁。[历史网关证据](../../mcp-acceptance/iflow.md#rustcodegraph-1211-严格三轮验收)

本机私有目录 `/private/tmp/iflow-coexist20.eoIree` 保留 `probe.py`、`merge.py`、`policy.py`、校验脚本 `verify.py` 与逐轮 JSONL。以仓库根目录和 `original`／`managed` 为参数运行三个探针，`NODE_BIN` 指向 Node 可执行文件。运行 `python3 /private/tmp/iflow-coexist20.eoIree/verify.py` 核对两版各 13 个样本，退出 0。自有适配器、stdio 子进程和回环监听均已关闭，私有设置、历史与读取标记已清理。新增回执通过任务指定的 `git diff --no-index --check /dev/null` 空白检查。
