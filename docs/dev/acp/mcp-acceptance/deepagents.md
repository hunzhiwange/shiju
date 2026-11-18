# DeepAgents 动态挂载 RustCodeGraph

## 受管 v2 基线

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

## 受管 v3 适配

固定 npm 发行版维持 0.1.42。v3 在 `session/new` 读取显式挂载，复用 Cortex 的受审 macOS stdio 沙箱查询目录和执行工具。支持绝对路径、无额外环境变量的 stdio。发现和执行分别启动短生命周期 MCP 子进程，使用私有临时 HOME，禁止网络和派生子进程。挂载工具以服务名限定名称，绑定创建该工具集的会话。每次执行前沿用宿主审批，并通过 ACP 同一调用 ID 发布参数、状态和结果。加载仅接受会话原有的选集。更换或撤除选集使用新会话，无挂载仍沿用原生守卫。

仓库安装脚本在私有目录完成 v3 安装并校验五项受管文件摘要。聚焦命令 `python3 patches/acp-model-source/deepagents/mcp_test.py <受管入口> <RustCodeGraph> <已索引的 RueJS 副本>` 使用真实 RustCodeGraph 和回环模型。v2 在挂载建会话处退出 1。v3 退出 0，覆盖无挂载、批准挂载、拒绝挂载、撤除，以及相同选集加载成功和改变选集加载失败。真实输出含查询定位 `app/app.tsx:32`、调用者 `mountRueSiteApp` 和 `useApp` 源码中的 `_$createApp`，模型续轮收到相同结果。三种工具各自的模型入参、宿主审批和 ACP 完成事件可按调用 ID 对应。批准才执行，拒绝后输出明确错误。原守卫和受管 CLI 审批回归通过。私有测试为 SQLite 索引保留 WAL sidecar，使只读沙箱可以访问原索引。

守卫测试同时覆盖已选 MCP 工具的迟到审批。审批等待期间取消提示后，即使收到批准也不会执行工具。槽位 6 的 `python3 scripts/cargo_slot.py cargo test -j 1 --locked -p acp-agent-registry --lib deepagents_session_mcp_uses_a_separate_reviewed_install` 有效执行 1 项，通过。

## 受管 v3 指定模型复验

同一 RueJS 提交、固定请求、租户、模型与 RustCodeGraph stdio。槽位 6 的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为仓库的 `target/rust-slot-6`。同一个槽位入口先执行上述定向测试，再执行 `cargo build -j 1 --locked -p vw-cli --bin shiju`，退出 0，耗时 18 分 40 秒，有既有未使用导入告警。网关 SHA-256 为 `6954a2bb76ca26aa39c3b0dd909786c68b85f336648526ef39651b1e3a31f718`。受管安装使用固定 npm 包 0.1.42 v3。

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态与答复 | 完成调用 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f03493be001BFZ73Poig0C7TR` | 200 / 0 / 4.608904 秒 | 一个 `chat.done`，仅答 `MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1f034a6d700141lP6Jkbv9EYIF` | 200 / 28 / 300.008961 秒；原游标续传 200 / 0 / 209.262766 秒 | 一个 `chat.done`，无 `chat.error`，给出完整调查答复 | 252 |
| 撤除 | `ses_1f03c6f21001HqPz5QUbUwWP1G` | 200 / 0 / 6.544528 秒 | 一个 `chat.done`，仅答 `MCP_NOT_AVAILABLE` | 0 |

三轮 summary 均确认 `acp-deepagents / deepagents`、`full_access=true`。运行中 ACP options 读回原完整模型、`session_model` 和 `client_only`，终态仍显示相同模型且连接关闭。挂载续传保持原会话 ID 和请求标识。代表性开始及完成事件通过同一调用 ID 对应，原始输出均为 MCP 文本块且 `isError=false`。

| 工具 | 调用 ID | 输入与输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_ea78e6c2d3214af1af268f39` | `search=createRueSiteApp` 命中 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_23241e4ae40f4076a2e66480` | `symbol=createRueSiteApp` 返回 `mountRueSiteApp`（`app/app.tsx:39`） |
| `rustcodegraph_node` | `call_d4c3bc9298934270b62703ab` | 定位 `packages/runtime/src/hooks/useApp.ts:4`，回读转发 `_$createApp` 的源码 |

答复正确串起 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp`，引用 SSR 守卫、缺少 `#app` 时抛错、router 与 i18n 的安装顺序，以及 `/rust-layers` 布局绕过。答复从挂载的追加行为推断快照会残留，提出静态文档避免重复 DOM 和导航至 `/rust-layers` 时无残留布局两项可检验场景，未执行浏览器实测。它记录了顶层调用在调用图缺失、源码仍存在的差异。插件实现的部分细节系由源码片段推断，i18n 的 `install` 在原文件 903–905 行还会为当前容器注入 `I18nContext`。

挂载轮 252 个完成调用均来自 `ruejs_graph_probe`。除三种目标工具外，模型还调用了 `status`、`files`、`explore`、`callees`、`impact` 共 15 次。该轮完成真实工具闭环，未满足仅使用三种目标工具的验收条件。

用同一网关二进制、同一请求和同一工作区，以三个新会话再跑一组。两轮对照均只答 `MCP_NOT_AVAILABLE`，工具事件为零。三轮终态无 `chat.error`，summary 和 ACP options 与首组相符。

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f03d00fd0017wx8yOZWocYSw2` | 200 / 0 / 4.264233 秒 | 一个 `chat.done` |
| 挂载 | `ses_1f03d12d1001MI8jLsAPBqjc1W` | 200 / 28 / 300.009013 秒；续传一 200 / 28 / 300.011092 秒；续传二 200 / 0 / 282.218998 秒 | 一个 `chat.done`，两次续传保持同一会话 |
| 撤除 | `ses_1f04a8b6d001su2waZeyBSxaSm` | 200 / 0 / 5.117841 秒 | 一个 `chat.done` |

第二组挂载三种目标工具都成功，另有 34 次同服务辅助工具成功。4 次查询失败均指定了没有独立索引的子目录为 `projectPath`。绕开 DeepAgents 直接向原 RustCodeGraph stdio 发送同样的四个参数，均复现 `isError=true`。答复仍给出主调用链、SSR、容器错误、插件接入、布局绕过、快照推断与两项验证场景。两组均无其他服务、文件或终端工具事件，严格工具范围回归仍待通过。

两个私有网关及其端口均已关闭，私有挂载配置、授权及租户数据副本已清理。原始 SSE 和探针收据留在本机 0700 私有目录。
