# 任务 11 适配 amp-acp

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 amp-acp 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/amp_acp.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/amp_acp.rs`、`patches/acp-model-source/amp-acp`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_amp_acp_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/amp-acp.md`、`docs/dev/acp/skill-coexistence/amp-acp.md`。

## 上下文

审计基线版本为 0.12.0 / Amp 0.0.1778732631-gc5875c。MCP 现状｜v1 拒绝非空 new/load；上游 Worker 可读静态 MCP，原版适配器闭环未知。模型来源现状｜所有进程入口核验，须有私有绑定；通用，其他平台受管安装报错。原生工具现状｜普通保留四项宿主路由及标题；选中服务即裁剪官方流式目录并拒绝执行，标题另有分支；无逐服务名单投影。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 检查 native runner 缺失时的明确错误。挂载后原生工具裁剪需要改为只在明确策略要求时发生，缺少逐调用审批则保持拒绝。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_amp_acp_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_amp_acp_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

提交 `c782edf8d` 新增 Linux 固定制品、受管补丁模式和网关验证源码。

| 目标 | 固定运行制品 | 本轮证据 |
| --- | --- | --- |
| GNU x64 | `amp-acp@0.12.0` 的 `dist/index.js` 和 `@sourcegraph/amp@0.0.1778732631-gc5875c` 的 `dist/main.js`，由目标 Node 解释执行 | 两个归档 SHA-512 SRI 匹配，四个补丁产物 SHA-256 及语法通过。linux/amd64 未运行。 |
| GNU ARM64 | 同两份跨架构 JavaScript 制品，由目标 Node 解释执行 | 同一份归档与四文件摘要适用。linux/arm64 未运行。 |

- [Linux 制品清单](../../patches/acp-model-source/amp-acp/linux-runtime.json) 包含固定归档、两个 GNU 目标和安装后摘要。同版本上游 `@ampcode/cli-linux-x64` 与 `@ampcode/cli-linux-arm64` 的 npm 元数据分别声明 Linux/x64 与 Linux/arm64。原生二进制缺少可复用的模型和审批补丁入口，因此复用已受管的 JavaScript ThreadWorker。另核对了 Worker 可选加载的 `@napi-rs/keyring@1.1.10` 两份 GNU 制品，ELF machine 为 62 与 183。Linux 补丁固定为私有文件存储，阻止访问 OS keyring 与钥匙迁移。
- 任务 48 使用稳定 key `amp-acp`。在 `acp_agent_registry::managed_runtime::{installed_directory, install, verify, reviewed_agent_command, verify_reviewed_agent_executable}` 仅为上述 GNU 目标接入 `amp-acp-0.12.0-model-source-linux-v6`。安装使用清单中的两份固定归档，按 SRI 校验后在私有暂存目录提取两个包的 `package.json` 和固定入口。候选入口内嵌 SDK，无需触发 SDK 的 `@ampcode/cli@latest` 依赖和安装脚本。需要目标架构 Node ≥20（历史受审版 24.22.3）、Python 3 和 `/bin/sh`。执行 `python3 amp-acp/apply.py <stage>/node_modules/amp-acp --linux`。创建相对 `.bin/amp-acp` 链接，发布前核验两个包身份、四个规范文件路径和摘要、launcher 执行位。发现、显式覆写和启动共用验证。拒绝重定向、缺失、摘要不匹配及 Linux 受管路径回退 v5。入口为固定 `dist/index.js` 或解析到它的固定 bin 链接，供应商参数为空。控制桥预检 Worker、launcher、Node 和 shell。
- 复用 `model_source::prepare_profile`、`drivers::amp_acp::configure`、驱动的原生 Messages 协议判定和 `acp-client::spawn_child_at` 的审核与私有绑定。受管入口固定 `/v1/messages`，可复用宿主的 Messages 转换。普通 `cli_local` 的发现、审核和启动路径沿用现状，Linux 原生账号成功证据仍缺。Apple Silicon 默认补丁的四个产物与 v5 摘要逐一匹配。
- 技能使用 `skill_mounts::amp_acp::configure`。任务 48 在审核、平台及权限均满足时接入 `skill_mounts/mod.rs::validate` 和 `request.rs::do_stream_request_acp`。复用宿主技能视图，空数组代表工具已启用且无选择，`null` 对应禁用或拒绝策略。Linux 候选在禁用时裁剪流式目录并拒绝三个工具控制路由。挂载本身保留四个宿主原生工具，执行仍逐次请求 ACP 审批。
- 任务 48 在 `acceptance_support/mod.rs` 注册 `platform_amp_acp_tests`，条件为 `test`、Linux GNU 及 x86_64 或 aarch64。测试 `acp_platform_amp_acp_contract` 调用生产网关和真实适配器，断言固定制品、Messages 协议、模型读回、四项原生工具、外挂工具和两类技能来源。合成模型检查随机附件与服务结果按原调用 ID 续轮，四个独立网关会话断言服务新增审计 0/1/0/0。仍缺同会话取消、撤除及恢复的生产证据。旧 Linux 路由会在制品审核或技能门控停止，v3 的流式目录会在挂载时漏掉四项原生工具。
- 阻塞原因。受管 Worker 隔离了原生和环境 MCP。外挂 stdio 由 Worker 发起并在构建目录前连接服务。清理环境变量不提供文件、套接字、网络及子进程隔离，两种 GNU 目标仍缺等效隔离、逐服务工具名单和完整撤权证据。保持对外能力门控，任务 48 接通安全等效路径后再运行验证。
- 已静态核对归档 SRI、Linux 和 Darwin 补丁摘要、JavaScript 和 shell 语法、Python AST、单文件 Rust 格式及局部差异。未执行 Cargo、Docker、测试或安装。后续分别在两个 Linux GNU Docker 目标的非 root 环境提供 `SHIJU_ACP_AMP_GATEWAY_BIN` 和 `SHIJU_AMP_PROBE_RUNTIME`，执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_amp_acp_contract -- --include-ignored`。附加的 libtest 参数用于实际命中依赖网关和受审制品的测试，目前无红绿退出码或命中数。

## 续审

- 提交 `8d34366bb`。Linux 补丁同时核验补丁前适配器和 Worker 摘要。ACP 新建和恢复通过同一选择函数拒绝非空 MCP，空选择继续先撤销旧桥令牌、关闭 Worker，再清空配置。这给任务 48 提供可独立接线的无 MCP 受管候选，适用于两个 GNU 架构。按清单更新桥摘要 `c36162b2…`，模型和原生工具审批路由、技能视图环节沿用上面的接线。现有安装、能力发布及审批门控继续生效。
- 共享工作区复查到 `da4217f2ff`。MiMo 投影和 aionrs broker 都硬性门控 Darwin arm64，且实际创建 stdio 的位置依赖 Seatbelt。`open_isolated_stdio_peer` 接收已包装的 Command，负责免宿主回调的 MCP 连接，不提供子进程隔离。现有 Bubblewrap 包装替换原 Command、丢失 cwd 与显式环境、挂载宿主 `/tmp`，其 `--unshare-all` 也不能为整个 Amp Worker 保留宿主专用回环。AutoHand 的在途 HTTP 连接门控只校验会话选择及目标，需要既有宿主投影，本身不产生 Linux broker。直接开启供应商的 HTTP/SSE 或原生静态配置又会越过现有回环、逐调用审批和原生/外挂分源约束。这些路径均未提供可复用的 Amp Linux 等价执行边界。
- 两种架构的 `session_model` 无 MCP 子集有固定制品和适配器实现，仍待任务 48 接入生产安装、启动和验证。原生 MCP 和会话外挂 MCP 继续已阻塞。需要宿主提供 Linux 隔离的服务进程及工作区、环境、网络、清理合同，再绑定来源、工具名单、调用 ID 和单次审批，使撤销及恢复不复用旧连接或批准。普通 `cli_local` 原生账号仍缺具备等价审批的受审入口，原版 CLI 不能替换为受管旧版或直接绕过审核。
- 新增 `linux.test.mjs` 断言非空选择在配置或重载前被拒绝，并检查空选择的旧令牌撤销与 Worker 关闭。获准后设置 `SHIJU_AMP_LINUX_BRIDGE` 指向生成的 `dist/shiju-local.mjs`，运行 `node --test patches/acp-model-source/amp-acp/linux.test.mjs`。完整网关用例保留 MCP 预期，待受审传输接通后复跑。本轮仅重新核验上游归档 SRI、两套补丁产物 SHA-256、JavaScript 与 shell 语法、Python AST 和 Git 差异。未安装、未执行测试。

## Linux 无 MCP 制品组装

提交 `ac58dae7b`。

- 新增 Amp 专用 `patches/acp-model-source/amp-acp/install_linux.py`。任务 48 将它与同目录的 `apply.py`、`local.mjs` 和 `linux-runtime.json` 同置，将清单两份 HTTPS 归档下载到其私有 stage 后调用 `python3 install_linux.py <stage>/runtime <amp-acp.tgz> <amp-worker.tgz>`。脚本要求 GNU x64 或 GNU ARM64，校验两份归档和包版本，只写唯一的普通文件成员 `package.json` 及固定入口。然后应用 Linux 补丁，核对四个规范路径及摘要，设置可执行入口和相对 bin 链接。Rust 安装器继续负责 stage 清理、身份复核及原子发布。没有 npm 安装和供应商生命周期脚本。
- 已静态检查该入口的 Python AST。用其归档读取函数核验两份实际归档的 SRI、成员和版本，并在临时源码目录核对补丁生成的四个固定摘要和 JS、shell 语法。未调用制品组装入口、执行测试或安装。
- 复查至 `0f418397da`，新增 AutoHand 代码仅消费既有宿主 HTTP 投影，未新增 Linux 隔离 broker。两个 GNU 目标的无 MCP 制品组装已具备交付入口，生产接线仍归任务 48。任务完整状态继续已阻塞，原生和外挂 MCP 组合需要前述宿主隔离和审批合同，普通账号组合仍需受审的原生入口。
