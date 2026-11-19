# Hermes Agent 0.19.0 官方工具与 MCP 共存回执

2026-11-19。受管 v2 和带 MCP extra 的同版原包均通过独立新会话三轮的模型侧共存判据。两种入口的同进程 load 和 resume 均保留已撤除服务的工具声明。官方读取实际回传，ACP 实时通知缺少对应的完成事件。

## 制品、入口与权限

- Darwin 27.0.0 arm64，Python 3.12.16。内置注册 key 为 `hermes`，通用原生入口为 `hermes acp`，独立原包可读本地配置。受管探针从本机已有私有 v2 缓存复制环境，10 项 SHA-256 全部符合受管合约。原包对照在另一份副本离线重装固定 wheel、保留同一组依赖，核对 wheel 中 793 个 Python 文件并移走补丁新增模块。两组包元数据均为 Hermes 0.19.0 / MCP SDK 1.26.0。wheel SHA-256 为 `bd0bac012aee38a60894781f4597dc29ee7bedb3448540249921f10d3bef327f`，两组入口脚本 SHA-256 均为 `e66a196bceb0124acc9979a4187c8af49dbc48cb8c1cc67b482dfa32b9b33beb`。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 双方均从各自的 `bin/hermes acp` 启动，v2 追加 `--session-model-binding`。每轮使用独立工作区、HOME、HERMES_HOME、空 `.env` 和一次性回环占位认证，v2 另使用专属 `VW_ACP_LAUNCH_REFERENCE` 绑定历史。模型回读 `custom:fixture-coexist`，原生权限模式 `default`，无全访问或免审批参数，六轮零权限请求。只发送官方读取和合成只读 stdio 调用，显式拒绝未触发。初始化声明 ACP v1、load、resume、fork、list，未声明 HTTP 或 SSE MCP。本轮走直接 ACP，未运行真实模型、ShiJu 网关、非 Darwin arm64、fork 或跨进程恢复，也未冷装受管包或运行 Cargo。[选模驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/hermes.rs) · [旧版冷装与真实模型](../../mcp-acceptance/hermes.md)

## 新会话及同进程加载

每种入口独立启动三个 ACP 进程。官方 `read_file` 由 Hermes 从项目内的只读文件取得随机值。挂载轮再调用 `mcp__coexist_probe__read_probe`，由仓库验收脚本的 stdio 服务读取项目外另一份随机附件。每次模型发起的 ID 均对应到续轮中的实际随机值。服务审计独立记录 RPC `initialize=0`、`tools/list=1`、`tools/call=2`，仅挂载轮各出现一次。三轮的模型可见工具数均为 15 / 16 / 15。

| 入口与轮次 | ACP 会话 | 官方读取模型 ID / ACP 开始 ID | MCP 模型 ID / ACP ID | MCP 调用数 | ACP 终态 / 退出 |
| --- | --- | --- | --- | --- | --- | --- |
| v2 无挂载 | `6aad40ad-1931-424c-b7dd-4f4642f856ee` | `native-before` / `tc-12962c747df5` | — | 0 | `end_turn` / 0 |
| v2 挂载 | `5d39622f-9f85-4c48-84e1-3a93a797939c` | `native-mounted` / `tc-801590331cba` | `mcp-mounted` / `tc-748e17342a99` | 1 | `end_turn` / 0 |
| v2 撤除后新会话 | `933ea4f4-5a0d-49f7-836d-ce41f84bc9d8` | `native-removed` / `tc-ce3e4906a9b1` | — | 0 | `end_turn` / 0 |
| 原版无挂载 | `40d0ff10-da7f-482f-9057-186e48c66831` | `native-before` / `tc-713fe2a18111` | — | 0 | `end_turn` / 0 |
| 原版挂载 | `69df0d1d-1615-4ebc-891e-2ed73d38803e` | `native-mounted` / `tc-528568ecea41` | `mcp-mounted` / `tc-96a2c6f313ae` | 1 | `end_turn` / 0 |
| 原版撤除后新会话 | `ed992040-f267-49eb-a712-2e640a865277` | `native-removed` / `tc-cedc60893272` | — | 0 | `end_turn` / 0 |

两组挂载轮的 MCP 实时开始与 `completed` 共用表中 ACP ID。官方读取有实时开始事件及模型侧成功结果，未见同 ID 的实时 `completed`。历史重放另用模型 ID 发出已完成的读取和 MCP 事件。两组无挂载进程均另测推理中的取消，回环已收到请求后发送 `session/cancel`，返回 `cancelled`，本轮无工具调用，进程退出 0。

挂载进程内依次对原会话发送 `session/load` 和 `session/resume`，两次均给空 `mcpServers`。两组请求均成功，权限模式回读 `default`，随后的官方读取各读取独立文件并带匹配模型 ID 续轮，终态均为 `end_turn`，进程退出 0。

| 入口 | load 后官方读取模型 ID / ACP 开始 ID | resume 后官方读取模型 ID / ACP 开始 ID | load / resume 后的目录与服务审计 |
| --- | --- | --- | --- |
| v2 | `native-load` / `tc-c202f9597875` | `native-resume` / `tc-181fa0793393` | 16 / 16，旧 MCP 声明仍在，无新增服务调用 |
| 原版 | `native-load` / `tc-5bfd25300c90` | `native-resume` / `tc-1a4e075c8c1e` | 16 / 16，旧 MCP 声明仍在，无新增服务调用 |

首个撤权失败点是空列表 load 后的首次模型请求仍声明 `mcp__coexist_probe__read_probe`。夹具此时只选官方读取，零新增调用仅证明夹具未选旧工具，旧声明的执行拒绝缺证。两组 `acp_adapter/server.py` 字节相同，SHA-256 为 `e780061d47fa6c874f89676afc75ab386b50f79bb269fdef38095245e8f32145`。原生 load/resume 复用会话状态，`_register_session_mcp_servers` 收到空列表直接返回，跳过工具刷新。受审 v2 的依赖、私有状态、选模及 Skill 补丁予以保留。本轮未确认受管补丁裁剪了官方工具，未改运行时代码。合约接线及宿主层撤权复验纳入[任务 43](../../../../../.plans/2026-11-19-acp-工具共存/43-集成与证据收口.md)。

探针及本地合成轨迹保存在私有目录 `shiju-hermes-task19.8DN2xxen`。用仓库根目录参数运行 `python3 <私有目录>/probe.py "$PWD" run`，先给工作组准备空轮次目录及上述两份固定运行环境。脚本复用仓库 `scripts/acp_mount_integration.py` 的只读 MCP RPC，OpenAI Chat Completions 夹具提供确定调用并核对续轮。探针 SHA-256 为 `c86bc0312b396da19adecf91aef978feaff61d5fa83a0d3a45e6564f0c0e91c7`。
