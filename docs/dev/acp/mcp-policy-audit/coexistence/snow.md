# Snow 0.9.14 原生工具与 MCP 共存回执

2026-11-19，Darwin 27.0.0 arm64、Node.js 24.22.3。受管 v3 的 Chat Completions、Responses、Anthropic 和 Gemini 均通过三轮共存复验。Chat 等待 MCP 审批时的取消阻止了执行，但提示返回 ACP 内部错误，受管入口的取消复验待任务 43 更新制品合约。

## 制品与权限

- 注册 key 为 `snow`，通用入口为 `snow --acp`。实际执行已受管安装的 `node_modules/.bin/snow --acp`，链接目标为 `snow-ai/bundle/cli.mjs`，SHA-256 为 `2a38b3678070cf5d47892ad0c8c43429090b73e5a4000cb3d92edaa5478c4a77`。复核缓存的 npm 归档 SHA-512 与安装合约一致。私有目录重打仓库补丁后与实际入口逐字节一致，`node --check` 通过。握手自报 Snow 0.9.14、ACP v1，声明 `loadSession=true`，HTTP/SSE MCP 均为 false。[安装合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L240-L248) · [注册表](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L493-L501)
- 每个协议使用独立私有 `HOME`、`SNOW_CONFIG_DIR` 和工作区，`SHIJU_SNOW_SKILL_ROOT` 为空。私有 `config.json` 通过 `snowcfg.requestMethod` 选择协议，`advancedModel=probe`，关闭提示优化和重试，使用回环端点及占位认证。直接 ACP 探针沿原权限路径，仅对两项测试只读工具逐次 `approve_once`，另测 `reject_once`；未启用全权或永久批准。两个工具分别读取不同的只读合成文件，回环仅提供确定的模型行为并核对独立随机内容。[模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/snow.rs)
- 原生 `filesystem-read` 由 Snow 执行。模型名 `coexist-lookup` 由单独的 Python stdio MCP 服务执行原名 `lookup`，服务日志独立记录实际 `tools/call`。每个协议的两个空挂载对照在模型侧均有 25 个原生工具定义；挂载侧为同一集合追加一项。两个调用由同一模型响应给出，在同一 ACP 提示内执行。续轮在同一请求中收到两项实际结果并完成提示。

## 四种协议的三轮新会话

表内工具 ID 按原生读取、MCP 顺序排列。每项均发出 ACP 开始和 `completed`，每轮提示终态均为 `end_turn`，四个 Snow 进程均退出 0。每个挂载轮的目标服务实际调用一次，MCP JSON-RPC ID 为 `2`。两个对照均无目标声明、审批或服务调用。

| 协议 | 轮次 | ACP 会话 | ACP 工具调用 ID | 目标服务调用数 |
| --- | --- | --- | --- | --- |
| Chat | 无挂载 | `4cb195b5-5f5f-415c-9329-64342ebab556` | `call_before_read` | 0 |
| Chat | 挂载 | `0866d35b-5505-4127-96f1-6f9cdedc4430` | `call_mounted_read`、`call_mounted_mcp` | 1 |
| Chat | 撤除 | `27996ec9-c7f3-4fe4-a35e-79267a087648` | `call_removed_read` | 0 |
| Responses | 无挂载 | `f6f3597c-5df0-491b-a18f-9cc096c04cdb` | `call_before_read` | 0 |
| Responses | 挂载 | `c71dc715-6479-4270-b769-635e49ea5a1e` | `call_mounted_read`、`call_mounted_mcp` | 1 |
| Responses | 撤除 | `ed832447-7a50-4041-9beb-43180a91c1fc` | `call_removed_read` | 0 |
| Anthropic | 无挂载 | `ba33d645-c395-4ee8-880b-dc683ccfcd0d` | `call_before_read` | 0 |
| Anthropic | 挂载 | `d690ed8d-f50a-45ee-89cc-6f33b7070d74` | `call_mounted_read`、`call_mounted_mcp` | 1 |
| Anthropic | 撤除 | `25f5ee75-ae04-4957-b256-a4f3d7fe0fdf` | `call_removed_read` | 0 |
| Gemini | 无挂载 | `4be709ed-738a-405f-b7fc-e79187c4cf0f` | `call_0` | 0 |
| Gemini | 挂载 | `487f119c-7e56-447b-a09e-b34f64685404` | `call_0`、`call_1` | 1 |
| Gemini | 撤除 | `c5dad95f-71cb-437e-a2bc-2dc91023fa06` | `call_0` | 0 |

Chat、Responses、Anthropic 的续轮分别以 `tool_call_id`、`call_id`、`tool_use_id` 回传和 ACP 相同的调用 ID。Gemini 路径按响应中的位置生成 ACP ID，续轮的 `functionResponse` 按函数名和顺序对应结果，无模型侧 ID 回传证据。同名多次调用及真实上游模型行为均未覆盖。

## 拒绝、取消、加载与入口边界

- Chat 对撤除会话执行 `session/load` 并重新挂载，`call_loaded_read` 和 `call_loaded_mcp` 均完成，目标服务执行一次，续轮匹配两项 ID。再以空列表加载同一会话，仅有 `call_load_removed_read`，目标调用为零。两次都以 `end_turn` 结束。Snow 提供 load，未声明独立 resume。
- Chat 拒绝会话 `5dd6c6c3-db01-4e1c-a857-daf1cbb038c4` 中，宿主对审批请求 `7` 回复 `reject_once`。`call_denied_mcp` 发出 `failed`，同 ID 的续轮收到 `Permission denied by user`，服务零调用，提示 `end_turn`。
- Chat 取消会话 `c8853cc7-1552-4634-8d12-9ed420ce36ec` 中，等待 `call_cancel_mcp` 审批 `8` 时先发 `session/cancel`，再回复迟到的单次批准。两次独立复现均返回内部错误，取证轮记录零服务调用、无工具完成或模型续轮，Snow 自身退出 0。提示返回 `-32603`，详情 `This operation was aborted`，探针因缺少 `cancelled` 退出 1。首个断点在 v3 审批返回后的 `signal.throwIfAborted()`，外层取消分支只识别另两种错误消息。已进入 stdio 执行后的取消未覆盖。下节记录候选修复及受管制品接线证据。
- 同一归档原件借用已安装依赖后复现历史断点。无挂载官方读取成功；挂载会话 `732b2e59-91d3-4809-96ee-0a3bf6758ebc` 的模型目录缺少 `coexist-lookup`，受控发出的调用只得到原生分派错误，目标服务零调用。原版 ACP 仍可用私有配置选模。macOS arm64 受管入口由安装器核对 v3 摘要，通用入口按命令解析，驱动没有 v3 摘要前置，手装原版仍可能复现。macOS Intel、Linux、Windows 的通用 npm 安装分支及挂载行为本次未实测。[分组审计](../6.md#snow) · [历史验收](../../mcp-acceptance/snow.md)
- v3 补丁承担逐轮 stdio 发现、单次审批、同 ID 结果入历史及清理，Chat 通过实际发现生成目录提示。没有通用服务内名单投影；本次只读服务本身公布一项工具。四种协议均保留官方目录，未发现无意裁剪，旧回复格式提示与受审入口随原制品保留。本次仅测回环与直接 ACP，未重测网关、真实模型或运行 Cargo。

## 取消修复候选与制品接线

直接 ACP 复跑当前受管入口。收到 `session/cancel` 后，审批分别回迟到的 `approve_once` 和宿主实际使用的 `outcome: cancelled`，两种提示均返回 `-32603`，目标调用均为零。宿主许可回包已正确表达取消，错误在 Snow v3 的提示终态映射内复现。[宿主权限回包](../../../../../crates/libs/acp-client/src/client/event_client.rs#L160-L176)

私有候选在外层提示异常分支读取 `controller.signal.aborted`，保留原有错误映射、单次审批和 `finally` 清理。两种回包重放均返回 `cancelled`，无工具完成、服务调用或模型续轮；两个 Chat 探针均为七项通过、退出 0。候选分别复跑 Responses、Anthropic、Gemini 三轮，均退出 0。测试进程均退出 0。

候选 SHA-256 为 `4a9c72d33bdce84f0ceedf86d3f5b18d8f65683665ddb176da8c61ecc3443b8a`。使用候选补丁规范和仓库 `apply.py` 从固定原件生成逐字节相同产物，`node --check` 通过。执行安装器实际的 `check_files` 函数，现行合约返回 `Reviewed artifact checksum mismatch: bundle/cli.mjs`，同步候选摘要后通过。注册表将补丁和合约嵌入同一受管运行时。正式落地须由任务 43 同步 Snow 补丁、共享安装合约和摘要，安装后复跑现有红绿探针。本卡保留待回归，仓库中的运行时代码及合约仍为当前受审版本。[安装器](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/install.py#L727-L745) · [制品校验](../../../../../crates/libs/acp-agent-registry/src/managed_runtime.rs#L152-L179)

补充轨迹及私有候选位于 `/private/tmp/shiju-snow-task35.z1m19I/cancel-trial.RxbNIk`。使用该目录的 `probe.py chat <入口>` 重放迟到批准，加环境变量 `SNOW_PROBE_CANCELLED_PERMISSION=1` 重放宿主取消回包。

## 复验入口

探针和合成轨迹存于本机私有目录 `/private/tmp/shiju-snow-task35.z1m19I`。运行 `python3 <私有目录>/probe.py <chat|responses|anthropic|gemini> <受管 node_modules/.bin/snow>`。Responses、Anthropic、Gemini 各三轮退出 0；Chat 六个场景通过，取消场景退出 1。探针 SHA-256 为 `275c9a32611d32a174d993ee99dc2072fc1b34da428f5020fddb3904fcac3d04`。测试进程和监听均已退出。
