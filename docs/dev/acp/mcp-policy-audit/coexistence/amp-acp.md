# Amp 受管原生工具与 MCP 共存

2026-11-19 在 macOS 27.0 arm64（26A428）上，受管 `amp-acp@0.12.0` 和固定 Worker `@sourcegraph/amp@0.0.1778732631-gc5875c` 通过本地回环验收。同一 ACP 会话、同一次提示中，宿主实际读取了夹具文件，RustCodeGraph MCP 实际查询了夹具索引。两条成功结果均以原调用 ID 进入模型续轮。

## 修复与入口

- 基线 v3 的回归测试复现两个断点。已挂载会话的流式工具目录只含所选 MCP，且宿主路由在 ACP 审批前以 HTTP 500 拒绝原生 `Read`。修复沿用原有四项宿主路由 `Read`、`create_file`、`edit_file` 和 `Bash`，以及每次调用的 `allow_once` / `reject_once`。非流式标题分支、MCP 目录校验及 Worker 分派沿用原路径。
- 新合约路径为 `amp-acp-0.12.0-model-source-v4`。探针在私有目录内复制了受管 v3 安装，其中四个文件摘要均匹配旧合约，按现有补丁安装钩子的复制方式更新 `dist/shiju-local.mjs`。启动前核对 v4 合约的两个包版本、四个文件摘要及 `.bin/amp-acp` 的解析目标。新控制桥 SHA-256 为 `8abbb35b9ef50e087f328c1aca06aec25e4878bec902e94213857759eefb8845`。
- 实际入口为该副本的 `node_modules/.bin/amp-acp`，使用 Node 24.22.3 和空参数，ACP v1 握手返回空认证方法。启动前检查了入口到 `dist/amp-cli`、固定 Worker 及 RustCodeGraph stdio 子进程的链路。外层 macOS `sandbox-exec` 默认拒绝，子进程继承同一限制。策略只允许回环 IP 网络和明确列出的可执行文件，禁止 Mach 服务访问、Unix 套接字和钥匙串文件访问。模型通过回环 HTTP 接收合成请求，MCP 通过 stdio 读取私有工作区的索引。

## 同会话轨迹

首个模型流同时声明四项宿主工具和来自所选服务的八项 RustCodeGraph 工具。探针按顺序发出以下四次调用，每次续轮都检查结果 ID。

| 次序 | 工具与调用 ID | 执行方与续轮结果 |
| --- | --- | --- |
| 1 | `Read` · `toolu_amp_read_ok` | 宿主经 ACP `fs/read_text_file` 读取 `native.txt`。返回值含本轮随机文件标记，`is_error=false`。 |
| 2 | `mcp__graph_fixture__rustcodegraph_query` · `toolu_amp_mcp_ok` | Worker 调用现有 RustCodeGraph stdio 服务。返回 `function  ampCoexistenceFixture  probe.js:1`，`is_error=false`。 |
| 3 | `Read` · `toolu_amp_read_deny` | 宿主返回 `reject_once`，无第二次文件读取。续轮收到同 ID 的错误结果。 |
| 4 | `mcp__graph_fixture__rustcodegraph_query` · `toolu_amp_mcp_deny` | 宿主返回 `reject_once`，Worker 在 MCP 分派前结束该调用。续轮收到同 ID 的错误结果。 |

五次流式请求依次回读零至四条结果。ACP 提示结束为 `end_turn`，探针和受管入口均退出 0。

## 检查范围

- 在沙箱内运行 `node --test patches/acp-model-source/amp-acp/local.test.mjs` 的私有副本。修复前退出 1，5 项通过、2 项预期失败。修复后退出 0，7 项通过，包含挂载目录、独立审批、未选工具拒绝和压缩线程上传。
- 本轮执行直接 ACP 回环与静态合约核对。真实模型、供应商原包、登录、网关冷装、Rust 测试、撤除、恢复及其他平台均未运行。
