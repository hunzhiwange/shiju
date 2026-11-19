# Stakpak 原生工具与 MCP 共存复验

2026-11-19，Darwin arm64 上用 Stakpak 0.3.88 受管 v3、真实 RustCodeGraph 1.2.10 和回环模型完成无挂载、挂载、撤除三轮。挂载时官方查看与图谱查询均成功，原生创建与替换保留原审批。未复现适配器意外裁剪，本次提交为验收回执。

## 受管制品与运行条件

- 注册 key 为 `stakpak`，通用 ACP 入口为 `stakpak acp`。从本机缓存复制固定版到独立私有目录，与当前安装合约逐项比对。固定归档 SHA-256 为 `9c18808f7a3352a5ca2cbb9f123f2b2fafda03a5abef446c20065bb6ec629d1f`。安装文件与仓库补丁来源一致，原生握手自报 0.3.88。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [安装合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 五项 SHA-256 校验通过。

  | 文件 | SHA-256 |
  | --- | --- |
  | `stakpak-native` | `70c559eff250a5be0dcdb19f31f13dca86847ee57648dba78e531506bb482ebc` |
  | `stakpak` | `e23af645b513eae95f62d964ea067aeca76d88bc421a33656eefae7a7f2609c6` |
  | `mcp.py` | `2ec797054b0c9dfb8fb41db19ed15de98363f613e745be26afa29677d22ef580` |
  | `mcp_bridge.py` | `8aea18ecdbc23070a2fc6a9eb5245f095219ac76e9390f4ec2b32e1dd95db136` |
  | `shiju-stdio.py` | `e6870d58c6c59cb1f1d224a66cdaa8455b34057249912b7bce5b9e21f95a074a` |

- 每轮新建 ACP 会话，使用独立临时工作区、`0700` 私有运行目录及本地合成配置。探针从会话 cwd 启动 `stakpak acp`，经 `SHIJU_STAKPAK_RUNTIME` 和 `VW_ACP_LAUNCH_REFERENCE` 进入受管入口。启动器以 `sandbox-exec` 执行同目录的 `stakpak-native acp`，限制写入、派生执行和网络。私有 TOML 和模型缓存选中 `shiju-session`，ACP 当前模型 ID 与回环 `/v1/chat/completions` 收到的模型 ID 一致。模型服务为确定性替身，凭据为合成值。宿主探针仅允许对本轮夹具的读取和明确批准的写入。[启动器](../../../../../patches/acp-model-source/stakpak/launcher.py) · [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/stakpak.rs)
- ACP 握手为 v1，报告 `loadSession=true`、HTTP/SSE 和空认证方法。挂载只声明本轮选中的 `rustcodegraph_query`，用绝对 stdio 命令、空环境和 `--no-watch`。启动器检查 cwd，重启原生进程后核对握手，并经会话私有 HTTP 桥把调用交给保留只读沙箱的 stdio 进程。三轮均向模型声明官方 `view/create/str_replace`，其他原生工具仍受固定白名单限制。外部工具标题与原生身份的冲突拒绝由源码核对。[挂载与审批](../../../../../patches/acp-model-source/stakpak/mcp.py)

## 三轮结果

| 轮次 / ACP 会话 ID | 工具和原生 ACP 调用 ID | 模型请求数 / ACP 终态 / 进程退出 |
| --- | --- | --- |
| 无挂载 / `ce8cf525-5093-4b3b-b1ff-ef66f7101744` | `stakpak__view` / `toolu_781ae5c6db9942b5ad77822f47e5d4da`。仅官方目录，目标 MCP 零调用。 | 2 / `end_turn` / 0 |
| 挂载 / `53f0396b-1b11-4831-979b-b6a2d8bec48d` | `stakpak__view` / `toolu_d3ef539207f94f56a66cd88adfe0f476`；`stakpak_graph__rustcodegraph_query` / `toolu_29bcb7f3856a4f35937ed04e43f7684d`。两项均 `completed`。 | 3 / `end_turn` / 0 |
| 撤除 / `b37511f6-163a-46a8-a510-d30188c33a73` | `stakpak__view` / `toolu_1d18dff21a1648a6b6005de0c87488a5`。新会话传空列表，只有官方目录，目标 MCP 零调用。 | 2 / `end_turn` / 0 |

三轮的官方查看由原生 ACP 委托宿主 `fs/read_text_file`（RPC ID `0`），返回夹具函数内容，模型续轮以 `probe-view` 收到结果。挂载轮原生审批 RPC ID `1` 对应图谱的 `toolu_29bcb7f3856a4f35937ed04e43f7684d`，宿主选 `allow_once` 后实际由 RustCodeGraph stdio 查得 `stakpak_probe_marker  fixture.rs:1`。第三次模型请求同时带有 `probe-view` 和 `probe-query` 的结果。原生 ACP 重编调用 ID，按逐次工具请求、ACP 事件和返回内容配对，模型侧继续使用其发出的 `probe-*` ID。对照轮从目录、ACP 轨迹及未启动挂载桥确认无目标调用，未另采 stdio 帧与服务侧计数。

## 拒绝、写入及入口边界

- **原生写入。** 挂载拒绝轮 `c7b645a9-bec9-422a-8433-4b5d30c4b462` 中，`create` / `toolu_b0c634c52078446dba3cedd241ba9ab9`、`str_replace` / `toolu_08d75695f3ce4395a8fc93962ae06524` 保留原 `allow_once/reject_once`，分别在审批 RPC `2`、`3` 选择拒绝。两项均 `failed`，模型按 `probe-create`、`probe-replace` 收到 `TOOL_CALL_REJECTED`。宿主零写回调，目标仍缺失，原文件内容未变。批准对照轮 `a09740b6-fb5a-410e-8062-2686a377165a` 的原生调用 `toolu_35bf295d201048af9233b36d438a862b`、`toolu_5bbdf79c6e8a42319f5264b63e300807` 均完成，宿主经 RPC `3`、`6` 写入私有夹具，模型续轮收到对应结果。两个写入轮均五次模型请求、`end_turn`、退出 0。
- **MCP 拒绝和取消。** 拒绝轮 `toolu_596f1917fc1b4b77926ed1f5f4999348` 为 `failed`，模型按 `probe-query` 收到拒绝结果，三次模型请求后 `end_turn`。取消轮在 `toolu_37a11975ab5c4ed2a8fc2758c8d55a6e` 审批期间取消后发送迟到批准，桥拒绝执行，原生事件状态为 `completed` 但输出为 `MCP_ERROR`，模型没有图谱结果或第三次请求，提示以 `cancelled` 结束。两轮退出 0。挂载进程对 `session/load`、`session/resume` 返回 `-32000`，随后原挂载会话仍可正常提示。未挂载历史恢复未实测。正常退出后私有桥接配置已移除。
- **原版与通用入口。** 直接运行同摘要的 `stakpak-native acp`，原版使用私有 Provider TOML 和模型缓存选中回环模型。接受携带 stdio `mcpServers` 的 `session/new`，目录仍仅三个官方工具。原生查看 `toolu_626b86c7a6694defb6d62eaa6aac8793` 成功，两次模型请求后 `end_turn`，退出 0。原版忽略建会话的 HTTP、SSE 声明与启动期 HTTP 可发现并执行的结果沿用历史诊断。通用入口的注册和参数已核对，受审会话模型入口另验安装摘要、私有环境及本机回环中继。本轮直连复现启动环境，未重跑宿主模型驱动或普通网关入口。[原版与启动期 HTTP 历史](../../mcp-acceptance/stakpak.md)
- **验证范围。** 本轮复跑已有 Python 定向测试，两项通过，退出 0，覆盖原生审批选项保留、调用归属、完整参数、单次消费和撤权。使用已校验私有制品目录作 `PYTHONPATH`，执行 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s patches/acp-model-source/stakpak -p mcp_test.py`。未运行 Cargo、桌面构建、真实模型、其他平台或网关 SSE。历史真实模型验收单列。[既有网关验收](../../mcp-acceptance/stakpak.md#新标准网关验收通过)

探针与 ACP、模型轨迹保存在本机私有目录。在私有探针的新副本中运行 `python3 probe.py` 并依次传入 `before`、`mounted`、`removed`、`deny-write`、`allow-write`、`deny-mcp`、`cancel-mcp`、`native` 可复跑。新增回执使用 `git diff --no-index --check /dev/null` 检查空白错误。
