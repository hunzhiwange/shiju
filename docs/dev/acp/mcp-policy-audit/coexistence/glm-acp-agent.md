# GLM ACP 官方工具与 MCP 共存复验

2026-11-19 在 macOS arm64 复验固定版 `glm-acp-agent@1.17.1`。受管入口的三轮 ACP 回环通过。挂载轮实际读取本地文件并查询 RustCodeGraph，两条结果按各自 ID 进入续轮。撤除后新会话的目标服务零声明、零访问。各名单场景均保留相同的 9 个官方工具，本轮未发现需要修复的适配器目录裁剪。

## 入口与权限

- 注册 key 为 `glm-acp-agent`，预设为 `acp-glm-acp-agent`。原版入口为 `dist/index.js`，注册表保留本地命令与固定 npm 版本的回退。带工具策略的会话由宿主要求受审入口并校验摘要。当前生产补丁只过滤各 MCP 服务自己的目录，拒绝优先。旧私有 `mcpOnly` 原型仅作为历史记录。[分组审计](../1.md#glm-acp-agent) · [原版实测](../../mcp-acceptance/glm-acp-agent.md#acp-探针与清理) · [受管合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- macOS 27.0（26A428）、Node 24.22.3、npm 11.19.1。生产安装器使用私有 npm 缓存，校验固定包完整性、原始文件及补丁后文件。实际启动为 `"$HOME/n/bin/node" "$runtime/glm-acp-agent-1.17.1-model-source-v2/node_modules/glm-acp-agent/dist/index.js"`，无额外参数。ACP v1 初始化回读 1.17.1，并声明 HTTP MCP、load、fork、resume。
- SHA-256 与合约一致。补丁为 `e0a9e4a02b4e3e939dc39edb8133a6f6d30ff50365df124d91d22445d90f0fdc`，入口为 `d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98`，执行器为 `c775d1418f84f2978ec421da50fa335f8334599afcbe47d8987882a4735e4bd6`，MCP 客户端为 `45bf201cd76da3418ee7557a7f49e11c3acc6d98ad3bfc7784c5eb88ef75d935`。
- 直接探针通过 `ACP_GLM_MODEL` 和 `ACP_GLM_BASE_URL` 指向回环 Chat Completions，每次请求均回读合成模型 `glm-fixture`。配置、会话和工作目录均为私有临时路径，令牌为占位值。只读调用未发起宿主审批，写入审批明确选择 `reject_once`，目标文件不存在。生产 `session_model` 另通过 `ACP_GLM_AVAILABLE_MODELS` 和请求级回环中继绑定模型，用 `XDG_CONFIG_HOME`、`ACP_GLM_SESSION_DIR` 隔离状态。Unix 校验所有者及 0700，Windows 启动经验证的 JS 时使用绝对 Node 路径，并校验私有状态目录 DACL。生产宿主对带策略挂载的全拒绝行为见跨平台回执。

## macOS arm64 正向三轮

复用仓库 [HTTP 夹具](../../../../../patches/acp-model-source/glm-acp-agent/managed-coexistence.test.mjs)，原命令 `node --test patches/acp-model-source/glm-acp-agent/managed-coexistence.test.mjs` 退出 0，1 项通过。私有副本补充 stdio 三轮、全允许、混合名单、取消及恢复。补充探针首次冷装超过 80 秒，保留本轮私有缓存重试成功。恢复检查首次误用了 SDK 方法名，改用 SDK 的会话恢复方法后通过。下列结果使用私有工作目录的 `fixture.txt` 和单文件 Rust 索引。RustCodeGraph 1.2.10 以 `rustcodegraph serve --mcp --path "$project" --no-watch` 运行，stdio 记录器原样转发帧。MCP 服务名为 `readonly_graph`，仅允许 `rustcodegraph_query`。

| 轮次 | ACP 会话 | 工具、调用 ID、执行方与状态 | MCP 审计 |
| --- | --- | --- | --- |
| 无挂载 | `dddc212f-4895-45c9-ab49-885a80cbd77d` | `read_file` / `call_unmounted_0` / GLM 官方执行器 / `completed` | 目录 0 项，零访问 |
| 挂载 | `8790d4cb-6f82-427e-993d-b15056395b69` | `read_file` / `call_stdio_0` / GLM 官方执行器 / `completed`；`rustcodegraph_query` / `call_stdio_1` / RustCodeGraph stdio / `completed` | 目录 1 项，一次 initialize、一次 tools/list、一次 tools/call |
| 撤除 | `735a0aac-0083-4a52-9ab1-1f1896a318a3` | `read_file` / `call_removed_0` / GLM 官方执行器 / `completed` | 目录 0 项，审计无增量 |

三轮各两次模型请求，第二次按相同 `tool_call_id` 读到 `native read result`，挂载轮还读到 `probe.rs` 中的 `glm_coexistence_marker` 查询结果。三轮均以 `end_turn` 结束。Node 进程退出 0，探针退出 0，1 项通过。此处为直接 ACP 回环，网关请求 ID、HTTP SSE 终态以及真实模型证据见后文的平台记录。

## 名单、拒绝与生命周期

| 场景 | 同 ID 的执行结果 | 服务侧收到的调用 |
| --- | --- | --- |
| 无名单 | `call_unrestricted_0` 官方 `read_file` 完成，`call_unrestricted_1` 的 `fixture_denied` 完成 | 服务 a 收到 `fixture_denied` |
| 两服务全允许 | `call_all_allow_0` 官方读取完成，`call_all_allow_1` 的 `fixture_sibling` 完成 | 服务 b 收到 `fixture_sibling` |
| 拒绝与允许混合 | `call_mixed_0` 官方读取完成，`call_mixed_1` 的 `fixture_denied` 失败，`call_mixed_2` 的 `fixture_sibling` 完成 | 仅服务 b 收到 `fixture_sibling` |
| 逐服务允许且冲突 | `call_allowlist_0` 官方读取完成，`call_allowlist_1` 的 `fixture_lookup` 完成，`call_allowlist_2` 的 `fixture_error` 因 `isError=true` 标为失败。`call_allowlist_3` 的 `fixture_denied` 命中拒绝优先，`call_allowlist_4` 的 `fixture_other` 未获允许，`call_allowlist_5` 的官方 `write_file` 被宿主拒绝，三者均失败 | 仅服务 a 收到 `fixture_lookup` 和 `fixture_error` |
| 加载并改为仅拒绝 | `call_load_0` 官方读取和 `call_load_1` 的 `fixture_lookup` 完成，`call_load_2` 的 `fixture_denied` 失败 | 仅服务 a 收到 `fixture_lookup` |
| 恢复仅拒绝会话 | `call_resume_0` 官方读取和 `call_resume_1` 的 `fixture_lookup` 完成，`call_resume_2` 的 `fixture_denied` 失败 | 仅服务 a 收到 `fixture_lookup` |
| 分叉并切换名单 | `call_fork_0` 官方读取、`call_fork_1` 的 `fixture_sibling` 完成，`call_fork_2` 的 `fixture_denied` 失败 | 仅服务 b 收到 `fixture_sibling` |

表中普通轮次均为 `end_turn`，成功和失败结果都按原 ID 进入第二次模型请求。独立取消轮在模型响应流等待期间发送 `session/cancel`，返回 `cancelled`，当轮只有一次模型请求和零工具调用。随后同一 ACP 会话的 `call_after_cancel_0` 官方 `read_file` 完成并进入续轮。已覆盖每服务不同工具的路由，尚未验证两个服务公布同名工具或工具执行中取消。

## 平台证据边界

- **macOS arm64 网关与真实模型**：复用 [跨平台计划的原生结果](../../mcp-acceptance/glm-cross-platform.md#macos-arm64-实测)。槽位 3 现有 `shiju 0.6.0` 与回执 SHA-256 一致（`653d967c267fe9ee1cb68e27109b6bccbbff2584b679c5611ada530bf702e2cb`）。既有一键安装、三轮 SSE 与定向共存均通过，挂载回读 `session_model`，宿主 `deny-all` 拒绝带策略挂载且服务零 `tools/call`。已有篡改和 0755 状态目录拒绝回执。本轮新增的是直接 ACP 证据，未重新调用指定云模型、运行 Cargo 或重新构建网关。
- **Linux 与 Windows 原生**：复用 [Linux arm64 容器结果](../../mcp-acceptance/glm-cross-platform.md#linux-arm64-容器证据)，包括冷装和一个通过的回环测试。Linux 双架构及 Windows 双架构缺授权原生环境，首个缺证步骤是从空私有目录运行目标平台当前 ShiJu 的一键安装并启动受审入口。Windows 另待实测 npm shim、私有 DACL 和重解析点。macOS x86_64 原生结果也仍缺，由跨平台任务保留。

本轮私有安装、模型监听及 stdio 子进程已退出，工作目录与状态已清理。脱敏的探针、代理和逐轮回执保存在本机临时目录 `$TMPDIR/glm-task2-6zlsnc8i/`，最终回执为 `receipt-resume-final.txt`。
