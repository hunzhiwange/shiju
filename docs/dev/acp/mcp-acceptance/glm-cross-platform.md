# GLM ACP 六平台验收

验收版本为 `glm-acp-agent@1.17.1`，受管安装目录为 `glm-acp-agent-1.17.1-model-source-v2`。覆盖目标取自 [OSS 制品矩阵](../../../../scripts/prepare_oss_release.py)。同一 Linux 架构的 DEB 和 RPM 共用一行。

| 目标 | 当前结果 | 缺失证据 |
| --- | --- | --- |
| macOS arm64 (`aarch64-apple-darwin`) | 通过 | 已保存冷装、启动、三轮 SSE、拒绝及清理证据 |
| macOS x86_64 (`x86_64-apple-darwin`) | 待回归 | 缺少 x86_64 macOS 原生验证机 |
| Linux arm64 (`aarch64-unknown-linux-gnu`) | 待回归 | 缺少搭载当前 ShiJu CLI、RustCodeGraph 和已授权指定模型的目标验证环境 |
| Linux x86_64 (`x86_64-unknown-linux-gnu`) | 待回归 | 缺少 x86_64 Linux 原生验证机 |
| Windows arm64 (`aarch64-pc-windows-msvc`) | 待回归 | 缺少 arm64 Windows 原生验证机，待补 ACL、重解析点、npm shim 与端到端回执 |
| Windows x86_64 (`x86_64-pc-windows-msvc`) | 待回归 | 缺少 x86_64 Windows 原生验证机，待补 ACL、重解析点、npm shim 与端到端回执 |

## 固定输入

- 受管补丁 SHA-256 为 `e0a9e4a02b4e3e939dc39edb8133a6f6d30ff50365df124d91d22445d90f0fdc`。[受管补丁](../../../../patches/acp-model-source/glm-acp-agent/glm-acp-agent-1.17.1-managed.patch)
- 预设 `acp-glm-acp-agent`，后端 `glm-acp-agent`，租户 `10001 / 100010005`，请求模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，来源 `session_model`。三轮分别开始新会话，沿用计划中的新版提示和 `full_access=true`。拒绝场景单独设置 `full_access=false` 与 `options.acp_permission_mode="deny-all"`。[验收请求](../../../../.plans/2026-11-19-acp动态挂载mcp/plan.md#新版复用请求)
- 将 RueJS `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出到私有目录。RustCodeGraph 1.2.11 以 `init -i` 重建索引，`query createRueSiteApp --limit 3` 命中 `app/app.tsx:32`。挂载轮使用项目根的 `.vwacprc.json` 声明 `ruejs_graph_probe`，通过 stdio 执行 `rustcodegraph serve --mcp --path "$project" --no-watch`。工具策略允许 query、callers 和 node，并对同时出现在允许名单里的 status 设置显式拒绝。

## macOS arm64 实测

2026-11-19，Darwin arm64、macOS 27.0（26A428）、Node 24.22.3、npm 11.19.1。运行 `python3 scripts/cargo_slot.py cargo build -j 1 --locked -p vw-cli --bin shiju`，退出 0，耗时 17 分 55 秒。槽位 3 的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。从当前共享工作区构建，GLM 包含截至 `2bb92f0d1c` 的接线。已复制产物供私有网关使用，构建进程退出且锁已释放。原生 arm64 `shiju 0.6.0` SHA-256 为 `653d967c267fe9ee1cb68e27109b6bccbbff2584b679c5611ada530bf702e2cb`。本次未运行新的 Cargo 测试。

私有网关使用空安装目录和独立 npm 缓存，事先缓存了公开发行包。以 `shiju --config-dir "$probe/config" gateway --host 127.0.0.1 --port 0` 启动，健康检查与一键安装均为 HTTP 200。安装前 `session_model` 返回 `unsupported_cli`，安装后返回 `supported`。安装入口、执行器和 MCP 客户端的 SHA-256 分别为：

- `dist/index.js` · `d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98`
- `dist/tools/executor.js` · `c775d1418f84f2978ec421da50fa335f8334599afcbe47d8987882a4735e4bd6`
- `dist/tools/session-mcp-client.js` · `45bf201cd76da3418ee7557a7f49e11c3acc6d98ad3bfc7784c5eb88ef75d935`

原配置网关在冷装后连接失败。保留本次冷装产物和回执，重启私有网关并从之前的验收记录复制指定模型定义，经私有鉴权 API 写入本机已授权凭据。新版提示 SHA-256 为 `18806f1a927d3587ee5c685107c89b5dbaeea09614efe9a327e293b649967f38`。RustCodeGraph 1.2.11 入口 SHA-256 为 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`，索引 3478 个文件。挂载使用私有 stdio 记录器原样转发目录、请求和返回，服务名单由 GLM 的受管入口执行。

### 真实模型会话

| 场景 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 与实际执行 |
| --- | --- | --- | --- |
| 普通会话 | `ses_1f289e89f001aIKW0k3m8HhL28` | 200 / 0 / 8.476 | 内置 `read_file` 成功读取 `app/app.tsx`，调用 `call_cf8107b1b65d4882aa4a423e` |
| 无挂载 | `ses_1f28a0af1001851KwLTwlMiJnS` | 200 / 0 / 23.351 | 项目没有挂载声明，MCP 服务零启动、零执行和零返回。模型用内置只读工具寻找配置 |
| 挂载 | `ses_1f28a6707001xoQ0gvpg2HVg2g` | 200 / 0 / 25.348 | 3 次 query、1 次 callers、11 次 node 得到真实 MCP 响应。内置读取 `packages/rue/src/index.ts` 成功 |
| 撤除 | `ses_1f28acb3a001ivPWvo5OvAYi63` | 200 / 0 / 7.709 | 已移除项目声明，MCP 服务零启动、零执行和零返回。模型仍可使用内置只读工具 |
| 定向共存 | `ses_1f28aeeb1001Y91ppMpS25lbWU` | 200 / 0 / 7.314 | 同一会话先完成内置 `read_file`，再完成真实 `rustcodegraph_query`，答复引用两个结果 |

这些会话均有一次 `chat.done`、零 `chat.error`。三轮 UI summary 回读相同预设和后端，`full_access=true`。ACP options 回读指定模型和 `session_model`，挂载期间连接有效，三轮终态均断开。

挂载首个 query `call_a66a81200d244a718024943a` 定位 `createRueSiteApp`（`app/app.tsx:32`），callers `call_5d1ff75885ca4d7bb415d7e6` 返回 `mountRueSiteApp`（第 39 行），node `call_2119372faaeb4dcc83c33953` 读回 `packages/runtime/src/hooks/useApp.ts:4` 的工厂委托，SSE 摘要截断处以同参数的原始 MCP 回执核对。后续调用进一步定位 `compiler-runtime/app.ts:57` 的 `_$createApp` 与第 17 行的 `_$mountApp`。最终可见答复引用这些工具返回中的符号和行号，说明插件安装、根块工厂执行及挂载，构成结果进入模型续轮的证据。MCP 侧只收到三种获准工具的执行请求。

### 拒绝与清理

| 场景 | 新会话 | HTTP / curl 退出码 | SSE 与服务侧证据 |
| --- | --- | --- | --- |
| 宿主显式拒绝 | `ses_1f28aea44001vhcukbb3p1NVHD` | 200 / 0 | `chat.error` 为 `GLM session MCP is denied by the host permission policy`。服务侧有目录探查，`tools/call` 为零 |
| 检测篡改 | `ses_1f28b0cbb001N7ukwT09ENDNQT` | 200 / 0 | 私有 `executor.js` 被改动后，能力回退为 `unsupported_cli`，会话因受审入口验证失败终止 |
| 不安全状态目录 | `ses_1f28b1018001vGGDXT0AhPy1Bq` | 200 / 0 | 私有状态根从 0700 临时改为 0755，启动返回 `insecure GLM Agent state directory` |

负向会话均只返回一次 `chat.error`，零 `chat.done` 和零工具事件。测试结束恢复文件和目录权限，撤除私有挂载，删除私有鉴权副本。两次私有网关的监听端口均关闭，未发现本轮 GLM、RustCodeGraph 或转发进程残留。RueJS 副本 4109 个文件及符号链接逐项与归档一致，索引除外。安装回执、SSE、会话回读与 MCP 帧留在本机私有验收目录。

## Linux arm64 容器证据

Docker Desktop 的 arm64 Linux 容器使用 `node:22-bookworm`、Node 22.23.4、npm 10.9.9、Python 3.11.2。首次执行 `node --test patches/acp-model-source/glm-acp-agent/managed-coexistence.test.mjs` 退出 1，npm 安装达到夹具的 80 秒时限。另起同架构容器，从空运行目录用生产 `install.py` 冷装。npm 拉取固定包时曾出现 `ECONNRESET`，重试后安装退出 0。沿用这一容器的 npm 缓存再执行原命令，退出 0，1 项通过、0 项失败。

夹具通过受管入口启动实际 GLM ACP，使用本地模型与 MCP 替身，核对官方只读工具、会话级工具名单及拒绝优先、工具结果进入续轮，以及加载和分叉时按当前名单处理。两个容器均已退出并由 `--rm` 删除。Linux arm64 的 ShiJu 一键安装及指定真实模型的三轮 SSE 仍待验证。[回环夹具](../../../../patches/acp-model-source/glm-acp-agent/managed-coexistence.test.mjs)

## 已有定向证据

以下 macOS arm64 结果来自前置任务，本任务复用。安装器 `glm_managed_runtime_` 1 项、ACP 客户端同过滤 2 项、模型来源 `glm_model_source_capability_` 2 项、Unix 私有状态目录 `glm_state_directory_` 3 项，均为 0 失败。Windows ACL 和重解析点测试仍需 Windows 原生验证机。

## 原生环境复跑

1. 核对 OS、架构、Node/npm 版本与受审补丁摘要，准备空的私有配置、安装和状态目录。启动 `shiju --config-dir "$probe/config" gateway --host 127.0.0.1 --port 0`，从日志读取端口并请求 `GET /v1/health`。仅向私有实例配置已授权的指定模型。
2. 携带租户头调用 `POST /v1/desktop/acp-install/glm-acp-agent`，保留响应和安装后入口摘要。发起普通会话，要求内置只读工具实际读取 `app/app.tsx`，保存工具结果。
3. 将新版请求的目录头和 `options.cwd` 指向私有项目副本，向 `POST /v1/chat/stream` 发起无挂载、挂载、撤除三轮。逐轮保存 SSE、响应头、会话 UI summary 与 ACP options。挂载轮核对内置工具与 MCP 调用、实际 MCP 返回和依赖该返回的后续模型输出。
4. 重新写入挂载并显式拒绝。绑定同一模型，检查拒绝回执和 MCP 侧零执行。核对篡改入口、私有状态权限及重解析点等原生防护，保存回执。结束后清除私有凭据，核对本轮网关、GLM、RustCodeGraph 与监听端口。
