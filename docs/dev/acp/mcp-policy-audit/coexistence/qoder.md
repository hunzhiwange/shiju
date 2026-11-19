# Qoder 1.1.88 工具共存复验

2026-11-19，Darwin arm64。受管私有绑定在同一新会话执行了 Qoder 原生 `Read` 和 stdio MCP `read_probe`，模型按各自调用 ID 收到真实磁盘结果。撤除后的新会话保留 `Read`，目标服务零发现、零调用。原版供应商入口停在登录门禁。

## 制品、入口与权限

- 注册 key 为 `qoder`，命令为 `qodercli --acp`。本轮复制现有 npm 安装到私有目录，核对固定 1.1.88 原包的 npm SHA-512 完整性，并确认归档入口与私有原版入口 SHA-256 相同。随后对第二份副本应用仓库补丁并核对两项输出摘要。受管私有绑定仅在 Darwin arm64 审核，其他平台的通用 npm 安装用 `@latest`，本轮未运行。[注册表](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L610) · [固定制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L695) · [平台安装表](../../../../../crates/libs/config-acp-install/src/lib.rs#L94)
- Node 24.22.3，两个入口均运行 `qodercli.js --acp --config-dir <私有目录> --setting-sources user --model shiju/probe`。受管版使用 `SHIJU_QODER_MODEL_CONFIG`、`VW_ACP_LAUNCH_REFERENCE`、0700 私有目录和 0600 绑定，配置 `mcp.lazyLoad=false`。模型只访问该次回环 Chat Completions；会话回读 `shiju/probe`，HTTP 请求模型为 `probe`。原版使用独立 HOME 和相同类型的本地设置，无绑定环境变量，macOS 沙箱仅允许本次模型回环端口的出站 TCP。未沿用历史租户、配置或凭据。[受管驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/qoder.rs) · [补丁说明](../../../../../patches/acp-model-source/qoder/README.md)
- 两版均握手 ACP v1，报告 `qoder-cli 1.1.88`、`loadSession=true`、`sessionCapabilities.resume` 及 HTTP/SSE MCP 能力。受管版各新会话回读原生 `default`，无需批准的工作区读取由原生策略处理，MCP 通过 ACP 单次批准。试验未使用完整访问或启动绕过参数，传输实测为 stdio。[原生权限同步](../../../../../crates/libs/acp-client/src/client/qoder_access.rs)

| 制品 | SHA-256 |
| --- | --- |
| 原版 `bundle/qodercli.js` | `1858e47ccf930ea91f30cfb9d2889e3cfb55c34beb1cf7b66a9a5e1a18efacb3` |
| 补丁后 `bundle/qodercli.js` | `e812bfd63431758a96d7888fefc7e67a02735e0bf69edf6b5e531a261c7a7ed1` |
| 私有绑定模块 | `27753aa78105ebe8fe19a9ed09b34069c5ca6efaa9388da3c8e699a03169c53c` |

## 新会话逐轮证据

私有工作区文件与工作区外附件分别生成随机值。原生读取由 Qoder 执行，MCP 由仓库只读验收服务处理。挂载轮的两个工具调用在同一次模型响应发出。模型目录保留相同的 28 项原生定义，挂载仅增加目标工具的直接声明。

| 轮次 | ACP 会话 ID | 声明与执行 | ACP 终态 |
| --- | --- | --- | --- |
| 无挂载 | `8c1c5dc7-3964-47c7-9654-de7d38fd706f` | 28 项。原生 `Read` 的 `call_before_0` 完成并回传；目标零发现、零调用。 | `end_turn` |
| 挂载 | `89dd43ed-215f-40b3-8b56-b8dc6bc73df1` | 29 项。原生 `Read` 的 `call_mounted_0` 完成；`mcp__coexist_probe__read_probe` 的 `call_mounted_1` 经 `proceed_once` 完成，stdio `tools/call` ID 为 `2`。两个完成事件和模型续轮均各自携带匹配的 ID 与随机值。 | `end_turn` |
| 撤除 | `42f8c1e9-866e-40d6-a53c-900b426ad96b` | 28 项。原生 `Read` 的 `call_removed_0` 完成并回传；目标零发现、零调用。 | `end_turn` |
| 显式拒绝 | `ca58ef4f-9734-4438-ad46-92e2dcbbc16b` | 对 `call_denied_0` 选 `cancel`。ACP 报 `failed`，模型续轮同 ID 收到拒绝文本，目标零调用。 | `end_turn` |
| 取消提示 | `3c1e5c92-1b12-42b6-99c2-33dc6920ab56` | `call_cancel_0` 等待审批时发 `session/cancel`，以取消结果结束审批，目标零调用。 | `cancelled` |

## 加载、恢复及边界

- 受管版通过独立进程对原 ACP ID 分别发送 `session/load` 和 `session/resume`。各自先带原挂载，然后用空 `mcpServers` 重连。加载 ID 为 `f82bbf77-3e4f-4d91-b322-9387f0d7b57b`，恢复 ID 为 `e031cd32-12b9-4770-80e2-d1b96ee325db`。两种请求均成功，带挂载时服务重新启动并列出工具；清空后新提示均保留 `Read` 并以 `end_turn` 结束，目标零发现、零新调用。模型历史仍含旧工具结果。主样本的重连轮仅取到目录，早先独立加载样本 `66874f2a-31ad-44f1-a3da-65487b632a47` 另有单次批准的 `call_load-mounted_1` 完成并回到模型。恢复补跑 `40491016-07b4-44e0-8789-c788b3703732` 中，夹具在未收到新结果时重发同一合成调用，服务实际收到一次新 `tools/call`。单次批准的 `call_load-mounted_0` 完成并同 ID 回到模型；随后空挂载恢复的发现和调用增量均为零。
- 原版握手公布 `qodercli-login`。首次无挂载 `session/new` 返回 `-32000` 与 `Authentication required`，零模型请求、零服务调用。探针退出 2，原版子进程超时后由探针强制结束，信号退出码为 -9。当前没有已授权的供应商登录及模型会话，原版目录、官方读取、批准/拒绝及模型续轮均停在这个前置断点。复验需在获授权的独立供应商会话重复上述步骤。[历史受管验收](../../mcp-acceptance/qoder.md)
- 本轮走直接 ACP，未启动 ShiJu 网关、调用真实供应商、执行 Cargo 或验证其他平台。旧三工具转发只属于历史验收，本轮用了独立 stdio 服务。当前证据未发现受管适配器裁剪原生目录，现有发现及权限逻辑保留。
- 私有证据目录为 `/private/tmp/shiju-qoder32.645LLn`。从仓库根目录用 `python3 <证据目录>/probe.py managed new`、`managed load`、`managed resume`、`managed resume-retry` 复验；脚本为每个入口和模式创建独立输出目录，重跑需先移开旧同名目录。受管探针和 `verify.py` 均退出 0。受管子进程在证据采集后由探针以 SIGTERM 收尾，进程退出码为 143。回执空白检查无诊断。

## 原版登录门禁补跑

在相同固定制品和仅回环出站约束下做三组对照，每组依次请求无挂载、挂载和撤除后的新会话。九次均在 `session/new` 返回 `-32000 Authentication required`，模型请求和 stdio 服务启动均为零。

| 入口 | 参数 | 无挂载 / 挂载 / 撤除 |
| --- | --- | --- |
| 原版 | 仅 `--acp`，空 HOME | 登录要求 / 登录要求 / 登录要求 |
| 原版 | `--acp` 加私有配置目录、用户配置源与回环模型 | 登录要求 / 登录要求 / 登录要求 |
| 受管 bundle 的供应商分支 | 仅 `--acp`，空 HOME，无私有绑定 | 登录要求 / 登录要求 / 登录要求 |

本机执行环境未提供 `QODER_PERSONAL_ACCESS_TOKEN`。官方 ACP 使用 Qoder CLI 的登录状态，也支持个人访问令牌。官方 BYOK 由个人版账号在 Custom 向导中选择受支持 Provider。完成原版比较需要获授权的独立 Qoder 登录和可调用的供应商模型。如需复用同一回环模型，还需确认该环境可通过官方途径配置此 Provider。[Qoder ACP 登录](https://docs.qoder.com/cli/acp#login-and-usage) · [Qoder 自定义模型](https://docs.qoder.com/cli/custom-models)

回看历史证据，原版 1.1.83 只记录了握手及权限字段，受管 1.1.88 的历史网关三轮使用了私有模型绑定。两者均无法补齐当前原版的工具执行和续轮。受管版当前回环证据的复核脚本再次退出 0，任务保留原版登录阻塞。[原版历史权限证据](../../../../shiju/acp-full-access/evidence/qoder.md) · [受管历史验收](../../mcp-acceptance/qoder.md)

私有目录中的 `probe_upstream.py` 支持 `original upstream-default`、`original upstream-explicit`、`managed upstream-no-binding` 三组对照。三次探针均退出 0 并记录每轮协议错误，三个子进程在关闭超时后由探针强制结束，退出码均为 -9。
