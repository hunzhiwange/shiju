# Goose 动态挂载 RustCodeGraph

2026-11-19，严格验收已阻塞。Goose 原生 stdio 挂载、三种目标工具调用和新会话撤除均有真实证据。挂载轮超出指定工具范围，且存在文件查询失败。

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

## 结论与清理

首个失败边界是模型选用额外工具及无效路径。原有 Goose 启动配置、宿主转发和原生会话解析足以支撑本次动态挂载。保留 Goose 原生扩展、审批以及现有模型绑定。复跑使用同一请求，需三种工具成功完成、无额外工具和无失败。未修改 Rust 或运行 Cargo。

私有网关已退出，端口关闭，未发现本次子进程残留。撤除临时挂载后，私有副本与固定提交的 4106 个普通文件对照无差异。私有租户配置和运行目录已清理，原网关的 Goose 预设仍停用。原始证据保存在本机 0700 目录。
