# CodeBuddy 2.179.3 官方工具与 MCP 共存回执

2026-11-19 在 macOS 27.0 arm64、Node.js v24.22.3 复验固定版 `@tencent-ai/codebuddy-code@2.179.3`。从现有受管 v1 安装复制到私有配置根。npm 的 `node_modules/.bin/codebuddy` 指向 `bin/codebuddy`，入口 SHA-256 为 `c8f736a33584a55d9d4c4cf5ac9785f582c5025e384034c73fb4a840901f2907`，三个发行 JS 和包元数据均通过受管合约摘要。原包对照在第二份私有副本复用依赖树，三个 JS 及入口逐字节比对通过，基准为 npm 缓存中通过合约 SHA-512 的供应商归档。原安装及用户配置未参与写入。[受管合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [历史验收](../../mcp-acceptance/codebuddy.md)

## 默认启动与权限

用仓库受管安装脚本在空私有目录冷装。在线请求下载归档过慢，补入按合约 SHA-512 校验的缓存归档后，以隔离 npm 缓存离线重跑。安装脚本退出 0，应用固定补丁并校验发布文件。冷装和已校验的旧安装副本都由下述默认配置接口复核。

- 私有回环 ShiJu 网关使用已构建的槽位 3 二进制，SHA-256 为 `653d967c267fe9ee1cb68e27109b6bccbbff2584b679c5611ada530bf702e2cb`。`GET /v1/global/config/acp/defaults` 返回 HTTP 200，`codebuddy` 选中已校验的受管绝对路径，序列化结果省略空 `args`。未安装时返回通用 `codebuddy --acp`。向默认空参数入口写入 ACP `initialize`，其按普通 CLI 处理 stdin、输出模型文本并退出 0。以同一入口显式传 `--acp`，返回 ACP v1。`reviewed_agent_command` 的 npm 参数分支可解释这一区别，模型驱动在启动前也要求 `args == ["--acp"]`。首个断点为受管默认命令解析。任务 43 负责共享注册修复及冷装复跑。[注册解析](../../../../../crates/libs/acp-agent-registry/src/managed_runtime.rs) · [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/codebuddy.rs)
- 工具轮次显式使用 `--acp`。受管版追加驱动的 `--setting-sources none`，设置私有 CodeBuddy/WorkBuddy 配置目录、空会话技能根及回环模型环境。原版对照用私有配置和原生模型环境，沿用默认设置来源。两组都用一次性回环测试凭据和模型别名 `acp-fixture-primary`。ACP 回读模型别名及 `mode=default`，模型实际请求均发到 `/v1/chat/completions`。没有启用权限旁路，沙箱选项回读为原生默认关闭。官方 `Read` 自动执行，MCP 只对当前会话、完整工具名和空参数做单次同意，其余权限请求默认拒绝。
- 受管发行补丁隔离技能扫描及配置，并适配宿主 Shell 请求形状。共享驱动另固定模型和回环路由。源码的通用路径保留原生权限与工作流审批，宿主的 CodeBuddy 全权限保障为 `partial`，且启动时拒绝提升权限的参数。受管安装限 macOS arm64，其他平台沿用通用原生安装路径。本次实测覆盖本机受管入口和原版字节对照，通用安装、Shell、ShiJu 聊天流及其权限转发未运行。[宿主访问策略](../../../../../crates/libs/acp-client/src/client/codebuddy_access.rs)

## 三轮执行

各轮新建 ACP 进程和会话，使用同一私有工作区及模型配置。官方 `Read` 读取工作区内的 `native-probe.txt`。MCP stdio 服务复用 `scripts/acp_mount_integration.py` 的 RPC 处理，`read_probe` 读取项目外的独立附件。每轮重写两个独立随机标记，模型夹具直到续轮才收到标记。默认发行发现保持启用。受管就绪组先用官方 `WaitForMcpServers` 确认 `coexist_probe` 已连接，再做精确搜索。第三组只在进程的私有环境设置 `CODEBUDDY_DEFER_TOOL_LOADING=0` 和 `CODEBUDDY_WAIT_FOR_MCP_SERVERS_ENABLED=0`，用直接声明作对照，stdio 服务目录始终只有一项。

| 入口 | 轮次 | ACP 会话 ID | 调用 ID 按执行顺序 | MCP 调用 | ACP 终态 / 进程退出 |
| --- | --- | --- | --- | --- | --- |
| 原版默认 | 无挂载 | `01a1f29e-0e5f-7c16-902e-8411b0d5f8d3` | `call_before_0` | 0 | `end_turn` / `-9` |
| 原版默认 | 挂载 | `01a1f29e-27e9-7652-93a7-ea9b2304d3ed` | `call_mounted_0`、`call_mounted_1`、`call_mounted_2` | 1 | `end_turn` / `-9` |
| 原版默认 | 撤除 | `01a1f29e-420f-782f-bfee-fdb4f9316381` | `call_removed_0` | 0 | `end_turn` / `-9` |
| 受管默认，先等就绪 | 无挂载 | `01a1f2a1-8965-7e2e-8707-3d14f4c2f3b0` | `call_before_0` | 0 | `end_turn` / `-9` |
| 受管默认，先等就绪 | 挂载 | `01a1f2a1-a24b-7d9c-9061-b273d14608a8` | `call_mounted_0` 至 `call_mounted_3` | 1 | `end_turn` / `-9` |
| 受管默认，先等就绪 | 撤除 | `01a1f2a1-bc6d-705f-bee5-5d3544b366a5` | `call_removed_0` | 0 | `end_turn` / `-9` |
| 受管直接声明 | 无挂载 | `01a1f29e-0ee0-7071-b8ba-325ba618f704` | `call_before_0` | 0 | `end_turn` / `-9` |
| 受管直接声明 | 挂载 | `01a1f29e-2799-7ce3-b1d8-182b64752e35` | `call_mounted_0`、`call_mounted_1` | 1 | `end_turn` / `-9` |
| 受管直接声明 | 撤除 | `01a1f29e-41a1-7ba8-8466-d0185e6782b1` | `call_removed_0` | 0 | `end_turn` / `-9` |

各轮的第 0 项由 CodeBuddy 原生 `Read` 执行。原版挂载第 1 项为官方 `ToolSearch`，按完整名 `mcp__coexist_probe__read_probe` 返回 schema，第 2 项由原生 `DeferExecuteTool` 将完整名和空参数转发给 stdio 服务。受管就绪组的第 1、2、3 项依次为官方 `WaitForMcpServers`、`ToolSearch`、`DeferExecuteTool`。直接声明组第 1 项以完整 MCP 工具名调用。三次挂载服务均实收 `initialize` ID 0、`tools/list` ID 1、`tools/call` ID 2。文件和附件标记均按原 ID 出现在 ACP 完成事件及模型续轮的 `tool_call_id` 下。无挂载及撤除的模型目录无目标服务，stdio 审计零请求。退出码 `-9` 来自探针在 ACP 终态后关闭 stdin 等待 5 秒，再回收仍存活的子进程。

受管默认组在没有先等就绪的两个新挂载中，目录已经列出完整 MCP 名称，但第一次精确 `ToolSearch` 都退到 `TeamCreate` 的模糊搜索结果。随后仍可用完整服务名执行：成功样本 `01a1f29e-27a3-77a8-a0a9-e48d14c8c37a` 的 `call_mounted_2` 获得真实 MCP 返回并续轮。同一隔离设置下先等就绪，两次新挂载和一次带服务的加载均返回正确 schema。单独移除技能根环境、单独恢复设置来源的诊断样本也返回正确 schema。原生搜索源码会在精确匹配失效时退到模糊匹配，当前证据指向首次就绪与搜索目录的时序差异。保留默认发现和就绪检查，尚无证据确认受管适配补丁裁剪了可执行工具。

## 拒绝、恢复与缺证

- 用同一默认受管权限模式显式选择 `reject_once`。会话 `01a1f29d-47eb-7f85-a0a9-e48d14c8c37a` 的 `call_mounted_2` 触发审批，服务只有握手和列目录，零 `tools/call`，ACP 终态 `cancelled`，无成功 MCP 结果。原版及直接声明对照同样零工具执行。受管就绪组独立会话 `01a1f2a2-0762-715b-a710-825ef6c19e2a` 在模型请求挂起时发 `session/cancel`，收到 `cancelled`，无工具和服务请求。两个样本在终态后按上述方式回收。
- ACP 声明 `loadSession=true`。原版、受管就绪及直接声明组均断开后以原挂载 ACP 会话 ID 加载，并分别重传及清空 `mcpServers`。受管就绪组重传后 `call_load_0` 至 `call_load_3` 均完成，MCP 请求 2 对应 `call_load_3`，结果进入续轮。清空后仅执行 `call_load_removed_0` 的原生读取，目标服务零请求。两次提示均为 `end_turn`。初始化未声明 resume 能力，未发独立 resume 请求。MCP HTTP/SSE 有协议声明，本轮实测 stdio。会话和服务名单、同名工具、供应商模型及其他平台未复验。
- 正向共存三轮通过。共享受管注册空参数缺陷已实测，归任务 43。未修改运行时代码，未运行 Cargo。原始 ACP、模型、服务审计和私有回环夹具保存在本机 `0700` 目录 `shiju-codebuddy-coexist.cMIia6XKmZ`。
