# OpenClaw 2026.10.10 官方工具与 MCP 共存回执

2026-11-19。固定版私有网关通过无挂载、挂载、撤除三轮。挂载会话实际执行官方读取和 MCP 读取，结果按调用 ID 进入模型续轮。当前宿主投影缺少 OpenClaw 的逐服务名单接线，实际后端在重放该投影时执行了待拒绝的工具。原生名单对照能阻止这次调用。

## 制品与权限

- Darwin 27.0 arm64、Node 24.22.3。通用注册 key 为 `openclaw`，通用入口为 `npx openclaw acp`，原生 Provider 可选模。实测复用已安装的受管 v1/v2，分别通过 `node_modules/.bin/openclaw` 链接到固定版 `openclaw.mjs`。两版入口 SHA-256 均为 `469e60c9f60b615ff71c985a54238ee532a8f949866d76aebe7d6e4bc7219902`，技能根文件均为 `db073f36c9ac2d94a9a51b5076bbe6e3ae780c67ec820e81ef000bef1bb83459`。挂载使用 v2，其额外 stdio 传输文件摘要为 `5bc81023917314bfba0ab85f6987909845a44ae43cfbde2e60038a71bf100b35`。三项与当前合约匹配，空挂载另验证了 v1。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [入口校验](../../../../../crates/libs/acp-agent-registry/src/managed_runtime.rs)
- 逐轮创建私有 HOME、配置、工作区与回环网关。用 `openclaw gateway run --port <port> --bind loopback` 和 `openclaw acp --url ws://127.0.0.1:<port> --token-file <file> --no-prefix-cwd --session <key>` 启动，设置 `OPENCLAW_STATE_DIR`、`OPENCLAW_CONFIG_PATH`、两个跳过通道和 Canvas 的环境变量以及空 `NODE_OPTIONS`。选择 OpenAI Chat Completions 回环模型 `test-model`，使用私有临时占位认证。补充轮关闭启动更新和远程模型目录刷新。挂载轮将字面量绝对 stdio 命令写入 `mcp.servers`，无服务环境覆盖，禁用 Tool Search，向 ACP 发送空 `mcpServers`。ACP v1 握手支持 load 和 resume，HTTP/SSE MCP 均为 false，挂载探针收到的 ACP 审批回调为零。`adaptive` 是推理模式回读。[宿主启动驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/openclaw.rs)
- 直连探针覆盖后端按原生权限执行的路径。当前宿主要求 `session_model`、显式完全访问、审批未受限、启用工具以及绝对 stdio，拒绝 `cli_local` 挂载和显式环境覆盖。宿主实时拒绝沿用历史真实网关证据，本轮依据当前源码核对，未重新启动 ShiJu 验证门禁。v1 直接给 ACP 非空声明的对照在 `session/new` 得到 `-32603`，明确要求配置到 OpenClaw 网关或 agent。零模型调用，两进程均退出 0。[请求分支](../../../../../crates/libs/llm-runtime/src/acp/request.rs) · [完全访问拒绝及选模历史实测](../../mcp-acceptance/openclaw.md#适配后的指定模型验收)

## 三轮执行

每轮均为独立新会话。官方 `read` 由 OpenClaw 读取本轮工作区文件。MCP `coexist__read_probe` 由真实 stdio 夹具读取工作区外的另一份随机附件，服务独立记录 `tools/call`。ACP 开始、完成与模型续轮的工具 ID 一致，模型收到对应文件的实际值。两个对照轮的模型工具表均无目标服务。

| 轮次 | ACP 会话 | 官方 `read` ID | MCP `coexist__read_probe` ID | 服务调用 | ACP 终态 / 桥与网关退出 |
| --- | --- | --- | --- | --- | --- |
| 无挂载 v1 | `f7305a29-9c99-431a-9a0f-a05f13d70ee9` | `callbeforeread` | — | 0 | `end_turn` / 0、0 |
| 挂载 v2 | `cfe325c0-9672-4edc-800f-e755f5bc7f78` | `callmountedread` | `callmountedcoexistreadprobe` | RPC `2 read_probe` | `end_turn` / 0、0 |
| 撤除 v2 | `8fe3eb48-cc51-4cb2-9213-18ad4fd4fd88` | `callremovedread` | — | 0 | `end_turn` / 0、0 |

补充轮在 v2 执行 `callmountrestoreread` 与 `callmountrestorecoexistreadprobe`，服务 RPC `2 read_probe`，两种调用均完成并续轮。网关退出后保留私有会话存储，在无 MCP 的新网关对原会话 `4495d688-12a6-4812-aa81-93ebd97d5604` 执行 `session/load` 及 `session/resume`，两者均成功。模型收到历史结果和新的 `callrestoreread`，当前工具目录无目标服务，撤除后零新 MCP 调用。两个 prompt 均为 `end_turn`，各进程退出 0。另在回环已收到模型请求时发送 `session/cancel`，返回 `cancelled`，零工具调用，桥和网关退出 0。所有已启动的 stdio 夹具在网关正常退出后核对 PID 已消失。强制终止及技能根隔离沿用历史证据。[生命周期历史实测](../../mcp-acceptance/openclaw.md#私有网关适配与生命周期复验)

## 显式名单

当前宿主的共享策略投影对 OpenClaw 没有逐服务名单分支，生成的私有网关项只含传输、命令和参数。会话 `allowedTools` 则写入 `_meta.claudeCode.options`。重放当前投影时，即便显式会话名单只给 `read` 与 `coexist__read_probe`，模型仍收到 `coexist__blocked_probe`，真实服务也执行了该调用。[逐服务投影](../../../../../crates/libs/session-engine/src/acp_defaults.rs) · [会话名单元数据](../../../../../crates/libs/acp-client/src/client/session_meta.rs)

| v2 私有网关输入 | 官方 `read` ID | MCP 获准 ID / RPC | 待拒绝的 `coexist__blocked_probe` ID / 结果 |
| --- | --- | --- | --- |
| 当前宿主投影，无原生名单 | `callhostshaperead` | `callhostshapecoexistreadprobe` / 2 | `callhostshapecoexistblockedprobe` / 可见、完成并续轮，服务 RPC 3 |
| 同一投影，另带会话 `allowedTools` | `callhostmetaread` | `callhostmetacoexistreadprobe` / 2 | `callhostmetacoexistblockedprobe` / 可见、完成并续轮，服务 RPC 3 |
| 原生 `toolFilter.include=[read_probe, blocked_probe]`，`exclude=[blocked_probe]` | `callnativefilterread` | `callnativefiltercoexistreadprobe` / 2 | `callnativefiltercoexistblockedprobe` / 隐藏、强制提名返回工具不存在，服务零调用 |
| 原生 `toolFilter.include=[read_probe]`，另带会话 `allowedTools` | `callnativeallowread` | `callnativeallowcoexistreadprobe` / 2 | `callnativeallowcoexistblockedprobe` / 隐藏、强制提名返回工具不存在，服务零调用 |

各轮官方读取与获准 MCP 结果均完成并进入续轮，拒绝轮的模型也收到相同调用 ID 的失败结果。四轮均为 `end_turn`，桥与网关退出 0。原生名单可执行，当前共享链路未将名单接至此入口。将策略接线与实际 ShiJu 网关复验留给[任务 43](../../../../../.plans/2026-11-19-acp-工具共存/43-集成与证据收口.md)。接线需核对宿主的名字匹配与原生 glob 语义，确保无法安全投影的请求在挂载前明确拒绝。复跑用显式会话及逐服务配置确认允许项成功、拒绝优先且目标服务零执行。三轮未发现 OpenClaw 补丁裁剪官方工具，未改产品实现。

## 证据与复验

合成线报文、模型续轮和服务审计在本机 0700 私有目录 `shiju-openclaw-coexist28.w6j1G5Sn` 的 `retry` 与 `extra`。两组分别将其中的 `probe.py`、`fixture.py` 复制到新的私有运行目录，保留脚本中固定版制品位置，从仓库根目录执行 `python3 <运行目录>/probe.py "$PWD"`。主探针 SHA-256 为 `e0a18f7cb9c2188953bab4200cc283a1f49d48bcb097bc1671122558cef6d374`，补充探针为 `87fbdec2e56172927d4265b6c1c96c4e8a0aa50e09bd85ab7bf1dd10275947b9`。未冷装制品、运行真实模型、运行 Cargo 或验证其他平台。
