# Junie 4100.5 官方工具与 MCP 共存回执

2026-11-19，在 macOS 27.0 arm64 上以固定签名 Junie 26.11.2（4100.5）和当前受管 v2 完成回环三轮。挂载轮的官方文件读取和真实 RustCodeGraph 查询均执行成功，两个结果均进入模型续轮。撤除轮只有官方读取。

## 制品、入口与权限

- 注册 key 为 `junie`，通用发现命令和受管外层启动参数均为 `junie --acp=true`。从历史私有缓存复用固定原版归档字节，在全新私有目录运行仓库安装脚本，退出 0。归档 SHA-256 为 `ad103a996f0a4f5cee051f858bdf0fc33d4ee83b7c9219971d485046f084cd3c`。原生入口摘要为 `4be1f51971aa638ebd9c3caa93e63d2a235e3a4bd133011476f6cbbbcfcc565e`，当前启动器为 `bc1feb4a55ca27f6965a82dd3a180c73cbbb7fee04253a3ee274cc3c63b63be7`，六项文件摘要及 app 的 `codesign --verify --deep --strict` 检查均通过。握手返回 ACP v1、`@jetbrains/junie`、`http=true`、`sse=true`。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [安装](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/install.py)
- 直接 ACP 探针使用 `SHIJU_JUNIE_CONFIG` 指向权限为 `0600` 的私有请求配置，且用 `VW_ACP_LAUNCH_REFERENCE` 绑定本轮启动。配置只使用当轮回环地址和一次性测试认证。受管启动器在 Seatbelt 中运行签名 app 的原生 `--acp=true --model=custom:shiju --model-location=<私有模型目录>`。模型和 MCP HTTP 共用受限监听端口，stdio 由只读沙箱中的真实 RustCodeGraph 1.2.10 执行。每轮在提示前读回 `brave_mode=off`（Safe）及 `v1:6:custom:custom:shiju`。官方读取由 Junie 原生处理，本轮未产生 ACP 审批请求。MCP 只选择单次批准或单次拒绝。[原生启动](../../../../../patches/acp-model-source/junie/launcher.py) · [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/junie.rs) · [MCP 审批](../../../../../patches/acp-model-source/junie/mcp_bridge.py)
- 原版支持自定义模型入口。受管合约只覆盖 Darwin arm64 签名 app。通用命令可被注册发现，模型来源能力只在入口校验通过时公布 `session_model`，Junie 不公布 `cli_local`。本轮使用受管入口直连 ACP，不涉及系统全局 CLI 或其他平台。[能力声明](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/mod.rs)

## 三轮实测

每轮重新启动进程并新建会话，共用隔离工作区。官方 `open` 读取 `native-probe.txt` 的随机标记，外部工具从独立索引查找带随机后缀的符号。提示和模型夹具响应均不包含这两份结果。挂载轮显式传递 `coexist_probe`，名单允许 `rustcodegraph_query`，并同时允许和拒绝 `rustcodegraph_node`。模型使用 Junie 原生的 `enable_mcp` 启用声明中的服务后看到查询，未看到被拒绝的节点工具。无挂载和撤除轮传空服务列表，模型均未收到目标工具目录或结果，ACP 目标工具事件为零。

| 轮次 | ACP 会话 | 工具、ACP 调用 ID、执行方与状态 | ACP 终态 / 受管进程退出 |
| --- | --- | --- | --- |
| 无挂载 | `session-261119-152920-1g5t` | `open` · `c11bae4e-039b-4e06-9b44-e7cb2516c413` · Junie · `completed`；目标 MCP 0 次 | `end_turn` / 0 |
| 挂载 | `session-261119-152928-1o5s` | `open` · `f643f25a-9cd2-4e70-95c7-1fb4439336b8` · Junie · `completed`；`coexist_probe/rustcodegraph_query` · `47567faf-c3c0-44a4-8a10-da96e96065e1` · RustCodeGraph stdio，经受管 HTTP 转接 · `completed` | `end_turn` / 0 |
| 撤除 | `session-261119-152936-10hf` | `open` · `07227b4c-9b11-43b8-be2d-db88a3ea73b5` · Junie · `completed`；目标 MCP 0 次 | `end_turn` / 0 |

挂载轮的原生 MCP 审批 ID 和完成事件 ID 均为 `47567faf-c3c0-44a4-8a10-da96e96065e1`。同轮 Chat Completions 续轮携带 `call_mounted_2` 对应的文件标记和 `call_mounted_6` 对应的实际索引符号。两个对照的续轮也分别匹配 `call_before_1`、`call_removed_1`。服务独立 stdin RPC ID 和退出码未暴露在直接 ACP 记录中。撤除的零调用由空挂载、桥仅按显式挂载建立 stdio peer、模型目录和 ACP 事件交叉核对，无独立服务计数器。

## 拒绝、取消和会话恢复

- 单次拒绝会话 `session-261119-152945-1mpa` 中，Junie 的 `open`（`e3ef97c1-39d4-4afb-880b-2fb06b5f03df`）完成。Junie 对 `coexist_probe/rustcodegraph_query`（`52e71a34-0cbd-4c4b-ab32-0bab2fd46bad`、模型调用 `call_reject_6`）发起审批后收到 `reject_once`，同 ACP ID 终态为 `failed`，续轮无图谱结果，`end_turn`，进程退出 0。
- 已挂载会话 `session-261119-153015-zz3x` 在首个模型请求等待期间发送 `session/cancel`，无工具调用 ID，ACP 返回 `cancelled`，受管进程退出 0。另用 `python3 -B patches/acp-model-source/junie/mcp_bridge_tests.py` 复验转接，4 项通过、退出 0，覆盖同参单次消费、拒绝时 peer 零调用、取消已活动调用和撤除后旧 URL 返回 404。
- 空挂载生命周期会话 `session-261119-153023-165v` 中，Junie 的 `open`（`019b6db6-b7bc-4955-b2e9-c77b4e14b000`）完成，提示返回 `end_turn`。`session/load` 后先关闭会话再执行 `session/resume`，均成功，受管进程退出 0。撤除轮直接在已加载会话上 resume 得到 `session_already_loaded`。带挂载恢复和跨进程恢复未验证。

本轮复用历史探针的 Chat Completions 协议和同一受管启动方式，私有探针及 ACP、模型记录保存在本机验收目录 `shiju-junie-21.GFKqiAg2`。未发现适配器无意裁剪，生产补丁无需调整。未调用真实模型、未运行网关或 Cargo。历史网关另有同一挂载会话六次目标调用、四次官方文件读取的实测。[历史验收](../../mcp-acceptance/junie.md#新版网关三轮验收)
