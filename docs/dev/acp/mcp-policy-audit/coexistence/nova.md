# Nova 原生工具与 MCP 共存复验

2026-11-19，Darwin arm64、Node 24.22.3。受管 Nova 1.1.65 v2 通过直连 ACP 的无挂载、挂载、撤除三轮。挂载轮同时完成 Nova 官方文件读取和受管 stdio 服务的只读调用，两项结果均进入回环模型续轮。撤除后的新会话只有官方读取，目标目录消失，目标服务零调用。

## 入口与隔离

- 注册 key 为 `nova`，预设为 `acp-nova`，固定原版命令是 `npx -y @compass-ai/nova@1.1.65 acp`。检查已安装 v2 的包名、固定版本、六项契约 SHA-256 及入口链接。实测启动命令为 `<私有运行时>/nova-1.1.65-model-source-v2/node_modules/.bin/nova acp`，解析到契约中的 `scripts/bin-wrapper.mjs`，受管 JS bundle SHA-256 为 `4138360feaa219ec8d52fae0ee34cc4dddfc6730515b0ed405a144c84ba82e45`。[注册与契约](../3.md#nova)
- 探针复用仓库 Nova Anthropic 协议夹具，使用私有 HOME、0700 Nova 状态、既有授权的本机同意记录和 0600 模型表。回环地址为 `127.0.0.1` 动态端口，受管会话使用 `SHIJU_NOVA_MODEL_CONFIG`、`SHIJU_NOVA_STATE_DIRECTORY` 和合成鉴权值，模型请求实收 `claude-haiku-5-5`，对应受管选模形式 `anthropic/<alias>`。原版用官方 Anthropic 环境路径，同样连接回环，实收 `claude-sonnet-4-6`。原版是通用 ACP 注册入口，发行包含 macOS arm64/x64、Linux arm64/x64 和 Windows x64。当前受管 MCP 执行要求 Darwin、受审会话模型和 owner approval，其他平台未实测。[入口与平台](../../../evidence/acp-onboarding-nova.md) · [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/nova.rs)
- 握手实收 ACP v1、`ownerApprovalRequired=true`。请求携带 `compass.owner-approval.v1`。受管入口在独占连接发布 `compass.host-tools.v1` 目录，服务声明使用绝对 stdio 命令且 env 为空；真实只读 MCP 子进程由仓库沙箱启动，宿主探针只处理 Nova 的 `permission/request`。本次以本地合成标记文件区分官方与 MCP 输出。

## 三轮结果

| 轮次 | ACP 会话 | 工具、执行方与 ACP 开始／完成 ID | 模型终态与进程退出 |
| --- | --- | --- | --- |
| 无挂载 | `f0785f31-3402-46e1-908f-91c9f558384a` | Nova `read_file`，`50389fc7-49de-4c63-8aad-c6476c0e4d46`，续轮 `unmounted_native`；MCP 目录为空、零调用。 | 2 次模型请求，`end_turn / completed`，清理时 SIGTERM（−15）。 |
| 挂载 | `670ab4f4-9247-4f25-b38e-d21767bc9d4b` | Nova `read_file`，`92c57bd6-5bac-4126-be74-df5705a99068`，续轮 `mounted_native`；沙箱 `coexist_probe.read_file`，`4c6f7535-3f7f-4a65-9076-060a264fb964`，续轮 `mounted_mcp`。 | 3 次模型请求，`end_turn / completed`，清理时 SIGTERM（−15）。 |
| 撤除 | `0fed3fe3-bfd3-470a-b8ec-094e01c7b022` | Nova `read_file`，`22d72e15-d0f8-4cf0-9e01-7f5afe0ead68`，续轮 `removed_native`；新连接 MCP 目录为空、目标服务零调用。 | 2 次模型请求，`end_turn / completed`，清理时 SIGTERM（−15）。 |

三轮表内的工具状态均为 `completed`。挂载轮的模型工具名为 `mcp_shiju_coexist_probe__read_file`。审批 JSON-RPC ID 为 `server-1`，审批结构的工具调用 ID 和模型 ID 同为 `mounted_mcp`，ACP 工具开始、完成共用表中的 UUID。MCP 服务 RPC ID `b06a68764917496e9daf80ac51a5ec9d` 由服务随实际读取的 `SERVICE_MARKER:SELECTED` 返回，两条 ACP 完成事件分别带回服务结果和原生读取结果 `NATIVE_MARKER`，模型第三次请求包含两项结果。官方读取沿用原生权限，本样本未触发审批。成功提示结束后关闭 stdin，受管进程未在四秒内退出，由探针结束进程组。

## 边界与对照

- **实际落点**。两个显式服务暴露相同的原始 `read_file`，会话允许名单只纳入 `coexist_probe`。该服务自身 allow 包含 `read_file` 和 `deny_me`，deny 排除 `deny_me`，未列入服务 allow 的 `other_only` 也未进入模型目录。去掉唯一可用权限得到 `-32602`。并列提交两个名为 `coexist_probe` 的声明得到 `-32602`，无提示轮。另在私有工作区和私有 Nova 状态中写入同名本地配置，ACP 挂载仍实际命中显式 `SELECTED`，MCP ID `f548c806fb624658aeec647a042a4d0a` 对应 ACP ID `76203f64-3884-4200-a869-224461176471` 和模型 `same-config_mcp`，原生读取 ID 为 `9670bb9d-0fdb-4ad1-88fa-b5ff497fc3f4`。本地定义的独立服务审计为空，模型收到显式服务标记，提示 `end_turn`，清理 SIGTERM（−15）。
- **审批与取消**。单次拒绝的 ACP ID `e15ea1f8-9284-4e56-9033-8042492e52f5` 以 `failed` 完成，续轮 `reject_mcp` 收到错误。批准等待中取消，ACP ID `f234e9f0-1589-4805-b997-275297e71fcb` 以 `failed` 完成，Nova 的 `turnOutcome.status` 为 `cancelled`，顶层 stop reason 为 `end_turn`。拒绝和取消均无服务结果，取消后没有模型续轮。两次探针清理 SIGTERM（−15）。
- **隔离与恢复**。同连接第二次建会话明确拒绝。新连接以带声明的 `session/load` 恢复历史，`load_mcp` 再次逐次获准，ACP ID `0268796f-631f-45c8-9633-771f6970228f`、服务 RPC ID `d63a85546fba4302984624a12488f355`，结果进入续轮。以空声明分别 load 和 resume 同一历史会话，只执行原生 `read_file`，新调用 ID 分别为 `1b65fd47-d119-464e-97b5-520365ed9639` 和 `a47e6b0e-4c2c-42b8-88e8-587dd0bc98f6`。空声明恢复仍有历史记录，当前目录无目标工具，新调用数为零。三个提示均 `end_turn`，清理 SIGTERM（−15）。携带挂载的独立 `session/resume` 返回 `-32602`，没有提示轮，进程退出 0。
- **原版直接门禁**。从 npm 本地缓存提取原版 1.1.65，归档完整性与契约相符，原版 JS SHA-256 为 `ee9d26976cda1beda9b86a0f05aea0e04a77665ae89d8cfbe5395a11ca6e9935`。直接运行 `node <原版>/dist/cli.js acp`，owner approval 下原生 `session/new` 的 stdio、HTTP、SSE 均返回 `-32602`，拒绝探针退出 0。独立空挂载对照中，原生 `read_file` 的 ACP ID `7ea42203-4893-4950-931d-6e44a00ba3c3` 完成并以 `original-control_native` 进入模型续轮，提示 `end_turn`。该原版进程在 stdin 关闭和 SIGTERM 后仍存活，清理时用 SIGKILL（−9）退出。

首轮裸模型注入缺少宿主驱动同时设置的 Anthropic 环境，补齐合成值后建会话成功。Python stdio 夹具在原沙箱内未完成目录握手，改用私有原生只读夹具后通过。未发现原生目录被适配器意外裁剪，本次只提交复验回执。当前运行未经过 ShiJu 网关 SSE、供应商模型或其他平台，未运行 Cargo；历史真实模型和网关三轮另见 [Nova 验收记录](../../mcp-acceptance/nova.md#新版提示正式网关三轮通过)。
