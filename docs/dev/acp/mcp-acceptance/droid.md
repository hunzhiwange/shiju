# Factory Droid 动态挂载 RustCodeGraph

2026-11-19，已阻塞。Factory Droid 1.9.0 原生 ACP 能在会话内发现并调用 RustCodeGraph。统一验收首先在无挂载轮失败，模型为确认工具缺失而调用了 Droid 内置工具搜索。挂载轮成功运行三个必需工具，也运行了额外工具并产生一次工具失败。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境

- Darwin arm64 使用受管 Factory Droid 1.9.0，制品 SHA-256 为 `b62ce8de0ed7ffaada1abb2a017fe182021003694e85fe7f3b1136ad43628a5d`，与受管清单一致。入口为 `exec --output-format acp`，保留 Airgap、私有 Factory 状态、回环中继和启动环境过滤。私有网关为已有 shiju 0.6.0，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`，健康检查 HTTP 200。仅在私有实例启用预设与模型，复用已授权的认证来源。[Factory Airgap](https://docs.factory.com/enterprise/airgapped-deployment)
- 三轮使用 `10001 / 100010005`、`acp-droid / droid`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 使用 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的导出快照，索引覆盖 3478 个文件。挂载采用 `ruejs_graph_probe` stdio，直接运行原 RustCodeGraph，并传入私有副本路径与 `--no-watch`。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 | 答复与工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efa2fd1f001uCPxWyN65d3XU5` | 200 / 0 / 16.823 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，图谱调用 0，`ToolSearch` ×1，返回 `Tool(s) not found` |
| 挂载 | `ses_1efa3412e001aaSVbsStmrNr4k` | 200 / 0 / 242.177 秒 | `chat.done` ×1 | 完成三部分调查，105 对工具事件，其中 104 项成功、1 项失败 |
| 撤除 | `ses_1efa6f5830019M8XPIZpIqAuSG` | 200 / 0 / 16.358 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，图谱调用 0，`ToolSearch` ×1，返回 `Tool(s) not found` |

三轮均无 `chat.error`。UI summary 回读预设、后端和 `full_access=true`。ACP options 在运行中和结束后都回读固定模型和 `session_model`，运行时报告 `connected=true`、`access_status.support=client_only`，终态连接关闭。撤除后，服务及其工具从新会话消失，仍有内置工具搜索事件。两个对照轮的搜索失败均在 SSE 中标为 `failed`。

## 真实工具输出

以下调用均有同一 ID 的开始和成功完成事件，SSE 标题绑定 `ruejs_graph_probe`。

| 工具 | 调用 ID | 输出节录 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_23b916bbd5c19457_0` | `createRueSiteApp` 命中 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_b6871d9359658e4f_0` | 调用方为 `mountRueSiteApp`，位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_9a4551d8d7d61289_1` | `packages/runtime/src/hooks/useApp.ts:4` 的 `useApp` 转发给 `_$createApp` |
| `rustcodegraph_query` | `call_36882c17af45220b_0` | `_$createApp` 命中 `packages/runtime/src/compiler-runtime/app.ts:57` |

挂载轮还调用了 `ToolSearch` 与图谱的 `status`、`files`、`explore`、`callees`、`impact`。调用 `call_2d86a46bfc4291a6_3` 读取不存在的 `packages/i18n/src/index.ts`，SSE 记为 `failed`。这些事件已使挂载轮不符合统一合同。

答复追踪了四段调用链。源码复核确认浏览器挂载及 SSR 跳过（`app/app.tsx:54`）、容器缺失抛错（`app/app.tsx:40–42`）、路由和 i18n 在根工厂前安装（`app/app.tsx:34`、`packages/runtime/src/compiler-runtime/app.ts:45–48`、`:77–79`）、`/rust-layers` 走无站点布局分支（`app/app.tsx:17–28`）。答复也指出顶层调用和同名 `.use` 的图谱差异。它分析了删除清空逻辑可能保留旧快照并追加新树，给出了布局节点计数、路由和语言切换两项验证场景。具体重复 ID、译文和整页跳转属于推断，未运行浏览器实验。答复中 `/jsx/basic-elements` 的路由定位应为 `app/router/index.ts:46`。

## 原生边界与权限

- 私有回环模型探针确认 `initialize` 为 ACP v1，代理身份 `@factory/cli` 1.9.0，声明 `loadSession`、`resume`、HTTP 和 SSE。分别向原生 `session/new` 提交空列表、真实 stdio 定义及 HTTP/SSE 合成定义，四轮均建会话，固定短提示均为 `end_turn`。初始模型工具列表均含 `ToolSearch`，图谱工具延迟发现。真实挂载轮经搜索成功调用图谱，说明 `FACTORY_DISABLE_DYNAMIC_CONFIG=true` 与 ACP 会话 MCP 可以同时启用。统一三轮采用原生 stdio。
- 同版 `--only-tools` 指定三个会话 MCP 选择器时，进程在 ACP 握手前以 `Unknown tool identifier(s)` 退出，空列表与拟挂载两种设置均复现。独立使用 `--remove-tools ToolSearch` 后，原生模型请求仍声明 `ToolSearch`。两个选项合用则被 CLI 判为无效。未将这些探针参数加入受管入口。[Factory CLI 参数](https://docs.factory.com/droid-cli/cli-reference#droid-cli-flags)
- 另发独立权限探针，将 `full_access` 设为 `false`。HTTP 200，curl 退出 0，5.929 秒，SSE 返回 `chat.error`，原始消息为 `droid approval mode cannot constrain native tools; a current full access grant is required`。未出现 `chat.done` 或工具执行。保留该门禁。

## 复跑条件与清理

原生挂载及撤除已经生效。严格验收仍需能在会话级限定工具集合、同时保留 Droid 权限边界的受审入口，再使用固定提示和模型跑三轮。当前生产驱动和公共 ACP 转发未改。未触及公共转发测试所覆盖的故障，未运行 Cargo、桌面构建或浏览器测试。

私有网关退出 0，端口关闭，未发现本次工具或代理子进程残留。撤除配置后与导出快照逐项比较，4109 项内容一致，额外文件仅为私有图谱索引。私有源码、运行目录及授权副本已清理。现有网关的 Droid 预设仍为停用。原始 SSE、回读和诊断证据保存在本机 0700 目录。

## 共享新版网关复验

复用共享槽位 3 的 shiju 0.6.0 私有副本，SHA-256 为 `06086ba0fc9580bd82fdcf1327696acb6ee648727ed2b054e1ee32d60d8f37d5`。同一受管 Droid 1.9.0、模型、提示、租户、权限、RueJS 提交及 stdio 传输重新执行三轮。

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 | 答复与工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f019d19a00150aCqqdKGAPsCl` | 200 / 0 / 14.164 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，工具 0 |
| 挂载 | `ses_1f01a0b09001FxEvEQSXKR7rFu` | 200 / 0 / 237.961 秒 | `chat.done` ×1 | 三部分调查，102 对工具事件，101 项成功，1 项失败 |
| 撤除 | `ses_1f01dae7e001NT9sqKLTM7wCuM` | 200 / 0 / 19.523 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，图谱工具 0，`ToolSearch` 失败 1 次 |

三轮均无 `chat.error`，回读确认 `acp-droid / droid`、`full_access=true`、`session_model` 和完整模型 ID。运行中权限为 `client_only`，结束后连接关闭。挂载轮首个工具为内置 `ToolSearch`，之后还执行了 `status`、`files`、`explore`。调用 `call_6dea39fa25f9de37_2` 读取不存在的 `app/router.ts` 并失败，随后读到实际入口 `app/router/index.ts`。撤除轮搜索返回 `Tool(s) not found`。

挂载轮的 `call_5d05ea71c998762b_0` 查询得到 `createRueSiteApp`（`app/app.tsx:32`），`call_e074a195617db8d0_0` 得到调用者 `mountRueSiteApp`（`app/app.tsx:39`），`call_42e43f7f3a5ca362_0` 读取到 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）调用 `_$createApp`。答复覆盖四段调用链、SSR、容器缺失、插件顺序和布局分支，还指出 `createAppRouter` 定义在 `app/router/index.ts:787`，索引按符号查询漏报。答复提出浏览器初次挂载和客户端导航两项回归，未运行这两个场景。答复声称只用了图谱 MCP，与 SSE 所示的内置搜索不符。

### 替代入口复验

- 校验了 Factory 公开 Darwin arm64 独立发行版 1.13.0，官方 SHA-256 为 `ae552d8cdb515728f899558747207b950f4332762ea3495894d70cc680ba611d`。在私有环境中保留 Airgap 和回环模型，分别发送空列表、stdio、HTTP 与 SSE 的 `session/new`。均成功建会话并取得 `end_turn`，第一份工具目录始终有 `ToolSearch`，图谱定义仍未直接发布。该版本只用于诊断，受管清单仍固定 1.9.0。[Factory 发行说明](https://docs.factory.com/changelog/release-notes)
- 1.9.0 与 1.13.0 在 ACP 模式用 `--only-tools Read` 均仍向模型声明完整内置列表。1.13.0 也在握手前拒绝会话 MCP 选择器，且 `--remove-tools ToolSearch` 未移除工具。两版均接受带工具选择字段的原生 `session/new`，字段置于顶层或 `_meta` 时模型侧列表均无变化。Factory SDK 的工具限制属于 Droid 自有会话接口，当前 ACP 入口的建会话只转发 cwd 和 MCP 定义。[Factory SDK](https://docs.factory.com/sdk/typescript)
- 固定版回环诊断向原生 Droid 返回已挂载且尚未声明的图谱查询调用。批准探针收到 `session/request_permission`，以 `allow_once` 批准后，真实图谱输出进入 ACP 完成事件及下一次模型请求，终态为 `end_turn`。拒绝探针以 `reject_once` 回应，调用失败、无图谱输出，终态为 `cancelled`。可见原生执行器已有权限控制，剩余缺口在模型目录发布和工具选择。
- 新版网关的独立 `full_access=false` 探针再次返回 HTTP 200 与 `chat.error`，初始化拒绝仍要求当前完全访问授权。全程保留受管门禁。网关及探针退出，端口关闭，私有授权副本清理。导出快照比对 4109 项一致，现有网关 Droid 预设仍停用。未修改应用代码，未运行 Cargo。

仍为已阻塞。复跑需要受审的 ACP 入口能在请求绑定的目录中直接发布并限定会话工具，同时保留原生审批和撤除。若验收负责人决定接受 Droid 的原生搜索及其额外调用，应先更新工具与失败判据，再按更新后的合同验收。

## 固定版补丁原型

在 Darwin arm64 的 Factory Droid 1.9.0 私有二进制副本验证了直接发布 MCP 工具的补丁。产物保留原生 ACP 接口、模型回环中继和审批执行器。补丁脚本校验原版 SHA-256、Bun 模块记录和唯一源码锚点，在单个模块的原字节码空间放置替换源码，保留其他模块的缓存，再对独立产物签名。第一版仅改源码字节时仍观测到旧工具列表。让该模块改为从源码加载后，Droid 首个模型工具目录即包含当前会话的图谱工具。[Bun 独立模块图](https://github.com/oven-sh/bun/blob/bun-v1.4.3/src/standalone_graph/StandaloneModuleGraph.rs)

从仓库根目录运行以下命令，输出路径须位于现有私有目录，且文件尚不存在。

```bash
python3 patches/acp-model-source/droid/apply.py /path/to/original/droid /private/path/droid-patched
```

当前脚本再按实际发布的工具定义生成会话清单，提醒模型遵循用户指定的工具范围及缺失时的回退答复。原生回环模型四场景均符合预期。空列表和撤除后的新会话都没有图谱工具。挂载时 `query`、`callers`、`node` 首轮可用，经三次 `allow_once` 后均有真实完成及模型续轮输出，分别得到 `app/app.tsx:32`、`app/app.tsx:39` 与 `packages/runtime/src/hooks/useApp.ts:4`。独立会话使用 `reject_once` 时调用失败、无图谱输出，终态为 `cancelled`。非供应商制品和已有输出路径均被脚本拒绝，签名校验通过。

两次真实复验沿用固定请求和真实 RustCodeGraph。各自在私有网关副本中仅更新嵌入的 Droid 制品摘要，摘要验证仍执行。第一版只直接发布工具，不刷新原生发现提醒。无挂载轮 `ses_1f15b72f9001IP7I1W0QCDxUBZ` 在 14.870 秒返回 `MCP_NOT_AVAILABLE`，有一次失败的搜索。挂载轮 `ses_1f15bb051001hSyrgYu9SWLhcQ` 初次 HTTP 200、curl 28，300.005 秒后用同一会话与游标续传。续传 HTTP 200、curl 0、24.221 秒后收到 `chat.done`。三种必需工具成功，也有内置搜索、其他图谱工具与两次失败。撤除轮 `ses_1f160a5c4001ITSjqX7duLKbX0` 在 11.976 秒收到 `chat.done`，返回 `MCP_NOT_AVAILABLE`，工具调用为零。

带会话清单的最终原型摘要为 `0858a4f7411b0159780246598fc6170a17d695556e4da66ab939bff82b58050b`。用于复验的私有网关摘要为 `2e0046204bf4e5430080117d61cbe0e331c79279c88a2460dfefaf91052affe1`。

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 与工具 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f15dbd38001M5baakwpuL8VgY` | 200 / 0 / 15.111 秒 | `chat.done` ×1，`MCP_NOT_AVAILABLE`，工具 0 |
| 挂载 | `ses_1f15df995001aEI0eHOzMzpdwh` | 200 / 28 / 300.005 秒，续传 200 / 0 / 28.909 秒 | `chat.done` ×1，184 项工具成功，1 项失败 |
| 撤除 | `ses_1f16301a6001QM5VK0oXLyUAND` | 200 / 0 / 10.565 秒 | `chat.done` ×1，`MCP_NOT_AVAILABLE`，工具 0 |

三轮均无 `chat.error`，回读绑定及 `full_access=true`，运行时权限为 `client_only`，终态连接关闭，模型为固定阿里模型及 `session_model`。挂载轮直接调用 `query` 和 `callers` 得到 `createRueSiteApp` 和 `mountRueSiteApp`，再以 `node` 取得 `useApp` 调用 `_$createApp` 的源码，调用 ID 分别为 `call_904352df0ec978b1_0`、`call_a0dc4c34e9e07a13_0`、`call_7cd743c6d9e3cb69_1`。答复按三部分追踪调用链、浏览器和 SSR、插件与布局分支，给出静态快照首屏及导航回归场景。两项浏览器场景仍属源码推断。挂载轮没有 `ToolSearch`，仍使用额外图谱工具，并因请求未声明的 `rustcodegraph_search` 失败（`call_5bf62f23b5af5fea_1`）。

直接声明与原生审批已验证。严格验收仍需受审入口支持在执行边界限定会话工具，或由验收负责人调整额外工具调用判据。原型未接入受管安装。两次网关均退出，端口已关闭，各自副本的 4109 项导出内容一致，临时授权与工作目录已清理，原版 Droid 摘要一致。未运行 Cargo。SSE 和原生探针证据保存在本机 0700 目录。

## 受管安装接入

Darwin arm64 的 Droid 安装入口已接入同一补丁。安装器先按原版摘要校验官方 1.9.0，再应用补丁、校验签名及补丁版摘要，随后由注册表发布到独立 `droid-1.9.0-model-source-v2` 目录。原版 v1 在 v2 缺失时仍可按原版摘要校验后选用。v2 已存在时，摘要校验失败会拒绝其受管启动。现有入口参数、模型中继和权限路径沿用。

在两个私有目录运行受管安装器，分别使用官方地址冷下载和同一官方制品的已校验本地副本，两次均退出 0。安装产物 `--version` 输出 `1.9.0`，签名检查通过，SHA-256 为 `0858a4f7411b0159780246598fc6170a17d695556e4da66ab939bff82b58050b`，与前次指定模型三轮的产物一致。对安装产物运行回环原生 ACP，挂载轮三次逐次批准均成功，续轮收到三项真实输出。拒绝轮无输出，终态 `cancelled`。无挂载与撤除均无图谱声明。未重新运行指定模型三轮。

注册表用 `python3 scripts/cargo_slot.py cargo test -j 1 --locked -p acp-agent-registry --lib droid_managed_patch_is_pinned_in_a_separate_install` 验证 v2 路径、原版与补丁摘要分离及篡改拒绝，槽位 10，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均指向工作区的 `target/rust-slot-10`，退出 0，1 项通过。

会话工具清单用于模型发现。独立原生白名单探针尝试将 ACP 服务级策略传到 Droid 的 MCP 工具配置，模型仍收到八个图谱工具。未把该实验接入受管入口。严格验收仍阻塞在工具执行限制。

## 再次真实模型复验

复用受管安装冷下载得到且已验签的 v2 Droid，继续使用固定提示、租户、模型、`session_model`、`full_access=true` 和同一 RueJS 导出快照。共享构建尚未产出包含 v2 清单的网关，本轮用原槽位 3 网关的私有兼容副本，制品校验仅替换为同一 v2 摘要，兼容目录指向私有 v2。网关原件摘要 `25988bf02e238fe5450ca7c941015eaed1c067f5c64123c85aeacf1e2454734a`，私有副本摘要 `ca7b0542f0c414b12e8e1c1af898bc9e046a30f0e5673f5f738391c44a806f2e`。本轮验证运行行为。新源码构建的 v2 注册表路径仍由前述定向测试覆盖。

旧请求的 `acp_agent` 在此网关返回 HTTP 422，错误为 `unknown field acp_agent`，无会话也未发起推理。三轮复跑仅移除该字段，保留 `agent=acp-droid`，由预设解析出 `droid`。提示校验摘要保持一致。

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 与工具 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f17e268b001uhCCvD9aMp6HOz` | 200 / 0 / 12.342 秒 | `chat.done` ×1，`MCP_NOT_AVAILABLE`，工具 0 |
| 挂载 | `ses_1f17e579b0012pekzZ2blvLIku` | 200 / 0 / 222.486 秒 | `chat.done` ×1，106 项工具成功，4 项失败 |
| 撤除 | `ses_1f181be3d001RYffCwzSm73kZg` | 200 / 0 / 11.843 秒 | `chat.done` ×1，`MCP_NOT_AVAILABLE`，工具 0 |

三轮均无 `chat.error`，绑定、模型及 `full_access=true` 回读一致，结束后连接关闭。对照轮运行中权限为 `client_only`。挂载轮采到一次 `access_status.support=failed`，说明为权限同步尚未确认或会话已断开，该次回读未确认权限同步。网关日志另记录了后台会话标题模型 HTTP 403，三轮指定模型的会话均正常结束。

三个目标工具成功输出 `createRueSiteApp`（`app/app.tsx:32`，`call_aa1df6d662f8664a_0`）、调用者 `mountRueSiteApp`（`app/app.tsx:39`，`call_c288b2c193ad7ccd_0`），以及 `useApp` 调用 `_$createApp` 的源码（`packages/runtime/src/hooks/useApp.ts:4`，`call_a616800a8a692c16_0`）。答复覆盖固定提示的调用链、分支、路由和 i18n、布局绕过及两项有预期结果的验证。快照影响仍为源码推断。

挂载轮没有调用 `ToolSearch`，额外运行了 `status`、`explore` 与 `files`。四次 `node` 路径失败分别为 `app/router.tsx`、`app/i18n.ts`、`packages/router/dist/router.esm-bundler.js`、`packages/i18n/dist/i18n.esm-bundler.js`。严格验收仍阻塞。私有网关退出，端口关闭，临时授权与工作目录清理，4109 项导出内容一致。未修改运行时代码或运行 Cargo。
