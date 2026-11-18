# DeepAgents 动态挂载 RustCodeGraph

2026-11-19 使用 [可复用的验收请求](../ACP验收.md#可复用的验收请求) 在提交 `189d22dc1` 中的版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。三轮使用租户 `10001 / 100010005`、`acp-deepagents / deepagents`、`session_model`、`full_access=true`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。

## 版本和传输

- 私有回环网关复用现有 shiju 构建，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。从开发 SQLite 获取私有快照，只在该网关启用预设。
- 使用 Darwin arm64 上的受管 npm 包 `deepagents-acp@0.1.42`（受管修订 v2）的副本。入口和守卫 SHA-256 均与仓库清单一致，分别为 `d65ca9295356f90c11402e3ec9f39856324329a8a7acbdfe79592444e471abce` 和 `82e173b8284d3496e2032be5bec1c155241164f2f4c07aeff5d36e6f27e2faff`。独立探针确认 ACP v1、`agentInfo.version=0.0.1`、`loadSession=true`、MCP HTTP/SSE 均为 false。
- RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出。RustCodeGraph 1.2.10 重建索引，共 3478 个文件，命中 `app/app.tsx:32` 的 `createRueSiteApp`。挂载轮在副本启用 `ruejs_graph_probe`，使用原二进制的 stdio 与 `--no-watch`，撤除轮前移除配置。网关在挂载轮完成 MCP 服务初始化。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态与答复 | 工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1ef9af305001reLdaacUte5b7W` | 200 / 0 / 6.291225 秒 | 一个 `chat.done`，仅答 `MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1ef9b0d3b001Gof5YMrOn0mYBm` | 200 / 0 / 1.814087 秒 | `chat.model_recovery` 停止，随后 `chat.error`，无 `chat.done` | 0 |
| 撤除 | `ses_1ef9b14c7001ZxZJcZ37ns47rA` | 200 / 0 / 5.540521 秒 | 一个 `chat.done`，仅答 `MCP_NOT_AVAILABLE` | 0 |

各轮 summary 读回均为 `acp-deepagents / deepagents` 和 `full_access=true`。两轮成功对照的 ACP options 在运行中读回 `connected=true`、所选来源 `session_model`、提供商 `alibaba-token-plan-cn`、原完整模型 ID 和 `access_status.support=client_only`。终态读回同一模型，连接已关闭。通用能力列表同时报告 `session_model=unsupported_cli`。挂载轮终态 options 为 `connected=false`、`cli_local`，无 `selected_model`。

首个失败边界是发给受管入口的 `session/new`。SSE 原因为 `acp new_session failed: Internal error`，详情为 `DeepAgents MCP configuration is unavailable`，代理随后退出 0。独立线协议探针用同一个 stdio 服务定义复现 `session/new` 和已有会话的 `session/load` 均返回 ACP `-32603` 与同一详情，空挂载建会话成功。挂载轮未进入模型提示，`rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 均无调用和结果，未生成 RueJS 调查答复。

## 工具接线和权限

受管 CLI 用静态 `config.tools` 创建代理。现有守卫按当前会话匹配调用，仅允许 `read_file` 和 `write_file` 进入宿主审批。原生结果转发可将 `ToolMessage` 按同一调用 ID 转成 ACP 完成事件，入口缺少 MCP 发现、会话绑定及结果接入逻辑。经发布完整性校验的 npm `0.1.44` CLI 同样缺少读取 `mcpServers` 的分支，HTTP/SSE 宣告仍为 false。复验需要可审计的会话级 MCP 接线，覆盖服务选择、调用审批、结果返回、撤除和进程回收。受管拒绝保留。 [npm 发布包](https://www.npmjs.com/package/deepagents-acp/v/0.1.44)

- `node patches/acp-model-source/deepagents/guard_tests.mjs <受管包目录>` 退出 0。覆盖 5 类权限结果、5 项取消检查、宿主撤销、7 类未审工具拒绝和宿主文件错误脱敏。
- `python3 scripts/acp_onboarding_deepagents_acceptance.py <受管入口>` 退出 0。真实受管 CLI 经回环模型夹具验证拒绝、RPC 错误、取消、未知选项、宿主拒写、批准并写入，以及本地编辑拦截、非空 MCP 加载拒绝和取消后继续。夹具使用独立占位令牌，仅用于审批回归。
- 本次仅提交验收记录。Rust 生命周期用例使用模拟对端，覆盖建会话、复用和取消，未走受管 MCP 拒绝分支。本次未运行 Cargo。私有网关已停止且端口关闭，私有凭据和租户数据副本已清理。原始 SSE 和探针收据保存在本机 0700 私有目录。
