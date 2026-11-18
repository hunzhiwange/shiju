# Amp ACP 动态挂载 RustCodeGraph

2026-11-19，Amp 的挂载轮在 ACP 建会话时被受管入口拒绝。无挂载和撤除轮均通过。任务保留为已阻塞。

## 环境与请求

- Darwin arm64，受管适配器 `amp-acp@0.12.0`，本地 ThreadWorker `@sourcegraph/amp@0.0.1778732631-gc5875c`。安装目录版本为 `amp-acp-0.12.0-model-source-v1`。适配器入口、控制桥、启动器及引擎四个摘要均匹配仓库契约。
- 复用已有回环网关构建，健康接口与文件 SHA-256 均为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。网关使用独立配置根和数据目录。首次三轮在宿主凭据检查处返回 `ACP session_model requires Provider credentials`。经私有网关的租户鉴权接口写入已有凭据后，按同一请求重新执行三轮。
- 从 `189d22dc1` 提取[复用请求](../ACP验收.md#可复用的验收请求)。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。使用 `acp-amp-acp / amp-acp`、租户 `10001 / 100010005`、`full_access=true`、`session_model` 及 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出干净副本。索引 3478 个文件，`createRueSiteApp` 位于 `app/app.tsx:32`。挂载使用原 RustCodeGraph 的 stdio 与 `--no-watch`，服务名为 `ruejs_graph_probe`。私有网关观察到宿主启动了对应的 stdio 进程。

## 三轮结果

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态与答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef8d3dc7001haVg4RHh8nonKy` | 200 / 0 / 3.927 秒 | `chat.done`，最终答复 `MCP_NOT_AVAILABLE` |
| 挂载 | `ses_1ef8d4e5e001w8fG1cotRs1l1m` | 200 / 0 / 1.011 秒 | `chat.error`，`Amp selected MCP isolation is unavailable` |
| 撤除 | `ses_1ef8d5417001N8U8L9zkWyDmB5` | 200 / 0 / 4.022 秒 | `chat.done`，最终答复 `MCP_NOT_AVAILABLE` |

三轮 UI summary 均回读到 `acp-amp-acp / amp-acp` 与 `full_access=true`。无挂载和撤除轮的 ACP options 回读到原指定模型和 `source=session_model`。挂载轮建会话失败，ACP options 无所选模型。三轮均无 `chat.tool` 事件。挂载轮的 `rustcodegraph_query`、`rustcodegraph_callers` 和 `rustcodegraph_node` 均无输出，调用链及答复准确性无法验收。

## 阻塞边界与权限

- 对同一受管入口单独发 ACP v1 请求。握手声明 `loadSession=true`，HTTP 与 SSE MCP 均为 `false`。携带实际 `ruejs_graph_probe` stdio 定义的 `session/new` 和 `session/load` 均返回 JSON-RPC `-32603`，详情同挂载轮。两次空列表 `session/new` 均成功并返回不同的会话 ID。
- 核对 npm 中同版本的原始 ThreadWorker，入口 SHA-256 为 `458435712c9c4e222822847fc9e70080cdf023ffb3c820e959a30425a22a5836`，匹配补丁输入。原生 `--mcp-config` 将 flag 条目并入全局与工作区设置。受管补丁在工具发现前清空这些配置，并将工具执行交给只识别宿主工具白名单的回环桥。直接传入 flag 仍会被清空，恢复原始合并会重新引入环境配置。当前受管路径缺少只接受会话所选服务且经宿主审批的 MCP 执行接口。
- 使用当前受管控制桥和拒绝审批的宿主桩分别探测 `Read`、`create_file`、`edit_file` 和 `Bash`。四次均请求审批且收到拒绝，文件与终端调用均为零。直接请求 MCP 工具返回 `Amp tool has no host route`。现有门禁继续生效。待有会话级 MCP 工具路由后，复跑批准、拒绝、撤除和原模型三轮。
- 本轮修改仅为记录，未执行 Cargo。私有网关和对应 stdio 进程均已退出，端口关闭。已清理私有凭据副本及项目挂载文件，原始 SSE 与 ACP 帧保留在本机权限 0700 的验收目录。

## 受管 v2 复验

沿用同一固定版 ThreadWorker 的 stdio 连接、工具注册与结果回传。会话从 ACP 新建、加载或恢复请求接收 stdio 选择，加载新选择前撤销旧控制令牌并关闭旧 Worker。受管 Worker 从私有环境读取当前选择，原有配置目录、插件及远程 MCP 仍受隔离。仅将当前所选服务的工具声明转给模型。实际调用前，宿主以绑定的会话和工具 ID 逐次请求审批。没有挂载的会话沿用原有工具集。

- 针对 v1 的同一 ACP 建会话断言先因 `-32603` 失败，替换为 v2 后通过。两次原生回环探针使用实际 RustCodeGraph 和合成推理。批准场景按顺序完成 `rustcodegraph_query`、`rustcodegraph_callers` 和 `rustcodegraph_node`。三个完成事件与同一工具 ID 的模型续轮结果对应，分别返回 `createRueSiteApp`、调用者 `mountRueSiteApp` 和 `useApp`。拒绝场景返回 `is_error=true`，随后成功结束。两种场景均用空列表加载同一会话并再次发提示，模型的工具清单均无 MCP。另一次探针经 `session/resume` 传入空选择，续轮同样无 MCP。stdio 进程探针确认未继承中继令牌或 Worker 配置，退出后对应 PID 均已消失。另在私有副本配置一个环境 MCP 标记命令，探针中的选中服务仍工作，环境命令未执行。
- `node --test patches/acp-model-source/amp-acp/local.test.mjs` 退出 0，4 项通过。`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_amp_acp acp_onboarding_amp_acp_permission_denial` 的等价入口在槽位脚本内运行，退出 0，1 项通过、3 项过滤。编译缓存与构建目录均为仓库的 `target/rust-slot-3`。
- 同一槽位的定向 CLI 构建退出 0，耗时 17 分 57 秒。私有网关健康接口的构建 SHA-256 为 `06086ba0fc9580bd82fdcf1327696acb6ee648727ed2b054e1ee32d60d8f37d5`。两次系统冷安装均为 HTTP 200，分别耗时 1.567 和 1.341 秒。安装目录均为 `amp-acp-0.12.0-model-source-v2`，四个实际文件摘要与受审契约相符。

### 指定模型复跑

两组沿用原提示、同一模型、同一 RueJS 提交和 RustCodeGraph 1.2.10。第一组直接挂载原 stdio。第二组用私有 stdio 代理仅公布三种目标工具，把原始参数和结果交给同一 RustCodeGraph 二进制。两组均从空配置开始，以新会话执行每一轮。

| 工具集合 | 轮次 | 新会话 | HTTP / curl / 秒 | SSE 与结果 |
| --- | --- | --- | --- | --- |
| 原生 | 无挂载 | `ses_1f00ec35f0013figRC5kRBiPh3` | 200 / 0 / 5.240 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |
| 原生 | 挂载 | `ses_1f00ed934001ur1aZN2OggGqbQ` | 200 / 0 / 266.344 | `chat.done`，三种目标工具成功，模型选择了额外工具 |
| 原生 | 撤除 | `ses_1f012eabb001BT9a0v6WrA44JG` | 200 / 0 / 4.066 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |
| 三工具 | 无挂载 | `ses_1f0130516001pT6oSRDgZqcQLP` | 200 / 0 / 3.924 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |
| 三工具 | 挂载 | `ses_1f013159a001xieFwSHMKkxJjd` | 200 / 0 / 116.858 | `chat.done`，三种目标工具成功，一次错误路径读取 |
| 三工具 | 撤除 | `ses_1f014df5e001ERWvSgqbt6isEr` | 200 / 0 / 3.691 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |

六轮 UI summary 均为 `acp-amp-acp / amp-acp` 且 `full_access=true`，ACP options 均回读指定 `session_model`、provider 和完整模型 ID。两轮挂载运行时均为 `connected=true`，宿主权限状态为 `client_only`。六轮最终连接均释放，SSE 无 `chat.error`。原生组另有一次独立会话标题生成请求被上游拒绝，主会话仍完成。

原生挂载的 SSE 成功完成 37 次 query、4 次 callers、15 次 node、20 次 explore、5 次 files。首次额外请求为 `rustcodegraph_status`（`call_675162eae45b4530a1434c4f`）。三工具挂载的 SSE 成功完成 14 次 query、1 次 callers、14 次 node。唯一报告失败的调用为 `call_95fbd6a311644cf4b191d824`，参数为 `{"file":"app/router.tsx"}`，原服务返回文件不存在。部分并发调用在 SSE 只有开始事件。合成批次的原生探针复现这一输出特征，模型续轮仍收到每个工具 ID 的对应结果。

两组的 query 命中 `createRueSiteApp`（`app/app.tsx:32`），callers 返回 `mountRueSiteApp`（`app/app.tsx:39`），node 回读站点入口、`useApp`（`packages/runtime/src/hooks/useApp.ts:4`）及 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`）。模型答复按这些结果追踪主链路，解释浏览器与 SSR 守卫、缺少 `#app` 时抛错、插件安装、`/rust-layers` 绕过布局，并给出两项快照回归预期。三工具答复未确认独立 SSR 入口，对路由默认历史值的推断缺少工具回读。宿主核对真实工厂定义在 `app/router/index.ts:787`，原索引的同名 query 返回空，按正确文件和行号运行 node 可读到该定义。

受管实现、审批与撤除通过聚焦验证，严格真实验收待回归。余项为指定模型使用有效路径完成调查并产出完整答复。两套私有网关、监听端口、适配器及匹配私有副本的 stdio 子进程均已退出。临时挂载、凭据和数据副本已清理。原始证据仅保留于本机权限 0700 的目录。

## 集成回归与长线程修复

使用 Pi 验收中的同一共享构建，SHA-256 为 `7f8fb6ab97e9e6b00e6694ce48376e2d50a3ce4b2bff3fff93004b98db824916`。同版复用 `python3 scripts/cargo_slot.py cargo test -j 1 -p session-engine --lib acp_mcp_mount_carries_resolved_tool_policy` 的 1 项通过结果，以及之前通过的 Amp 定向审批测试。该测试覆盖同一受管审批入口，后续只改动 Amp 的本地 HTTP 请求解码和辅助标题声明。复用 Goose 的临时 stdio 转发器，仅公布三种真实工具，补齐已有行号分页参数和真实入口元数据，提示 `projectPath` 可省略。保持原提示、模型、权限、参数和真实返回。

三次未限速挂载均在约 55 至 57 秒收到 `chat.error`。原生 Worker 日志确认指定模型中继返回 HTTP 429。转发记录确认其中两轮的 47 次和 52 次图谱调用均成功。降低私有转发器的结果发送频率后，v2 三轮均收到 `chat.done`。对照会话 `ses_1f07608f90016suUoETKA6yxTv` 和 `ses_1f07a578a001R0iy7ZH4o5Z0Uk` 各只回复 `MCP_NOT_AVAILABLE`，零工具事件。挂载会话 `ses_1f07619ec00172p6ARo0j0E3Zy` 用时 277.760 秒，原始转发记录有 43 次 query、6 次 callers、37 次 node，全部成功。

该轮原生日志仍有线程持久化 HTTP 500。固定版 ThreadWorker 会在内部请求超过 25,600 字节时使用 gzip，受管桥原先直接把压缩体解析为 JSON。v3 在既有认证后的内部控制路由接收原生 gzip，并保留 16 MiB 的请求及解压上限。会话挂载时保留非流式辅助推理的 `set_title` 声明，流式用户回合仍限定为已选 MCP。通过相同原生探针确认 32 KB 线程在 v2 无法保存，在 v3 完整写入，加载后可继续执行，撤除后无 MCP。Node 定向运行 5 项通过，覆盖压缩保存、超限拒绝、普通工具路由拒绝 gzip，以及辅助标题请求仅保留 `set_title`。
