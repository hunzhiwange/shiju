# Kiro 3.0.0 V3 工具共存复验

2026-11-19，Darwin arm64。原版和受管版均在同一挂载会话执行 Kiro Powers 列表和 stdio MCP 读取。模型按原调用 ID 收到两种结果并结束该轮。撤除后的新会话仍可用 Powers，目标服务零发现、零调用。

## 制品、入口与权限

- 注册 key 为 `kiro`，默认命令为 `kiro-cli acp`。普通入口允许本地原生选模，Windows 另有 PowerShell 查找。此次原版实测使用 `kiro-cli acp --agent-engine v3 --auth-method cli`。受管模型驱动也显式使用这组参数，固定制品安装器仅支持 Darwin arm64。Windows、Linux 和 Intel macOS 本轮未运行。[注册与查找](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L105) · [注册条目](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L573) · [受管模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/kiro.rs)
- 用官方固定归档冷装，SHA-256 为 `0a8f067c4fe6a23a231aa3a18e3e4332120ef9ed3d2d7b6eea907beb8b4cdd8b`。整包直接下载较慢，使用 HTTP 分段下载并合并，对完整归档验摘要后沿安装器顺序提取、补丁并逐项验摘要。原版使用同一经核验的原生二进制与未补丁的嵌入运行时。[官方 3.0.0 归档](https://prod.download.cli.kiro.dev/stable/3.0.0/kirocli-universal-darwin.zip) · [固定制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L272) · [安装路径](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/install.py#L469)
- 独立 ACP 探针使用两个私有 HOME、私有 Kiro 配置及历史，工作区只有合成数据。出站限制为本次回环控制面与模型端口。固定 `KIRO_HOME`、`KIRO_RUNTIME_DIR`、`KIRO_KAS_ENDPOINT`、`KIRO_KAS_CONTROL_PLANE_ENDPOINT`、`KIRO_KAS_REGION`，以合成短期令牌认证。两版均回读 `vibe`、`autopilot=on`、模型 `coexist-primary`。每次 `session/new` 和 `session/load` 都发当前 `_meta.kiro.policyPreset=[]`。只对计划内的 MCP 读取返回 `allow_once`，拒绝样本返回 `reject_once`。未传旧信任 flags。受管版用安装器绑定的 V3/Node 和空私有 Skill 视图。外部运行时及 mandatory MCP 覆盖保持禁用。[授予构造](../../../../../crates/libs/acp-client/src/client/session_meta.rs#L83) · [加载请求](../../../../../crates/libs/acp-client/src/client/protocol.rs#L421) · [V3 补丁](../../../../../patches/acp-model-source/kiro/apply.py)
- 两版握手均为 ACP v1，`loadSession=true`、`http=true`、`sse=true`，无通用 `resume` 能力声明。本轮实测 stdio 和 `session/load`。先前无绑定模型握手的能力值只适用于当时入口。[历史验收](../../mcp-acceptance/kiro.md)

| 受管文件 | SHA-256 |
| --- | --- |
| 启动器 `kiro-cli` | `5728781bd5d77735d912467b15ba37c67220830952c17cf2e3066005044ee8e6` |
| `native/kiro-cli` | `4e4b2ee6b5021624d23c6ef78b18624c438ee898596a683f5a55889c512965bc` |
| 随包 Node | `46d1713acc486d844a8b944fb0f606366aebc70d6601646b9807abc7cb1a2201` |
| 补丁后 `acp-server.js` | `26907b1f2895cf28a850d7720b340cb206965d8ffbc17cb62dd78276f2dbd02d` |

## 逐轮证据

两个入口的前三轮初始模型工具目录均为 14 / 16 / 14 项。挂载只增加 Kiro 内部 `tool_load` 和 `tool_call`，原生目录完整保留，其中含 `kiro_powers`。探针等服务完成 `tools/list` 再发挂载提示。较早探针在发现前发提示，只观察到原生目录。随后按固定版协议发 `tool_load` 披露 `coexist_receipt::read_probe` 的结构，再通过 `tool_call` 执行。披露本身没有触发服务调用。

各入口用独立的会话和模型记录。以下合成调用 ID 在两次独立运行中相同，ACP 终态及退出码按受管版 / 原版列出。每次挂载的 stdio 服务先收到发现探针，再处理 MCP `initialize`（ID `0`）与 `tools/list`（ID `1`）。

| 轮次 | 调用 ID、执行方与结果 | ACP 终态 | CLI 退出码（受管 / 原版） |
| --- | --- | --- | --- |
| 无挂载 | Kiro `kiro_powers` 的 `list`，`call_before_power`，`completed`；模型收到空列表，MCP 服务零发现、零调用 | `end_turn` | 1 / 143 |
| 挂载 | Kiro `kiro_powers` 的 `list`，`call_mounted_power`；Kiro `tool_load`，`call_mounted_load`；本地 stdio `@coexist_receipt/read_probe`，`call_mounted_mcp`，MCP RPC ID `2`。三项均 `completed`，读取经 `allow_once`，模型分别收到空列表、结构和实际附件标记 | `end_turn` | 143 / 1 |
| 撤除后新建 | Kiro `kiro_powers` 的 `list`，`call_removed_power`，`completed`；模型收到空列表，MCP 服务零发现、零调用 | `end_turn` | 1 / 143 |
| 显式拒绝 | Kiro `tool_load`，`call_deny_load`，`completed`；对 `call_deny_mcp` 返回 `reject_once`，ACP `failed`。模型收到同 ID 拒绝结果，目标服务零调用 | `end_turn` | 143 / 1 |
| 新进程加载并重传挂载 | 两版均复用各自挂载轮会话 ID。服务重新发现。stdio 读取 `call_reload_mcp` 经 `allow_once`，MCP RPC ID `2`。Kiro `tool_load` 再披露一次，ID `call_reload_load`。两项 `completed`，新读取标记按 ID 回传 | `end_turn` | 1 / 1 |
| 新进程加载并撤除 | 仍用挂载轮 ID，提交空 `mcpServers`；当前目录回到 14 项。模型输入可见旧历史回执，本轮没有工具调用，目标服务零发现、零调用 | `end_turn` | 1 / 1 |
| 取消 | stdio 服务发现后，回环模型收到提示且尚未发出工具调用时发送 `session/cancel`，目标服务零调用 | `cancelled` | 1 / 1 |

受管挂载会话为 `sess_49e5346c-3c56-4a0b-a7d7-a416947afce0`，原版为 `sess_ea9463a3-bb74-4de3-a460-3fc57ab63c0c`。所有轮次完成相应 ACP 请求及 `session/close`。关闭 stdin 后，原生启动器报告 V3 未在 5 秒内退出时返回 1，探针等待到期后向自己的进程组发送 SIGTERM 时记录 143。两个固定入口均有相同的关闭问题，出现于请求终态之后。

## 范围和复跑

两版的权限请求、目录与工具行为一致，本轮未证实适配器对挂载工具的额外裁剪，未修改运行时代码。直接 ACP 复验覆盖原生发现、审批、结果回传和加载，不包含真实模型、网关投影与宿主逐服务名单。未复验 `allow-all` 预设切换、私有 Skill 正向挂载及 terminal 参数转发。未运行 Cargo 或桌面构建。

私有目录 `/private/tmp/shiju-kiro24.zmWxsEs1`（0700）保留固定版、探针和 JSON 回执。每次运行使用独立的临时配置和工作区，结束后移除并关闭回环端口。

```sh
cd /private/tmp/shiju-kiro24.zmWxsEs1
python3 probe.py runtime/kiro-cli managed4
python3 probe.py runtime/native/kiro-cli upstream4
python3 check.py managed4.json upstream4.json
```
