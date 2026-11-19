# Goose 1.57.0 工具共存复验

2026-11-19，Darwin arm64。受管制品与官方原版均通过新会话三轮、单次拒绝、带挂载加载和取消。两者清空 `session/load.mcpServers` 后均重新发现并公布旧服务。探针在审批处拒绝该次调用，目标服务没有执行。恢复撤除仍待回归。

## 制品与权限

| 入口 | 本次制品及 SHA-256 | 范围 |
| --- | --- | --- |
| 注册 `goose acp` | 官方 v1.57.0 Darwin arm64 发行包 `80d43c49b77302777e9d2fe32faf3c40c1028dafbea1e0a39b0378404a8a074b`，解包二进制 `4576d80220f31d1deba029d0dbebe84f2f6a4a3762e29f395b7755406f52a72e` | `cli_local` 保留原生 Provider 与模型选择。原版实测拒绝受管 flag，退出 2。 |
| `goose acp --shiju-model-source` | 本机既有受管 v1 安装回执，二进制 `6facb500ca46839afcbe45617ae1ecf261b28022ea45b20d63c86389f6d8162a` | 当前受管发布为 macOS arm64。回执的固定源码及补丁摘要与合约逐项一致。 |

官方包来源为 [v1.57.0 release](https://github.com/aaif-goose/goose/releases/tag/v1.57.0)。本轮核对了 [注册参数](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L465)、[制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L211)、[入口验证](../../../../../crates/libs/acp-agent-registry/src/managed_runtime.rs#L301) 和 [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/goose.rs)。挂载走通用 ACP stdio。两版握手均为 ACP v1，`loadSession=true`、MCP `http=true`、`sse=false`，无 `resume` 能力声明。仅实测了 stdio 和 `session/load`。

独立 ACP 探针为每种入口创建私有 HOME、Goose profile、空白工作区和历史目录。全部会话读回 `approve`、OpenAI Provider 和 `coexist-primary`。回环模型使用 Chat Completions，实际 wire 请求均通过固定模型、路径及短期本地凭据校验。受管轮按现有驱动使用模型锁、私有启动引用及隔离配置。夹具仅为确定的本地只读调用返回 `allow_once`，为拒绝和恢复撤除样本返回 `reject_once`。原生 `shell` 仅读工作区自有文件，MCP 附件独立放在工作区之外，两个随机标记均从实际工具结果取证。

## 逐轮记录

三次独立执行分别使用受管 flag、官方原版、受管二进制的原生入口。下列 ID 是各独立私有 profile 的本地 ACP ID，三组复用了相同的合成调用 ID。每组七个 Goose 子进程均退出 0。一次提示可能先发辅助模型请求，目录计数取执行提示的请求。

| 轮次及 ACP 会话 | 工具、调用 ID 与执行方 | 终态及模型续轮 | MCP 审计增量 |
| --- | --- | --- | --- |
| 无挂载 `20261119_1` | 原生目录可见，无调用 | `end_turn` | 无发现，0 次调用 |
| 挂载 `20261119_2` | Goose `shell`，`mounted-native`。stdio `coexist_receipt__read_probe`，`mounted-mcp`，MCP RPC ID `3` | 两次均单次批准，ACP `completed`。两份实际读取标记按原 ID 返回模型后 `end_turn` | `initialize` ID `1`、`tools/list` ID `2`，1 次 `tools/call` |
| 撤除后新建 `20261119_3` | 原生目录可见，无调用 | `end_turn` | 无发现，0 次调用 |
| 显式拒绝 `20261119_4` | 模型发出 `coexist_receipt__read_probe`，`deny-mcp`，审批 `reject_once` | ACP `failed`，模型收到同 ID 失败结果，无附件标记，`end_turn` | 仅发现，0 次调用 |
| 重连加载并重传 `20261119_2` | 新进程加载原会话，stdio `coexist_receipt__read_probe`，`reload-mcp`，MCP RPC ID `3` | 单次批准，ACP `completed`，模型收到同 ID 的附件标记后 `end_turn` | 重新发现，1 次调用 |
| 重连加载并清空 `20261119_2` | 空 `mcpServers` 仍公布旧工具，模型发出 `load-mcp`，审批 `reject_once` | ACP `failed`，模型收到同 ID 失败结果，无新附件标记，`end_turn` | 旧 stdio 服务被重新启动并发现，0 次调用 |
| 取消 `20261119_5` | 挂载后等待回环模型进入提示，再发送 `session/cancel`，无工具调用 | `cancelled` | 仅发现，0 次调用 |

受管制品带 flag 和不带 flag 的目录均为 10/11/10，原版发行制品为 17/18/17。两种制品的挂载目录都等于各自原生目录加上唯一目标工具，均保留 `shell` 和 `tree`。官方包额外公布 `analyze`、四项 `apps__*`、`extensionmanager__list_resources`、`extensionmanager__read_resource`。该差异随制品出现，关闭模型锁后依旧存在。当前固定补丁没有改动原生扩展选择，未证实有 ShiJu 适配器挂载裁剪。历史限定三个图谱工具的设置属于服务转发器。本轮采用独立的单工具只读服务。[历史复验](../../mcp-acceptance/goose.md)

## 恢复撤除与待证范围

最早偏差是 `session/load` 收到空列表后重新启动旧的 stdio 脚本，随后模型看到旧工具。两种固定制品同样复现。固定上游的 `prepare_session_for_activation` 仅合并非空列表，空列表跳过更新，仍从持久化会话加载启用项。当前 ShiJu [加载请求](../../../../../crates/libs/acp-client/src/client/protocol.rs#L420) 把当前服务列表传给 ACP，客户端缓存键也包含服务列表。更换连接不足以清除上游持久化扩展。任务 43 需在宿主恢复入口复跑当前列表由有到无的场景，并以旧服务零发现、零调用为验收。[固定上游实现](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/acp/server.rs#L1052-L1109)

本次证据来自直接 ACP 与回环模型。未运行网关和真实供应商，网关请求及会话 ID、宿主逐服务名单和生产恢复撤除未复验。未改运行时代码，未运行 Cargo 或桌面构建。Linux、Windows、Intel macOS 和其他传输无本轮运行证据。

## 复跑

私有证据目录 `/private/tmp/goose-coexist.lDFTi4`（0700）保存 `probe.py`、`mcp.py`、`check.py` 和三组脱敏 JSON。复跑时在该目录执行以下命令，两种二进制先按上表校验。每次启动的临时 profile、历史、工作区及回环监听均已清理。

```sh
PYTHONDONTWRITEBYTECODE=1 python3 probe.py "$GOOSE_MANAGED" managed.json --managed
PYTHONDONTWRITEBYTECODE=1 python3 probe.py "$GOOSE_OFFICIAL" upstream.json
PYTHONDONTWRITEBYTECODE=1 python3 probe.py "$GOOSE_MANAGED" managed-native.json
python3 check.py managed.json upstream.json managed-native.json
```

三个探针入口与证据核对均退出 0。证据核对覆盖 21 轮，包含三次明确复现恢复撤除偏差。
