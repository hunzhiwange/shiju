# OMP 18.22.0 原生工具与 MCP 共存回执

2026-11-19，Darwin 27.0.0 arm64。三次独立 ACP 新会话通过。挂载轮在同一提示内完成官方文件读取和三项真实 RustCodeGraph 调用，模型按调用 ID 收到结果并续轮。撤除后的新会话仅声明官方工具。

## 制品、命令和权限

- 注册 key 为 `omp`，通用发现入口为 `omp acp`。本次复用已下载的 Darwin arm64 独立发行物，SHA-256 为 `d185113d63eba23925e5d70ab661a424b827c06c98f8e766c3bf4ad62ddaff9a`，与受管合约一致。ACP 自报 18.22.0、协议 v1，声明 HTTP/SSE MCP 和 load/resume。通用本机入口的选模已有原生证据，本次检查受审私有模型入口。[分组审计](../4.md#omp) · [原生选模证据](../../ACP验收.md#omp-官方发行与原生-acp-重试2026-11-18)
- 空挂载直接执行 `/usr/bin/sandbox-exec -f <策略> <omp> acp`。挂载改走 `/usr/bin/python3 -I <omp-launcher.py> <策略> <omp> acp`，原样使用仓库启动器、HTTP 桥和只读 stdio 组件。两条路径均追加 `--tools=read,write,edit --no-extensions --no-skills --no-lsp --offline`。原生进程只可写私有 HOME，出站限本轮回环端口。只有挂载启动器把模型和 MCP 路由放到同一端口，并以 `tools.xdev=false` 直列工具。MCP 子进程继续受只读工作区、禁止网络及派生进程的沙箱约束。历史直接把 stdio 交给原生进程时，首个断点为 `posix_spawn` 返回 `EPERM`。[启动分流](../../../../../crates/libs/acp-client/src/client/process.rs) · [启动器](../../../../../patches/acp-model-source/omp/launcher.py) · [历史断点](../../mcp-acceptance/omp.md#首个失败边界)
- 每个进程使用独立私有 HOME、`PI_CODING_AGENT_DIR`、回环服务及一次性占位认证。原生模式为 `default`。私有 `models.yml` 配置 `shiju-session/coexist`，均以 `session/set_config_option` 选择该项，回环在模型请求中收到 `coexist`。夹具只负责提供确定的模型调用和宿主响应，实际执行由受审 OMP 和 RustCodeGraph 完成。图谱服务为 RustCodeGraph 1.2.10，从私有 RueJS 索引副本读取，按宿主元数据只发布 query、callers、node 三种工具。[模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/omp.rs)

## 三轮新会话

| 轮次 | ACP 会话 | 官方 `read` 模型 / ACP ID | 成功目标 MCP 调用 | ACP 终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| 无挂载 | `01a1f2c2-5d44-721e-97c2-b84e2b59cff5` | `before_read` | 0 | `end_turn` / 0 |
| 挂载 | `01a1f2c2-68c1-75fd-a810-52510ec84cdd` | `mounted_read` | 3 | `end_turn` / 0 |
| 撤除后新会话 | `01a1f2c2-77d6-71d2-bc76-95ef07324adb` | `removed_read` | 0 | `end_turn` / 0 |

三次官方 `read` 都由 OMP 直接读取工作区内同一个只读合成文件，ACP 同 ID 发出 `pending` 和 `completed`，续轮携带同一 ID 和文件随机值。两个空挂载对照均走直接沙箱入口，没有目标服务配置、MCP 声明、审批或目标调用。

下列 MCP 工具的模型函数名前缀为 `mcp__coexist_probe_`。OMP 报告的工具 ID 与模型 ID 相同，宿主批准 ID 是独立的 ACP 请求 ID。每项均先获得 `allow_once`，再由 HTTP 桥转交图谱 stdio，OMP 同 ID 报告 `completed` 并把真实结果送回模型。

| MCP 工具 | 模型 / ACP 工具 ID | 宿主批准 ID | 续轮收到的结果 |
| --- | --- | --- | --- |
| `rustcodegraph_query` | `mounted_graph` | `shiju-omp-mcp-BagjB2ufKmtduFGidF2GOEFytndzdvvb` | `createRueSiteApp`，`app/app.tsx:32` |
| `rustcodegraph_callers` | `mounted_callers` | `shiju-omp-mcp-fNSA9mIlbo41JSO5vQSejji66tJ6LGtH` | `mountRueSiteApp`，`app/app.tsx:39` |
| `rustcodegraph_node` | `mounted_node` | `shiju-omp-mcp-WdtpG879DUcmrlzDrkyfrG0ldYyWXmgz` | `useApp.ts:4` 调用 `_$createApp(factory)` |

## 拒绝、取消和恢复

- 同一挂载提示内，OMP 的官方 `write`（模型及 ACP 工具 ID `mounted_write`）对工作区外合成文件发出 `fs/write_text_file`，回调 ID 为 `0`。探针宿主返回 `-32602` 后工具为 `failed`，模型收到匹配 ID 的拒绝结果，磁盘文件原值保持一致。这里复验了回调链及拒绝传播，ShiJu 实际宿主的文件授权仍以其生产路径为准。
- 同一提示内另一次查询 `mounted_denied` 收到宿主 `reject_once`，审批 ID 为 `shiju-omp-mcp-6GlRSsCHaokWwr48Il9KwDzv0sfFEp0f`，工具为 `failed`，模型收到拒绝，桥在转发给 stdio 前结束此调用。该提示仍以 `end_turn` 收尾。
- 在挂载进程的第二个提示，查询 `mounted_cancel` 等待审批时先发 `session/cancel`，再回复旧审批 `shiju-omp-mcp-kqdTU-8XjwzZeA3q9xcb6EhLQFWcZxXU` 为 `allow_once`。ACP 工具为 `failed`，无成功结果，提示终态为 `cancelled`，进程退出 0。此次覆盖批准前的撤销，已进入 stdio 执行的取消未另测。
- 在补充进程 `01a1f2c2-7db0-7705-81c6-bb5049712b30` 首次提示成功执行 `resumed_read` 和 `resumed_graph`。同进程依次以空 `mcpServers` 调用 `session/load` 和 `session/resume`，随后两次提示均只有 `read/write/edit` 声明、零新目标审批和调用，均为 `end_turn`，进程退出 0。load 的通知重放旧提示，模型历史仍含旧结果 ID。

本次未发现现行 OMP 转接裁剪官方名单，回执为唯一提交内容。运行 `PYTHONDONTWRITEBYTECODE=1 python3 patches/acp-model-source/omp/mcp_bridge_tests.py`，4 项通过。历史正式网关的 80 次目标成功调用作为既有证据保留。本次走直接 ACP 与回环模型，未启动新网关、调用真实模型、运行 Cargo、测试其他平台或验证通用入口的挂载行为。[历史正式网关验收](../../mcp-acceptance/omp.md#正式网关验收通过)

探针与合成轨迹存在本机私有目录 `shiju-omp-task27.y9aHtU`。使用独立运行目录及私有 RueJS 索引副本，执行 `python3 <私有目录>/probe.py "$PWD" before mounted removed resumed`。探针 SHA-256 为 `4a26af47b537e6af9496269d93b5f047b010b4c590d7a292ad42067f96167f1c`。测试进程及监听均已关闭。
