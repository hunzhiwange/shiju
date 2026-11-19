# Autohand 0.9.11 官方工具与 MCP 共存回执

2026-11-19 在 macOS 27.0 arm64 使用固定原版 `0.9.11 (46a28b16)` 直连 ACP v1。实际入口为 `autohand --acp`，入口文件 SHA-256 为 `339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb`，与[前次验收](../../mcp-acceptance/autohand.md)一致。注册表的备用入口是 `npx -y autohand-cli@0.9.11 --acp`。Unix 受管安装用 npm，Windows 经 PowerShell 调用同版本 npm 安装。安装和注册均设置 `AUTOHAND_SKIP_AHTRACES_INSTALL=1`、`AUTOHAND_SKIP_COMPUTER_CONTROL_INSTALL=1`，无安装补丁。本轮复用现有制品。

## 配置与权限

在同一个新建临时项目中用三个独立 ACP 进程和新会话对照。复刻驱动的私有 `AUTOHAND_HOME`、`AUTOHAND_CONFIG`、原生 OpenAI 选模和代理环境，指向回环 Chat Completions 夹具。原生会话回读模型 `acp-fixture-primary` 和默认 `default`（Ask）模式。使用默认 Code Mode 和总结设置，无私有项目权限名单、进程工具名单或直出启动参数。首次试跑发现适配器请求非流式 JSON，修正夹具响应格式后在新临时目录完整重跑。

宿主驱动静态核对确认，它验证原生命令参数、无凭据和查询的显式端口 HTTP 回环根地址，以及选定模型的工具调用和温度能力，随后写入私有 OpenAI 配置，由原生实现选模。仅显式追加审核过的成对直出和关闭自动总结参数时才启用相应覆盖。该能力及驱动源码由 macOS、Linux、Windows 共用。本次实测覆盖本机原生入口，备用 npm 启动、Linux 和 Windows 制品、ShiJu 网关的模型绑定与审批转发均未运行。

## 三轮执行

模型只发出搜索和只读脚本调度，两个随机结果分别来自工作区的 `native-probe.txt` 和项目外的独立 MCP 附件。服务复用 `scripts/acp_mount_integration.py` 的 stdio RPC 逻辑，并另外记录收到的请求方法和 ID。三轮的顶层模型目录均声明原生 `tool_search`、`run_tool_script`、`ask_followup_question`，目标工具通过搜索结果延迟发现。

| 新会话 | ACP 会话 ID | 执行与续轮 | 目标服务审计 | ACP 终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| 无挂载 | `c0b9f2c4-846d-40fb-9c18-d86805084006-1795071325622` | 原生 `tool_search` 为 `before_0`。原生 `run_tool_script` 为 `before_1`，派发官方 `read_file`（`before_1:0`）。均完成，模型按 `before_1` 收到当轮文件结果 | 搜索无目标工具，零请求 | `end_turn` / 0 |
| 挂载 | `a15de2f1-98b0-484b-a707-869acbaf3146-1795071327294` | 原生 `tool_search`（`mounted_0`）发现 `read_file` 和 `mcp__coexist_probe__read_probe`。两个原生 `run_tool_script`（`mounted_1`、`mounted_2`）分别派发官方 `read_file`（`mounted_1:0`）和 MCP `read_probe`（`mounted_2:0`）。五个 ACP 工具事件均以同 ID 配对开始与完成 | `initialize` ID 1、`tools/list` ID 2、`tools/call` ID 3，恰好一次执行 | `end_turn` / 0 |
| 撤除 | `354dc95b-caa7-4064-979d-6fadf0e7072f-1795071329049` | 原生 `tool_search` 为 `removed_0`。原生 `run_tool_script` 为 `removed_1`，派发官方 `read_file`（`removed_1:0`）。均完成，模型按 `removed_1` 收到新的文件结果 | 搜索无目标工具，零请求 | `end_turn` / 0 |

挂载轮的续轮输入用 `tool_call_id=mounted_1`、`tool_call_id=mounted_2` 分别带回官方文件和 MCP 附件的实际标记。两个子调用的 ACP 更新带有对应的 `parentToolCallId`，其原始输出与模型收到的标记一致。两轮对照的搜索均发现官方 `read_file`。三个请求均在新会话使用同一提示和模式。

## 恢复、取消及验证边界

- ACP 声明支持 load、resume、HTTP 和 SSE，本轮服务走 stdio。断开并重启进程后，以挂载轮的 ACP 会话 ID 和同一 cwd 执行 `session/load`，显式重传服务。历史进入模型输入，新一轮再获两类结果，父调用 `load_mounted_1`、`load_mounted_2` 分别对应子调用 `load_mounted_1:0`、`load_mounted_2:0`。MCP 初始化、列目录、调用各一次，MCP 调用 ID 为 3。
- 再次断开并重启，`session/resume` 带同一 ACP 会话 ID 与空 `mcpServers`。历史保留，本轮搜索结果移除目标工具，服务零请求。当前文件结果经 `resume_removed_1:0` 进入 `resume_removed_1` 的模型续轮。两次恢复提示均以 `end_turn` 结束，进程退出 0。
- 在独立无挂载会话阻塞首次模型应答，发送 `session/cancel` 后收到 `cancelled`，无工具或 MCP 请求，进程退出 0。
- 默认 Ask 模式的两种只读调用都未产生 `session/request_permission`。显式宿主拒绝在调用阶段缺少审批事件，未验证；会话名单、服务名单、同名路由以及其他传输也未复验。三轮为原生适配器与真实 stdio 服务的回环模型证据，真实供应商模型和网关聊天流未运行。

当前固定版三轮共存通过。未发现适配器裁剪，无产品代码改动，未运行 Cargo。探针关闭了自有进程和回环监听，删除私有模型配置。原始 ACP、模型和 MCP 审计保存在本机权限为 `0700` 的临时目录 `tmp.5AzvnCp3oD`。
