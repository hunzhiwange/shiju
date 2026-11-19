# Grok 官方工具与 MCP 共存复验

2026-11-19，Darwin arm64 的 Grok 1.0.77 投影入口通过三轮共存复验。原生回退有一项名单失败，保留任务卡供集成复验。[共存验收](../../ACP验收.md) · [分组审计](../5.md#grok) · [历史网关验收](../../mcp-acceptance/grok.md)

## 入口和环境

- 注册 key 为 `grok`，本地 `grok agent stdio`，后备 `npx -y @xai-official/grok@1.0.77 agent stdio`。无独立受管安装摘要合约。实际启动已缓存的原生 `grok-1.0.77 agent stdio`，版本为 `1.0.77 (50ca579fa996)`，SHA-256 为 `197c66a63e5ac46e07450b0ca21d141ce665050a412180f185dadfae1c7a5b2a`。另审 `1.0.24 (68e414c661e3)`，SHA-256 为 `4291021c1570a7c8610277a3d65490a5e54b50311e222c6b4614264f02a215b3`。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L382)
- 按会话模型驱动以 `/usr/bin/python3 -I projection.py <私有模板> <回环端口> <原生入口> agent stdio` 运行当前仓库投影。临时 `HOME` 和 `GROK_HOME` 隔离，私有 TOML 权限为 0600，`GROK_DEFAULT_MODEL=shiju-session`，禁用共享 leader、自动更新和子代理。模型回环核验虚拟环境令牌及 `fixture-model`，stdio 子进程核验未继承令牌。保留原生权限，未传提权参数。新建回包无 `modes.currentModeId`，以实际 `session/request_permission` 及单次审批选项取证。[驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/grok.rs)
- 三轮顺序使用相同工作区和不同私有会话，逐轮换工作区文件及项目外附件的随机内容。模型夹具只发确定调用，原生工具和只读 stdio 服务分别读取实际文件。挂载服务允许 `mcp_read` 并拒绝 `mcp_denied`。夹具与日志在本机 0700 私有目录 `/tmp/shiju-grok17.f10MIl`。

## 三轮直接 ACP

| 轮次 | 新会话前缀 | 工具、执行方和调用 ID | MCP 服务调用数 | 提示终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| 无挂载 | `01a1f2ae-3313` | `read_file`，Grok，`call_fixture_1`，`completed` | 0 | `end_turn` / 0 |
| 挂载 | `01a1f2ae-3c0a` | `read_file`，Grok，`call_fixture_1`，`completed`；`mcp_read`，stdio 服务 `local`，`call_fixture_2`，`completed` | 1 | `end_turn` / 0 |
| 撤除 | `01a1f2ae-4932` | `read_file`，Grok，`call_fixture_1`，`completed` | 0 | `end_turn` / 0 |

每轮 ID 由夹具独立生成。ACP 和模型续轮的 `tool_call_id` 相同，挂载轮最终模型请求同时收到两个本轮标记。挂载的模型目录直接公布 `mcp_read`，投影保留 `read_file` 等原生工具，替换 `search_tool` 和 `use_tool` 两个发现与分派入口。调用按同 ID 转为 Grok `use_tool`，经 `allow_once` 后，服务审计记录 MCP RPC `2 mcp_read`。两个对照轮仍成功读取工作区文件，模型目录无目标工具。

## 原生对照、权限和恢复

- 1.0.77 直启原生入口模拟无投影的回退命令，使用同形私有 TOML。未设置名单时，`search_tool`（`call_fixture_1`）由 Grok 完成，返回 `local__mcp_read`，续轮通过 `use_tool`（`call_fixture_2`）经 `allow_once` 执行。服务 RPC `2` 返回真实附件，模型收到结果后 `end_turn`，退出 0。投影入口的无名单组直接公布两个 MCP 工具，`read_file` 和 `mcp_read` 分别用相同两个夹具 ID 完成并续轮，退出 0。
- 仅拒绝 `mcp_denied` 时，投影入口保留 `mcp_read` 并移除拒绝项，目标调用为零，`end_turn`，退出 0。相同名单的原生直启仍通过 `use_tool`（`call_fixture_1`）向 ACP 请求批准。夹具给单次批准后，服务 RPC `2` 实际执行了应拒绝的工具，模型收到结果并 `end_turn`，退出 0。首个断点是 Rust 选择原生回退时缺少对 `shiju.grok.toolPolicy` 的门禁。当前宿主投影会附带名单，但启动请求未拦截该组合。CLI 本地模型也走原生入口，其网关复验尚缺。[宿主名单投影](../../../../../crates/libs/session-engine/src/acp_defaults.rs#L151) · [启动分流](../../../../../crates/libs/llm-runtime/src/acp/request.rs#L269)
- 投影入口单次拒绝轮的 MCP `call_fixture_1` 为 `failed`，提示以 `PermissionRejected` / `cancelled` 结束。独立取消轮以 `session/cancel` 得到 `cancelled`。两轮服务调用均为零，退出 0。`session/load` 和 `session/resume` 各在成功挂载后传空表，后续提示均 `end_turn`，只保留挂载轮原有的一次服务审计，进程均退出 0。历史工具事件可重放。
- 混合 HTTP 与带名单的 stdio 在 `session/new` 阶段中断，未到达模型和工具服务。客户端观察到 EOF，子进程退出 0，未收到结构化 ACP 错误。1.0.24 的投影挂载轮同样以 `read_file`（`call_fixture_1`）和 MCP（`call_fixture_2`）完成，服务 RPC `2` 执行一次，结果进入续轮，`end_turn` / 0。

## 证据边界

- 当前复验走直接 ACP，回退命令由夹具模拟。未在物理缺失系统 Python 的机器重新运行 Rust 宿主，未复跑网关租户投影或真实模型。历史真实模型网关验收独立列证。源码显示 Linux 走 Unix 分支，模型驱动验证参数、回环端点及已选工具型模型，Windows 会话模型在 ACL 门禁处拒绝。平台实机和纯 HTTP/SSE 未复验。
- 定向 `PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -m unittest discover -s patches/acp-model-source/grok -p projection_test.py` 运行 11 项，退出 0。未运行 Cargo 和桌面编译。当前投影中同会话官方只读工具实际执行，未发现任务卡要求修复的意外裁剪。回退名单失败需在共享启动接线处拒绝无法执行的范围，并复跑原生回退对照。
