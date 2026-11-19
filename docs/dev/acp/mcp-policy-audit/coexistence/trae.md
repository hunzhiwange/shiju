# TRAE 0.121.3 工具共存复验

2026-11-19，Darwin arm64。受管版通过无挂载、挂载和撤除后的三次新会话。挂载轮的原生 `Read` 和 stdio MCP `read_probe` 实际执行，两条结果按各自的调用 ID 回到模型并续轮。原包能发现服务，缺可用的平台会话状态，首轮模型请求未发出。[共存口径](../../ACP验收.md) · [分组审计](../6.md#trae) · [历史网关验收](../../mcp-acceptance/trae.md)

## 制品、模型和权限

注册 key 为 `trae`，参数为 `acp serve`。本轮从已有受审缓存复制三项制品到私有目录，各项 SHA-256 均符合当前合同。原包实测版本 `0.121.3`，build `72fe00ff52f56b0eb11a37bd561d4f10a4a44354`。[注册表](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L628) · [制品合同](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L706)

| 文件 | SHA-256 |
| --- | --- |
| `traecli` 启动器 | `de3bb2e987468cfc57023dbf64b8e990ef7228a4253b6f7f0e5e50201d3f1f1c` |
| `native/trae-cli` 原包 | `7648c66ffb535e83e4b1f5bbd820f0384d77234763cfb32700b1be3289358758` |
| `native/trae-local` 受管本地组合 | `f77806ca15efea1c33ffc4c3c08ab3f1c57111883dd3313822e673791484adde` |

- 同一启动器按绑定装配入口。受管版用权限为 0700 的请求目录和 0600 的绑定，使用 `SHIJU_TRAE_MODEL_CONFIG`、`VW_ACP_LAUNCH_REFERENCE`、`VW_TRAE_RELAY_TOKEN`。由启动器校验并交给原有 macOS 网络沙箱，只能出站至本次回环模型端口。ACP 回读 `session-selected`，实际请求 `/v1/chat/completions` 的模型为 `coexist-probe`。原包使用独立 HOME、合成的本地模型配置及仅允许该轮回环端口的外层网络沙箱，无私有绑定或供应商凭据。各轮均独立启动，未复用历史租户、模型或权限值。[启动器](../../../../../patches/acp-model-source/trae/launcher.py) · [受管驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/trae.rs)
- 两版都握手 ACP v1，回读原生 `default`，无跳过审批参数。握手报告 `loadSession=true` 和 HTTP/SSE MCP 能力，实测传输为 stdio。`sessionCapabilities` 只声明 list，本轮未验证 `session/resume`。受管组合只在 Darwin arm64 受审，macOS Intel、Linux 的官方 Unix 安装入口和 Windows PowerShell 入口仅静态核对。[平台安装表](../../../../../crates/libs/config-acp-install/src/lib.rs#L152)

## 受管版逐轮证据

每轮在工作区文件和工作区外的 MCP 附件分别生成随机标记。回环模型只生成调用，续轮才收到实际读取值。挂载在同一个模型响应发出两类调用。17 项原生工具在三轮均一致，挂载只增加 `mcp__coexist_probe__read_probe`。原生 `Read` 由 TRAE 执行，MCP 由仓库只读验收服务执行，ACP 开始和完成事件与续轮 ID 均匹配。

| 轮次 | ACP 会话 ID | 执行与结果 | 终态 / 子进程退出码 |
| --- | --- | --- | --- |
| 无挂载 | `b3eb428e-56fd-4a11-a6f3-eed1b60ad9fe` | `Read` 的 `call_before_0` 完成并回到模型。目标服务零发现、零调用。 | `end_turn` / 0 |
| 挂载 | `7f650a1b-ee3a-4b8f-b035-da3ccd92cd8e` | `Read` 的 `call_mounted_0` 完成。MCP 的 `call_mounted_1` 经原生审批选 `allow`（单次）后完成。独立服务线记录 `initialize=1`、`tools/list=2`、`tools/call=3`。两条实际标记以各自 `tool_call_id` 回到模型。 | `end_turn` / 0 |
| 撤除 | `97fbd748-8c0e-41c3-8934-877c6559fe7d` | `Read` 的 `call_removed_0` 完成并回到模型。模型无目标声明，目标服务零发现、零调用。 | `end_turn` / 0 |

## 受管拒绝、取消和加载

| 操作 | ACP 会话 ID | 结果 | 终态 / 子进程退出码 |
| --- | --- | --- | --- |
| 显式拒绝 | `2e2801d1-c0d5-4e46-91eb-5df53f6d1780` | 对 `call_deny_0` 选 `reject`（`reject_once`）。同 ID ACP 更新标为 `completed`，负载是 `Tool call rejected by user`。服务仅发现，零执行。只有首个模型请求，无同 ID 模型续轮。 | `end_turn` / 0 |
| 取消 | `44220138-6a86-4d15-8f90-131c94e68e9b` / `9c40e840-4f58-4798-bbea-ab65cad3e66f` | `call_cancel_0` 等待审批时发 `session/cancel` 并回复审批取消。独立复测对 `call_cancel_only_0` 只发取消通知。两轮均仅发现，零服务执行、零模型续轮。 | 两轮均为 `end_turn` / 0 |
| 原生读取审批探测 | `b4796ed6-501a-44a3-92bb-0d7cb98a894f` | `call_native_deny_0` 读取私有验收目录中工作区外的合成附件，原生 `Read` 完成并回到模型。工作区内外样本均无原生读取审批请求，无法对该工具选择显式拒绝。 | `end_turn` / 0 |
| 新进程加载并重传 | 挂载轮原 ACP ID | 拷贝本轮私有会话状态，以原 ID 和原项目发 `session/load`，加载回读 `readonly: true`。原生 `call_load_mounted_final_0` 读取原项目的新标记，MCP `call_load_mounted_final_1` 单次获批后执行，服务 RPC ID 3。两条新结果各按原调用 ID 回到模型。 | `end_turn` / 0 |
| 新进程加载并清空 | 挂载轮原 ACP ID | 从同一原始状态加载，传入空 `mcpServers`。仅原生 `call_load_removed_final_0` 实读并续轮，目标工具从新目录移除，目标服务零访问。 | `end_turn` / 0 |

审批响应按 `optionId` 只选单次 `allow`。该版本还把 `session_level_allow` 标为 `allow_once`，本轮未选择它。这里是直接 ACP 证据，宿主审批选项映射、会话或服务名单及网关 SSE 未重放。

## 原包首个断点

空 HOME 的原包能初始化和建会话，首次提示报 `Models is required`。随后在三份独立 HOME 中配置选用合成的回环模型，均能建会话并选模。记录如下。

| 轮次 | ACP 会话 ID | 服务发现 / 执行 | 终态 / 子进程退出码 |
| --- | --- | --- | --- |
| 无挂载 | `2e0cc99b-e6b2-48ab-9b77-dadebbbb72dc` | 0 / 0 | JSON-RPC `-32603` / 0 |
| 挂载 | `af08764d-79a8-40c6-80fe-551ba6cc5303` | `initialize=1`、`tools/list=2` / 0 | JSON-RPC `-32603` / 0 |
| 撤除 | `6a8470ee-c0fb-4165-adc6-10f1b22c8a99` | 0 / 0 | JSON-RPC `-32603` / 0 |

配置后的三轮均在首次 `session/prompt` 返回 `failed to call agent: no value available in memory store`，回环模型请求数为零，因此两类工具均无调用 ID、审批或模型输出。历史固定版验收已在相同阶段记录原包的账号中间件门禁。有独立获授权的供应商会话状态后再用原包重放三轮。[固定版本历史](../../ACP验收.md#trae-cli-验收)

## 复验入口

- 原始记录和探针位于 `/private/tmp/shiju-trae37.2lIUUp`。从仓库根目录以 `PROBE_NATIVE=Read PROBE_ARGS='{"file_path":"WORK_FILE"}' python3 <私有目录>/probe.py mounted` 运行单轮。复跑用新的私有目录放置探针、仓库路径标记 `repo` 和经核对的 `runtime`，避免覆盖旧轮次证据。先跑 `before`、`mounted`、`removed`。加载探针的第二、三参数是新挂载会话的 ACP ID 与原工作区路径。原包入口依次用 `official-config`、`official-mounted`、`official-removed`。所有服务只处理临时合成数据。
- `verify.py` 逐轮核对实际标记、工具归属及 ID 配对，退出 0。现有绑定权限和真实 ACP 模型线路的两项定向 Python 测试均通过。首个探索夹具曾在不含 usage 的模型流上触发原生 panic，后续使用包含 usage 的完整 Chat Completions 流并显式选模。此次没有使用真实供应商模型或运行 Cargo。
- 受管正向目录与执行未见无意裁剪。原生拒绝回执和取消终态的上述差异已保留，原包完整轨迹仍待授权实测。本轮仅新增回执。

## 当前工作区重试

再次核对合同摘要，在全新私有目录重跑。受管版三轮、拒绝、取消及两种新进程加载重现上述结果，证据校验退出 0。另以挂载轮原会话 ID 实发 `session/resume`，返回 JSON-RPC `-32601 Method not found`，子进程退出 0。

| 轮次 | 受管 ACP 会话 ID | 直启原包 ACP 会话 ID |
| --- | --- | --- |
| 无挂载 | `27fa6b6b-c98a-4722-8dc9-191b28141150` | `ac544a19-00e7-465f-84bc-f386226d65b4` |
| 挂载 | `6555b83f-121d-4f04-9880-b122278dac14` | `b692f226-afe4-41db-aa58-2adb31268100` |
| 撤除 | `4f60829d-5f39-4086-95de-7de315338a56` | `fadd063c-87ba-4eec-ae4b-199ddd80f1f3` |

原包直接以 `native/trae-cli acp serve` 启动，继续使用独立 HOME 和仅允许回环端口的沙箱。三轮都能选模，挂载轮服务仅握手和列目录。三轮首个提示均报 `-32603` 和 `no value available in memory store`，模型请求与服务执行均为零，进程均退出 0。以同类配置直接运行原包 `--print` 时，令牌存储初始化报 `keyring is not supported on this system`，进程退出 1，模型请求为零。原包 ACP 握手的 `authMethods` 为空。下一次原包复验需要当前测试环境可用的令牌存储和独立获授权的供应商会话。

私有证据位于 `/private/tmp/shiju-trae37-retry.G15GgO`。ACP 复验沿用上节命令，非 ACP 对照使用 `python3 <私有目录>/probe_cli.py official-cli`。两类探针均只访问本地验收服务与回环模型。
