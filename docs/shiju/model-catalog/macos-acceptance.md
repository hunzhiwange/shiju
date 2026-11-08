# macOS 模型目录链路验收

2026-11-08，macOS 27.0（26A428），arm64，识矩 0.6.0。批次 42 的桌面发现进度与取消由 `0aee99a3a` 交付。本项仅覆盖这台 macOS 设备。测试使用独立项目、用户目录、SQLite、授权策略、models.dev 缓存和本机 HTTP 上游。未使用生产账号、发布应用或访问真实用户配置。

## 目录、保存与执行

- 在浅色窗口打开「设置 → 应用列表 → Codex → 供应商 → 模型目录」。搜索「月之暗面」返回 Kimi 国内、海外和两个 Coding 部署；另一个中文查询显示无匹配。滚动前后标题、搜索和底部操作区的位置不变；目录控件沿用 4px 滚动条和 7px 内容间隔。
- 本机 HTTP 上游返回两个模型；窗口只勾选并保存了 `Vendor/Exact[1m]`。编辑后预览并应用补全，回读保留了模型原始大小写、命名空间、后缀和目标部署。缓存贡献了 64000 上下文、4096 输出上限、纯文本输入和两项单价，来源是 models.dev，时间为 `2024-10-27 03:33:20 UTC`。
- 深色窗口关闭后重开目录，再输入同端点并刷新，回读相同模型。中文搜索、动态发现状态和原始 ID 均可读。延迟发现进入运行态后取消，显示「发现已取消，已保存模型保持不变」。释放延迟响应后，网关目录回读逐字节不变，SHA-256 为 `a2a7c7f454f8c13031dbe401d6fdf32df1bac91b3668e3c4dacb962903b8deab`。
- 用同一条窗口保存记录绑定运行时供应商，生产网关的供应商列表返回 `Vendor/Exact[1m]`。聊天网关拒绝图片输入；随后带工具的请求也按保存声明被拒绝。当时的上游日志只有两次发现 GET，没有执行 POST。之后通过同一修订事务明确启用工具、设置中文别名和默认模型，文本请求收到「fixture ready」；上游只收到一次 POST，模型 ID 仍是 `Vendor/Exact[1m]`。停止并重开生产网关后，目录响应与重启前逐字节相同。
- 新增 SDK 验收从发现、选择、补空、提交一路验证到重开网关。图片被拒绝时执行上游请求计数为 0；同一部署支持的文本请求恰好发送 1 次，使用原始模型 ID。另三项验证中文搜索和端点隔离、价格不串用、过期修订和未授权主机不写入、取消不会改动目录。

## 正式 CLI

真实网关通过受信路径执行官方 [OpenCode 1.19.5](https://github.com/anomalyco/opencode/releases/tag/v1.19.5) 和 [Antigravity CLI](https://antigravity.google/docs/cli/reference/) 1.5.3 的 Apple Silicon 二进制。下载分别按官方资产 SHA-256 和发布清单 SHA-512 核对。两个任务均观测到 queued → discovering → completed。OpenCode 返回 9 个含供应商命名空间的 ID，如 `opencode/beam-free`；AGY 返回 14 个原生显示名称，如 `Gemini 3.1 Pro (High)`。两个响应都保留各自部署身份，凭据未进入响应，发现前后已保存目录相等。AGY 使用隔离的 Gemini API Key 模式和本机端点；Google 登录账号专属目录未实测。

## 来源核对

固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7`；逐来源符号、条目数量及前项验证见 [实施证据](implementation-evidence.md)。本轮经网关分页读取 14 个应用，合计 1441 条可见预设和 3993 个模型条目。AGY 仅有运行时目录。可见行数包含通用预设投影，隐藏项与同一行的重复模型不计入。

| 应用 | 预设 | 模型条目 |
| --- | ---: | ---: |
| Claude | 140 | 144 |
| Claude Desktop | 128 | 288 |
| Codex | 138 | 277 |
| Gemini | 52 | 51 |
| Grok Build | 55 | 54 |
| OpenCode | 110 | 338 |
| OpenClaw | 122 | 383 |
| Hermes | 121 | 400 |
| Mcode | 115 | 406 |
| Kimi | 115 | 410 |
| DSH | 115 | 410 |
| Pi | 115 | 416 |
| OMP | 115 | 416 |
| AGY | 0 | 0 |

本轮业务回归覆盖七个应用的预设元数据投影、Codex/OpenClaw/Hermes/OpenCode/Kimi/Pi 六类原生补空、能力三态与档位、手工/批量/默认/别名、修订冲突、进度与取消、越权及脱敏。resolver 回归覆盖缓存刷新及断网保留，价格只核对同部署来源与任务 17 的消费契约。

## 验证命令

Cargo 共享槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。格式化在源码快照中执行，仅回写本项文件。

| 命令 | 退出码 | 有效用例 |
| --- | ---: | ---: |
| `cargo test -p vw-agent catalog_api_ -- --nocapture` | 0 | 26 |
| `cargo test -p vw-gateway-client catalog_api_ -- --nocapture` | 0 | 6 |
| `cargo test -p vw-agent-apps catalog_ -- --nocapture` | 0 | 242 |
| `cargo test -p vw-provider-resolver catalog_ -- --nocapture` | 0 | 25 |
| `cargo test -p vw-agent catalog_execution_gate_ -- --nocapture` | 0 | 11 |
| `cargo test -p vw-agent catalog_resolver_projection_ -- --nocapture` | 0 | 6 |
| `cargo test -p vw-desktop catalog_desktop_ -- --nocapture` | 0 | 24 |
| `cargo fmt --all`（最终） | 0 | — |
| `cargo test -p vw-agent catalog_macos_acceptance_ -- --nocapture`（红灯） | 101 | 3 通过，1 失败 |
| 同命令（最终） | 0 | 4 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | — |
| `cargo run -p vw-desktop --bin shi-ju` | 143 | 当前源码构建并启动；仅向隔离验收进程发送 SIGTERM |

红灯用例 `catalog_macos_acceptance_persists_and_enforces_identity` 已通过保存、重开、resolver 和图片拒绝断言，随后在文本应到达上游的断言失败：单元测试默认使用脚本流，返回 `missing scripted LLM stream`。仅在隔离子进程中启用既有真实 HTTP 测试入口后，该用例捕获到 1 次真实环回请求，四项全部通过。更早的编译和夹具授权失败没有计作行为验收。链接时报告既有 macOS `__eh_frame` 大小警告。

真窗口链路由任务 42 构建产物验证；本项没有修改桌面或生产路径。当前源码重新构建并以同一隔离网关配置启动复验。退出后确认槽位锁已释放，保留编译缓存。
