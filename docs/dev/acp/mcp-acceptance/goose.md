# Goose 动态挂载 RustCodeGraph

2026-11-19，严格三轮验收通过。Goose 1.57.0 通过原生会话 stdio 加载限定目录，指定阿里模型仅使用三种目标工具，90 次调用全部成功。无挂载和撤除轮的可见答复均为精确标记，工具事件为零。

## 环境

- 私有回环网关复用 `shiju 0.6.0`，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。仅在私有实例启用 `acp-goose`，使用租户 `10001 / 100010005` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，来源为 `session_model`，请求 `full_access=true`。[复用请求](../ACP验收.md#可复用的验收请求)
- Darwin arm64 复用仓库受管 Goose 1.57.0，二进制 SHA-256 为 `6facb500ca46839afcbe45617ae1ecf261b28022ea45b20d63c86389f6d8162a`，与安装回执一致。启动器通过 `GOOSE_PATH_ROOT` 指向请求私有目录，清空 `GOOSE_ADDITIONAL_CONFIG_FILES`，模型凭据由会话中继持有。
- 收到的提示取自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 私有副本从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出，以固定源码对照结果。RustCodeGraph 1.2.10 索引 3478 个文件，`createRueSiteApp` 命中 `app/app.tsx:32`。挂载使用原 RustCodeGraph stdio 命令。

## 原生会话与工具声明

- 同一受审二进制的 `initialize` 回执为 ACP v1，`goose` 1.57.0，MCP 能力 `http=true`、`sse=false`。独立原生探针按空、stdio、空发送三次 `session/new`。三个新会话均返回 `skills`、`Extension Manager`、`developer`、`summon` 和 `tom`，仅第二轮增加成功加载的 `ruejs_graph_probe`。原生新会话解析标准 `mcpServers` 并合并扩展。[Goose 1.57.0 源码](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/acp/server.rs)
- 独立回环模型探针记录三轮声明的工具数为 10、18、10，分别正常返回 `end_turn`。只有挂载轮声明八个 `ruejs_graph_probe__rustcodegraph_*` 工具，包含 `query`、`callers`、`node`。各轮均保留原生工具，包括 `tree`。探针使用临时回环令牌和固定答复，真实推理以网关轮次为准。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 | 可见答复与工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efa9e99f001c1vyxhyHBDj5W0` | 200 / 0 / 11.157 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1efaa1615001WQ3Z85ZlwygwiE` | 200 / 28 / 300.005 秒，续传 200 / 0 / 57.529 秒 | `chat.done` ×1 | 返回 RueJS 调查，210 条工具事件 |
| 撤除 | `ses_1efaf8d63001u31kjbFNTH5dZr` | 200 / 0 / 9.510 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，零工具 |

- 三轮无 `chat.error` 或模型恢复事件。挂载轮按原请求 ID 与事件游标续传至同一会话，没有重新推理。无挂载与撤除的 SSE 文本含上游输出的 `<think>` 块，表中记可见答复。
- 挂载轮三个代表调用均有匹配的开始与完成事件。`call_c1660659bdaf407cadcdd2e0` 查询命中 `createRueSiteApp`（`app/app.tsx:32`），`call_2b735482417146278c04197b` 返回 `mountRueSiteApp`（同文件第 39 行），`call_17b1b203325341b5a70ebc0f` 和 `call_e25434eb583046dab3e7c0e9` 分别取回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）及 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`）。
- `query`、`callers`、`node` 分别成功完成 32、7、47 次。首个不符合合同的事件为额外 `rustcodegraph_status`（`call_323d082e57c44d53a98547d1`），之后有 `explore`、`files` 和 `callees`。两次 `node` 猜测 `app/router.ts`、`app/i18n.ts` 并返回文件不存在。本轮所有工具事件均来自挂载的 MCP。
- 答复准确追到应用工厂，列出 SSR 跳过挂载、无容器抛错、路由/i18n 的插件安装顺序和 `/rust-layers` 布局分支。两项验证分别检查构建期快照及浏览器加载和导航后的重复节点。构建期场景取决于模板是否为空。有关重复传播、节点恰好翻倍的断言缺少实测，应按实际页面验证。记录仅核对工具输出和源码。
- UI summary 回读三轮均绑定 `acp-goose / goose`、`full_access=true`。ACP options 运行中和终态回读完整模型 ID 和 `session_model`，终态连接均已释放。无挂载与挂载连接中读到 `client_only`。撤除轮最后一次在线采样返回 `failed`，提示权限同步未确认或连接已断开，先前的独立撤除会话读到 `client_only`。三份原生会话数据库均为 `approve` 模式，且扩展只写入挂载会话。没有验证后端原生免审批。

## 补充复现

首次私有网关未在受管安装目录找到 Goose，HTTP 200 的 SSE 以 `chat.error` 结束。将已核验的程序和构建回执复制到私有配置下的 `acp-runtime` 后，通过原启动校验。修正过程中取得以下真实挂载和撤除证据，随后用正确目录重跑完整三轮。

- 挂载会话 `ses_1efa5e656001XwwmwvsmcyY0Tp`，HTTP 200，curl 0，242.881 秒，收到一个 `chat.done`，无 `chat.error`。`query`、`callers`、`node` 分别完成 50、4、43 次。另有一次 `node` 读取不存在的 `app/router.tsx` 失败，也完成了两次原生 `tree` 调用及其他图谱工具调用。
- `call_bdfd3eeb47894d6784145462` 查询返回 `createRueSiteApp`（`app/app.tsx:32`），`call_1b3f0f174539472a836e50a8` 返回调用者 `mountRueSiteApp`（同文件第 39 行）。`node` 调用 `call_a516b360049348f88d9c757f`、`call_ea127fb44ae245669bb5f047`、`call_97e5759f87234f0890d56051` 分别读回站点入口、`useApp` 和运行时工厂。三种工具都具有对应的开始、完成和输出。
- 可见答复覆盖调用链、SSR、缺少 `#app` 时的异常、路由及 i18n 安装和 `/rust-layers` 绕过布局。它结合 `app/app.tsx:45` 和 `packages/runtime/src/compiler-runtime/block.ts:104` 推导快照 DOM 残留，并提出首屏重复节点计数和 SPA 导航后旧标记残留两项场景。主要路径与源码一致。两倍滚动高度、重复 ID 等取决于实际页面，仍需浏览器验证。答复把未读到的路由定义部分归因于索引缺失，实际 `app/router/index.ts:787` 存在定义，单次 `node` 输出截断也会限制可见范围。
- 撤除会话 `ses_1efa99cf1001cZJBF5kazAHHK8`，HTTP 200，curl 0，9.719 秒，一个 `chat.done`，零工具事件，可见答复为 `MCP_NOT_AVAILABLE`。

## 首次验收结论与清理

首个失败边界是模型选用额外工具及无效路径。原有 Goose 启动配置、宿主转发和原生会话解析足以支撑本次动态挂载。保留 Goose 原生扩展、审批以及现有模型绑定。复跑使用同一请求，需三种工具成功完成、无额外工具和无失败。未修改 Rust 或运行 Cargo。

私有网关已退出，端口关闭，未发现本次子进程残留。撤除临时挂载后，私有副本与固定提交的 4106 个普通文件对照无差异。私有租户配置和运行目录已清理，原网关的 Goose 预设仍停用。原始证据保存在本机 0700 目录。

## 共享构建复验

使用共享槽位 3 的 `shiju 0.6.0` 副本，SHA-256 为 `f46699dd46d8520d8a7cfc3324132ccd5a2e05857e89fde160052fdcf75add3c`。复用 Goose 1.57.0、固定 RueJS 提交、原请求和指定阿里模型。以同批次已验证的 stdio 转发器只公布 `query`、`callers`、`node`，补全 `node.offset`、`node.limit` 的元数据，并提供当前副本中的真实 TypeScript 入口名。工具参数和返回保持原样。实测原 stdio 目录 8 项、限定目录 3 项，两个目录返回相同的符号查询、调用者和分页源码。限定目录拒绝额外工具及越界路径。

| 工具目录 | 轮次 | 新会话 | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- | --- |
| TypeScript 入口 | 无挂载 | `ses_1f0299931001jCfAjLBg4mwPdl` | 200 / 0 / 8.236 | `chat.done`，精确标记，零工具 |
| TypeScript 入口 | 挂载 | `ses_1f029ba61001W1ro5jNPTBi6uG` | 200 / 0 / 166.877 | `chat.done`，只调用三种指定工具，65 次成功、2 次失败 |
| TypeScript 入口 | 撤除 | `ses_1f02c4727001TV1swP8VcIL3mg` | 200 / 0 / 9.682 | `chat.done`，零图谱工具，调用一次原生扩展搜索，标记附带额外文字 |

挂载轮的 `query` / `callers` / `node` 分别成功 23 / 5 / 37 次，命中 `app/app.tsx:32`、`app/app.tsx:39`、`useApp.ts:4` 和运行时工厂。两次失败读取分别猜测了 `app/index.html` 与 `dist_static/index.html`。真实源码入口为仓库根 `index.html`，静态输出尚未生成。答复覆盖主链及两项快照验证，具体 DOM 计数仍为推断。

第二组在工具元数据中加入实际根 HTML 入口与构建脚本文件名，并明确构建产物可能不存在。仍用 RustCodeGraph 1.2.10。

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f02c8756001SZEvTJuKqB83rE` | 200 / 0 / 8.697 | `chat.done`，零工具，标记附带额外文字 |
| 挂载 | `ses_1f02caaa5001vdwqxWdNTt1mzT` | 200 / 0 / 202.864 | `chat.done`，只调用三种指定工具，67 次成功、零失败 |
| 撤除 | `ses_1f02fc578001RL7wdwPC0g1W4M` | 200 / 0 / 9.032 | `chat.done`，精确标记，零工具 |

挂载轮的 `query`、`callers`、`node` 分别成功 24、8、35 次。`call_a734912738b34485a0c93563` 命中工厂，`call_89eb5b15b5d348eb85df8dc9` 返回挂载调用者，`call_44c1b4e847954ac4bae4f6a2` 和 `call_5534b06f663b41c1ba8f8bea` 读回 hook 及运行时工厂。`call_9d30a78aeb0647c5b8449537` 成功读到 `index.html:73` 的空容器和下一行的客户端入口。源码支持其主调用链、SSR、容器缺失、插件安装、布局分支与追加挂载的推断。两项场景使用有效的 `/design/button` 路由，分别检查预渲染页的重复节点和预填容器的标记残留，DOM 精确计数未经运行验证。

第三组使用本地既有 RustCodeGraph 1.2.11 发行构建（SHA-256 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`），以及第二组工具元数据。新版索引能直接命中 `createAppRouter`（`app/router/index.ts:787`）。协议探针确认新版仍公布三个原名工具。

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f030a7fe001GXGXs554OxSses` | 200 / 0 / 10.057 | `chat.done`，零工具，标记附带额外文字 |
| 挂载 | `ses_1f030d1b7001UdzBhCaC74q3yT` | 200 / 0 / 182.722 | `chat.done`，只调用三种指定工具，70 次成功、1 次失败 |
| 撤除 | `ses_1f0339cf9001Y3sPkJdmsqpmu4` | 200 / 0 / 8.351 | `chat.done`，精确标记，零工具 |

失败调用 `call_265f12f1ce914b30bb49dadd` 把未独立建立索引的 `packages/router` 设为 `projectPath`，RustCodeGraph 如实返回未初始化。三轮继续保持指定模型和访问配置，使用相同源码副本。对独立的 2.0.0 调试构建只检查了工具目录，私有全量建索引耗时较长后停止。该调试构建没有参与真实验收。

## 通过的三轮

复用第二组的 RustCodeGraph 1.2.10 和工具元数据，并注明 `projectPath` 可省略、子包没有独立索引。网关、Goose、索引源码提交、请求提示、模型、服务名和审批设置与上述共享构建复验一致。转发器仅在私有副本的挂载轮出现。

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f033d705001Uf0U1DBLoRHl2n` | 200 / 0 / 9.603 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f033fe9b001xqmgmlZ2PByDUB` | 200 / 0 / 256.260 | `chat.done`，38 次 query、8 次 callers、44 次 node 全部成功 |
| 撤除 | `ses_1f037e8ea001XSctcqgNZ4mlw2` | 200 / 0 / 8.583 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

- 三轮均独立新建，未触发对照重试。每轮恰有一个 `chat.done`，无 `chat.error` 或模型恢复。挂载轮 180 条工具事件对应 90 次调用，只有三种指定工具。
- `call_5308dd9e82a64288958bf6f3` 查询命中 `createRueSiteApp`（`app/app.tsx:32`）。`call_22cd13feaa26444490abee91` 返回 `mountRueSiteApp`（同文件第 39 行）。`call_c0ab31b3b1ca48f1bad609db`、`call_96c9ee3793f2409c9ee4d178`、`call_f63630ba791a4f3ea6b3ee8b` 分别读回客户端入口、`useApp.ts:4` 和 `packages/runtime/src/compiler-runtime/app.ts:57` 的工厂。上述调用均有同 ID 的开始、完成和真实输出。
- 答复与源码对应，涵盖浏览器与 SSR 分支、缺少 `#app` 时抛错、路由与 i18n 安装、`/rust-layers` 布局条件，并说明该路径当前未注册。移除清空后的快照残留为源码推断。两项验证分别覆盖实际注册的 `/` 预渲染页中重复节点，以及带旧标记的预填模板。浏览器计数和挂载失败的具体终态未经实际运行验证。
- 三轮 UI summary 都确认 `acp-goose / goose`、`full_access=true`。ACP options 均回读指定提供商、完整模型及 `session_model`，连接期间为 `client_only`，终态均已释放连接。三份原生会话数据库均为 `approve`，且只有挂载会话包含 `ruejs_graph_probe`。
- 本轮只提交验收记录，未改运行时代码，未运行 Cargo。四座私有网关与子进程均已退出，端口关闭，临时配置和凭据已清理。RueJS 副本的 4107 个文件与符号链接按内容验证一致，索引除外。现有用户网关的 Goose 预设仍停用。
