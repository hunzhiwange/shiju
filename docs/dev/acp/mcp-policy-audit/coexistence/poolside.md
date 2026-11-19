# Poolside 原生工具与 MCP 共存复验

2026-11-19，Darwin arm64 上用 Poolside 1.0.16 受管 v2、真实 Docker 沙箱和回环模型完成无挂载、挂载、撤除三轮。挂载轮的官方读取和图谱结果均按原调用 ID 进入模型续轮。撤除轮的目标工具目录为空，ACP 与模型轨迹均为目标服务零调用。

## 入口、制品与权限

- 注册 key 为 `poolside`，预设为 `acp-poolside`，普通注册为通用 ACP 入口 `pool acp`。受管入口实测为私有目录下的 `pool acp`，拉起 `pool-native acp --sandbox required`。安装脚本下载在 120 秒后以 curl 28 退出，随后取已缓存的固定归档，核对归档和布局，再按当前安装脚本组装私有副本。归档 SHA-256 为 `0932af3eb2b57a863acacb664ec8b2b1d3a76c2570a788b086001608cc585f74`，入口自报 1.0.16。[入口与历史断点](../4.md#poolside) · [安装合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 四项文件通过当前合约 SHA-256 校验。原生程序为 `5dcb0162fd21a535e4f67117125e7d205e27a6881862aa223d0424082848f874`，启动器为 `9f4a7ac15e4d7d5804ad90ed326183e100e071b8ba977f37603893c4bb29145c`，HTTP 桥为 `8aea18ecdbc23070a2fc6a9eb5245f095219ac76e9390f4ec2b32e1dd95db136`，共享 stdio 模块为 `e6870d58c6c59cb1f1d224a66cdaa8455b34057249912b7bce5b9e21f95a074a`。
- 每轮使用独立私有工作区、HOME、配置和原生进程。ACP v1 握手报告 `pool-acp` 1.0.16、`loadSession=true`、`mcpCapabilities={}`。新会话回读 Docker 沙箱 `required / enabled / read-only`、模式 `default`（Always ask）及协作模式 `build`。挂载声明使用真实 RustCodeGraph 1.2.10 stdio，绝对命令、空环境及 `--no-watch`，对私有 RueJS 副本建立索引。桥在创建新会话时原子替换自己的 `mcp_servers` 设置。三轮均声明原生 `read`、`write`，固定禁用表涵盖 shell、edit、subagent 等工具。[启动器和写入审查](../../../../../patches/acp-model-source/poolside/launcher.py) · [私有设置与审批](../../../../../patches/acp-model-source/poolside/mcp_bridge.py)
- 直连探针复现受审模型环境，使用 `VW_ACP_LAUNCH_REFERENCE` 和 `SHIJU_POOLSIDE_RELAY_*` 指向动态回环端口和合成鉴权。原生 `session/set_config_option` 的 `model` 回读为 `poolside-31-probe`，模型请求实际命中 `/v1/chat/completions` 且使用该 ID。独立边界探针校验请求 Bearer 值，接受绑定模型后续轮；选择 `unbound` 返回 `-32603`。中继地址附加路径时启动器在握手前拒绝，进程退出 1。Rust 驱动的受审入口、参数、环境和工具可用性检查为源码复核。[会话模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/poolside.rs)

## 三轮结果

| 轮次 | ACP 会话 ID | 工具、调用 ID 与执行方 | 模型请求数 / ACP 终态 / 进程退出 |
| --- | --- | --- | --- |
| 无挂载 | `01a1f2cb-a6fb-7203-bcdd-051f1ee2af95` | `read` / `probe-read` 由 Poolside Docker 沙箱完成，回传 `app/app.tsx`。MCP 目录为空、零调用。 | 2 / `end_turn` / 0 |
| 挂载 | `01a1f2cb-d16c-7b52-8504-b1c59bb0c70d` | `read` / `probe-read` 由 Poolside Docker 沙箱完成。`ruejs_graph_probe__rustcodegraph_callers` / `probe-callers` 由受管 HTTP 桥转给宿主沙箱中的 RustCodeGraph stdio，返回 `mountRueSiteApp  app/app.tsx:39`。 | 3 / `end_turn` / 0 |
| 撤除 | `01a1f2cc-00cd-7aae-9b4d-3ea9e4ac689b` | `read` / `probe-read` 由 Poolside Docker 沙箱完成。新进程的目录只有原生工具，目标服务零调用。 | 2 / `end_turn` / 0 |

表列工具均以原模型 ID 进入 ACP `pending` 和 `completed` 事件，再由同 ID 的 `tool_call_id` 回到模型。挂载轮原生审批请求 ID 为 `1`，其 `toolCallId` 为 `probe-callers`，宿主选择 `allow_once`，第三次模型请求带有文件与图谱两项结果。独立 stdio JSON-RPC ID 与对照轮的服务侧审计本轮未采集。

## 审批、恢复与入口边界

- **单次许可及撤权。** 拒绝轮 `probe-callers` 以 `failed` 完成，模型无图谱结果，提示以 `end_turn` 结束，进程退出 0。审批期间发送取消后再发迟到批准，`probe-callers` 只有开始与审批事件，无图谱结果或下一次模型请求。现有启动器把取消终态映射为暂存写入拒绝的 `-32000`，进程退出 0。挂载进程在第二次 `session/new`、`session/load` 与 `session/resume` 均返回 `-32000`，原挂载会话随后仍能执行读取和图谱。未另跑未挂载历史恢复。已有 3 项 Python 定向测试通过，命令为 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s patches/acp-model-source/poolside -p '*test.py'`，退出 0。
- **暂存写入。** 两次合成写入在 Docker 内以 `write` / `probe-write` 产生已完成暂存差异，随后由启动器向探针宿主发出 `fs/write_text_file` / `shiju-poolside-write-1`。批准轮在宿主实际写入前验证目标尚未落盘，宿主仅写私有副本的 `proof.txt`，提示 `end_turn`。拒绝轮目标始终缺失，提示返回 `-32000`。两进程均退出 0。另向现有启动器的桥注入合成的原生直接写入回调 `native-direct`，收到 `-32000`，宿主收到零写请求。
- **原版入口。** 使用同摘要原版 `pool-native acp --sandbox required`、私有设置和回环模型。原生接受带 stdio `mcpServers` 的建会话请求，但模型只见 `read/write`，原生读取完成，两次模型请求后 `end_turn`，退出 0。原版私有配置可选模，受管会话模型使用已验证入口并要求 Docker。早期受管拒列表与启动期 HTTP 成功诊断保留在原验收记录。[原版断点与 HTTP 诊断](../../mcp-acceptance/poolside.md#失败边界与复跑条件)
- **普通宿主入口。** 已提交的 `process.rs` 仍对 `poolside` 无条件执行受审入口校验。工作区差异移除此项并在有模型绑定时纳入私有环境分组。槽位 1、3、6、9、10 的缓存 `acp` 已实测用私有项目覆盖调用原版并成功建会话（`poolside sessions new --deny-all`，退出 0）。缓存二进制缺构建来源证明，结果仅代表这些缓存产物。当前已提交源码和工作区源码的差异以源码为准，本轮未分别重编。受审模型驱动仍独立要求验证入口摘要。本轮没有修改共享 `process.rs`。[启动检查](../../../../../crates/libs/acp-client/src/client/process.rs)
- **证据范围。** 本轮使用真实适配器、Docker 和 stdio 服务，模型侧为本地回环。未复跑网关 SSE、真实供应商或其他平台，未运行 Cargo。正式网关先前的挂载轮共有 33 次 MCP、15 次官方读取成功，历史证据独立。边界补测曾遇到 Docker bind 路径暂时不可见，换用新私有目录后复验通过。未发现适配器意外裁剪，交付本回执。[历史网关验收](../../mcp-acceptance/poolside.md#新版标准下的真实模型验收)

复验结束后，按私有挂载来源确认并移除 5 个本轮停止的沙箱容器。私有证据在 `/private/tmp/shiju-poolside-31.9kxMss`。在该目录运行 `python3 probe.py` 可重跑直连三轮及拒绝、写入对照。新增回执按任务指定的 `git diff --no-index --check /dev/null` 检查空白错误。
