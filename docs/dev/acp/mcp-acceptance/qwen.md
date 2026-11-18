# Qwen Code 动态挂载 RustCodeGraph

2026-11-19。Qwen Code 0.26.7 能通过原生 ACP 新会话挂载 RustCodeGraph、调用工具并将结果送回指定模型。严格验收阻塞在原生工具发现和模型工具选择。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境与边界

- Darwin arm64，独立回环网关复用 ShiJu 0.6.0，二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。受管包 `qwen-0.26.7-model-source-v1` 从既有安装复制，六个补丁后文件均匹配仓库合同摘要。独立禁网握手返回 ACP v1、`qwen-code 0.26.7`、`loadSession=true`，并声明支持 HTTP 与 SSE MCP。
- 真实轮次均用新会话、租户 `10001 / 100010005`、`acp-qwen / qwen`、`session_model`、`full_access=true` 及 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。统一提示来自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。
- 宿主源码通过 `session/new.params.mcpServers` 传入会话配置，固定 Qwen 入口将其注册为会话 MCP。挂载使用 `ruejs_graph_probe`、RustCodeGraph 1.2.10、原生 stdio 与 `--no-watch`。独立 MCP `tools/list` 返回 8 项，含 `query`、`callers`、`node`。真实网关未抓取完整 ACP 建会话请求体或上游模型请求体。
- RueJS 为 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。工作区副本索引 3687 个文件，包含用户原有改动，其中 `app/router/index.ts` 和 `app/entry-server.tsx` 已删除。干净提交副本索引 3478 个文件，包含 `app/router/index.ts:787` 的 `createAppRouter`。两个索引均命中 `app/app.tsx:32` 的 `createRueSiteApp`。

## 默认配置

| 轮次 | 会话 | HTTP、耗时 | 真实事件与答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efbfbd780014yq4y3ha20tWDy` | 200，7.44 秒 | 2 次原生 ToolSearch，均无匹配，零目标工具调用。答复为 `MCP_NOT_AVAILABLE`。 |
| 挂载 | `ses_1efbfdb68001cAAS9vgf7zTpO8` | 200，197.63 秒 | 2 次 ToolSearch，34 次 `query`、2 次 `callers`、37 次 `explore`、6 次 `files`、2 次 `callees`，零 `node`。 |
| 撤除 | `ses_1efc2e1130014CQW9uMonwoNAQ` | 200，8.41 秒 | 2 次 ToolSearch，零目标工具调用。答复含确认说明及 `MCP_NOT_AVAILABLE`。 |

三轮 curl 均退出 0，每轮恰有一条 `chat.done`，零 `chat.error`。表中答复已去除模型的 `<think>` 段。首个合同偏差是无挂载轮调用原生 ToolSearch。挂载轮 ToolSearch 的自然语言检索只回显五项，按提示里的短名称执行精确检索返回 `3 missing`。模型据此认为 `node` 不可用，随后改用 `explore` 读取源码。独立 MCP 枚举及下方直接声明复验确认 stdio 服务确实提供 `node`。

挂载轮 `call_7c9ef1b9610a43f282565745` 的 `rustcodegraph_query` 定位 `createRueSiteApp`（`app/app.tsx:32`），`call_7e2b84ddf196485e9c90157c` 的 `rustcodegraph_callers` 返回 `mountRueSiteApp`（`app/app.tsx:39`）。SSE 有对应的开始、参数、完成及真实结果。答复正确给出完整四段调用链、浏览器守卫、缺少 `#app` 时报错、插件在挂载时安装、`/rust-layers` 绕过布局、快照保留风险和两个建议场景。路由模块在该副本缺失，插件内部路径的部分描述来自其他源码推断。快照场景未实际运行。

## 直接声明复验

固定发行包默认通过 ToolSearch 与 ToolCall 按需提供 MCP 定义。两个私有 RueJS 副本分别添加 `.qwen/settings.json`，设置 `tools.toolSearch.enabled=false`，复用相同请求。该开关禁用搜索桥，保留 Qwen 的权限规则，原生注册表可向模型直接声明普通 MCP 工具。这两个副本用于定位工具发现边界。

在工作区副本挂载轮，同一调用 ID 的 SSE 记录了三项指定工具的开始、完成与真实输出：

| 工具 | 调用 ID | 输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_e20cb3d84c724e52bca44aa6` | `createRueSiteApp` 位于 `app/app.tsx:32`。 |
| `rustcodegraph_callers` | `call_f321915b35ef4c88b62bc5e7` | `mountRueSiteApp` 位于 `app/app.tsx:39`。 |
| `rustcodegraph_node` | `call_c7019d7dc76b4d5a9f01f85c` | 回读 `app/app.tsx`，含挂载守卫、容器判空、清空快照与根组件布局分支。 |
| `rustcodegraph_node` | `call_fa8a1775cd84412d9fd01ee7`、`call_374be558369b48b68c55c1e3` | `useApp.ts:4` 将根工厂传给 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`），后者挂载时安装插件。 |

| 副本与轮次 | 会话 | HTTP、耗时 | 真实事件与答复 |
| --- | --- | --- | --- |
| 工作区·无挂载 | `ses_1efc1f0f3001DGEp3TXQD7Szt7` | 200，6.42 秒 | 零工具事件，答复为 `MCP_NOT_AVAILABLE`。 |
| 工作区·挂载 | `ses_1efc20a8d001IjxHTUhWJj8d0H` | 200，181.45 秒 | 成功 `query` 23 次、`callers` 2 次、`node` 16 次；另有 31 次其他图谱调用、3 次 `node` 因缺失路由路径失败。 |
| 工作区·撤除 | `ses_1efc4d023001l4mWjZGJc8chUM` | 200，5.71 秒 | 零工具事件，答复为 `MCP_NOT_AVAILABLE`。 |
| 干净提交·无挂载 | `ses_1efc2d560001bykAtKQeJ4D3c7` | 200，6.19 秒 | 零工具事件，答复为 `MCP_NOT_AVAILABLE`。 |
| 干净提交·挂载 | `ses_1efc2ef06001uuljN9cv4RPbP4` | 200，251.90 秒 | 成功 `query` 51 次、`callers` 6 次、`node` 44 次；另有 13 次其他图谱调用、1 次失败的 `node` 和 1 次失败的 `query`。 |
| 干净提交·撤除 | `ses_1efc6c743001KuX0lmgks9SSGI` | 200，5.20 秒 | 零工具事件，答复为 `MCP_NOT_AVAILABLE`。 |

两组 curl 共六次退出 0，每轮恰有一条 `chat.done`，零 `chat.error`。两个挂载轮都调用了额外的 `rustcodegraph_status`。干净副本仍尝试读不存在的 `app/router.ts`，另有一次 `query` 失败。它随后成功读取 `app/router/index.ts`（`call_06711cbcf786474b97084b1b`）和 `app/entry-server.tsx`（`call_4a3faa4f72074b86b4734075`）。两份答复均给出四段调用链、关键分支、快照残留推断及两项带预期的回归场景。干净副本补充了 SSR 入口证据，但模型未读到 `createAppRouter` 函数体，默认 history 和路由表装配仍未从答复证实。

## 权限与结论

九轮 UI summary 均回读 `acp-qwen / qwen` 和 `full_access=true`。运行中的 ACP options 均回读连接有效、指定模型及 `access_status.support=native`。终态连接释放，指定模型绑定仍可重读。目标工具调用未出现交互审批事件。此结果仅覆盖本次只读查询和请求的完全访问级别。

原生挂载、调用及撤除已有证据。严格合同仍阻塞在默认发现路径及指定模型的额外调用和读取错误。Qwen 驱动和受管补丁保持原有行为，私有关闭 ToolSearch 设置仅用于排障。三个私有网关已退出，监听端口关闭，临时授权文件及私有挂载、诊断设置已清理。未修改原 RueJS 或用户网关配置。原始证据保留于本机私有目录。此次未修改公共转发边界，未运行 Cargo。
