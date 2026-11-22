# 任务 35 适配 omp

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 omp 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/omp.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/omp.rs`、`patches/acp-model-source/omp`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_omp_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/omp.md`、`docs/dev/acp/skill-coexistence/omp.md`。

## 上下文

审计基线版本为 18.22.0。MCP 现状｜受管沙箱 stdio EPERM；原生同端口 HTTP 可发现，早期无 ACP 逐次授权。模型来源现状｜原生 models.yml/set_config_option 选模已证；受审私有模型；通用。原生工具现状｜普通和挂载同样仅 read/write/edit，禁扩展、技能及网络；挂载另加 MCP，文件写保留宿主边界。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 核对受限原生技能的上游约束。外挂技能只能在安全挂载路径恢复，保持文件写入策略。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_omp_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_omp_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 固定上游制品 | 静态架构证据 | 受管状态 |
| --- | --- | --- | --- |
| x86_64 | 18.22.0 `omp-linux-x64`，292006064 字节 | ELF64 machine 62，`/lib64/ld-linux-x86-64.so.2` | 启动沙箱、stdio 隔离和原生 MCP 审批阻塞 |
| aarch64 | 18.22.0 `omp-linux-arm64`，247385080 字节 | ELF64 machine 183，`/lib/ld-linux-aarch64.so.1` | 启动沙箱、stdio 隔离和原生 MCP 审批阻塞 |

- [Linux 制品清单](../../patches/acp-model-source/omp/linux.json) 固定每个目标的 HTTPS URL、大小与 SHA-256。两份完整上游制品流式 SHA-256 与 GitHub Release API 摘要及同版 SHA256SUMS 一致，两个 ELF 的头和解释器来自上游字节范围读取。上游独立发行物使用对应 GNU loader 和运行库，无需 npm、Bun 或 npm lifecycle。上游 release smoke 分别使用 Ubuntu 22.04 x64 和 Ubuntu 24.04 ARM64，尚未核对最低 glibc 版本。普通 `cli_local` 仍使用内置注册的本地 `omp acp`，回退为 `npx -y @oh-my-pi/pi-coding-agent@18.22.0 acp`。Apple Silicon 沿用现有 v1 目录、原发行摘要、Seatbelt 与技能配置。[发布与制品](https://github.com/can1357/oh-my-pi/releases/tag/v18.22.0) · [上游构建及冒烟](https://github.com/can1357/oh-my-pi/blob/v18.22.0/.github/workflows/ci.yml)
- 任务 48 的制品接线点为 `managed_runtime::{installed_directory,install_builtin_agent,reviewed_agent_command,verify_reviewed_agent_executable}` 和 `managed_runtime/install.py`。在 Linux GNU x86_64/aarch64 下声明 `#[path = "managed_runtime/omp.rs"] mod omp;`。安装器将 `omp::MANIFEST` 写入 staging assets，按架构选 HTTPS URL 和校验值，在私有 staging 用现有 curl 下载方式获取独立文件，设置 0700，发布前调用 `omp::verify(directory)`。发现和显式命令校验复用该函数，显式路径须与返回的正则化绝对入口一致，不能走 Darwin 通用 native 校验。建议 Linux 独立目录 `omp-18.22.0-model-source-v1-linux-x64` / `omp-18.22.0-model-source-v1-linux-arm64`，入口为其中的 `omp`，固定参数 `["acp"]`。Python 3 仅供安装流程、受管桥和验收，curl 仅供下载安装。身份校验独立于能力启用。
- Linux `model_source::drivers::omp::configure` 在写私有文件和传递凭据前拒绝。现有 `drivers::capabilities`、`skill_mounts::validate` 和 `acp-client::spawn_child_at` 的 `managed_workspace_io` 分支继续守住门控。后续安全运行分支应复用私有 `models.yml`、`set_config_option` 和回环 Chat Completions，保留 `--tools=read,write,edit --no-extensions --no-lsp --offline`。本轮未找到可直接复用的等价 Linux 沙箱。仓库 Landlock 仅约束文件系统，Bubblewrap 的默认网络隔离阻断宿主回环。受管启动和 MCP 子进程分别需要可限定模型端口与禁止外网的隔离、只读工作区、私有状态可写、禁用额外进程执行及撤销后收尾。Linux 分支不得以直接启动 OMP 或开放宿主网络代替。
- 原生技能沿用 OMP 扫描器。项目 `skills.enabled=false` 会停用目录，原生 `.omp/skills` 与私有 `skills.customDirectories` 保持独立，正文、附件按需走 `skill://`。外挂仅由 `skill_mounts::omp::configure` 注入已验证的会话技能选集。Linux 必须对选集和解析后的目标只给读权限，保持原生文件写入经宿主 Workspace I/O。当前 `allow_selection` 只生成 Seatbelt 规则。[固定版技能文档](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.0/docs/skills.md) · [ACP 文件桥](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.0/packages/coding-agent/src/modes/acp/acp-client-bridge.ts)
- 外挂 MCP 可延续 `omp/launcher.py` 和 `omp/mcp_bridge.py` 的同端口 HTTP 转接与按提示轮撤销，先替换 OMP 启动器和 Cortex `Stdio.start` 的两处 Seatbelt 依赖。当前桥自建审批 ID，未证明 ACP 原调用 ID 能传到执行端。上游 ACP 建会话关闭静态 MCP，仅给 ACP 声明建立独立 manager。原生静态 MCP 仍需经可信配置与任务 4 的来源、参数、调用身份和撤销要求接入受管执行器。不能把普通原生 MCP 接通视为逐次批准或允许在受管分支直连。任务 49 将两架构的受管语义登记为阻塞。[固定版 ACP 会话与 MCP](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.0/packages/coding-agent/src/modes/acp/acp-agent.ts)
- 独立测试复用已注册的 `model_source/drivers/omp_tests.rs`，任务 48 无需另注册。测试使用真实网关生产入口和固定 OMP，外部模型及 MCP 为测试夹具。旧源码会在 Linux 制品/网关门控处失败。测试记录实际制品版本、入口、架构及鉴权 Chat Completions，检查工具列表、原生与选中技能正文和附件、批准后的实际调用与同 ID 续轮，以及拒绝和撤除的审计零新增。四轮用独立网关会话，静态原生 MCP、文件写批准及同会话取消/恢复待受管接线后补验。
- 后续获准后在两种非 root Linux GNU Docker 目标分别设置 `SHIJU_ACP_OMP_GATEWAY_BIN` 和 `ACP_OMP_REVIEWED_BIN`，执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_omp_contract -- --ignored`。原命令不带 `-- --ignored` 会跳过此真实网关夹具。预期每目标命中 1 项，接线前红灯，受管闭环完成后零失败。静态检查通过，包括限定 Git 差异、Rust 语法解析、新增 Rust 代码格式、Python AST 和清单字段。本轮未编译、未执行测试、未安装或运行上游程序，暂无运行退出码或实际命中数。

## 续查与补充交付

- 任务 48 已在 `managed_runtime::linux_cli` 接入两架构普通安装、解析和按原路径启动复核。实际目录为 `omp-18.22.0-linux-cli-x86_64` / `omp-18.22.0-linux-cli-aarch64`，覆盖上文候选目录。生产入口匹配 `omp` 及 `["acp"]`，文件校验走 `omp::verify`，普通会话的受限权限、外挂 MCP 和受管选模在各自宿主入口拒绝。探针已改用实际安装目录，保留 `ACP_OMP_REVIEWED_BIN` 覆盖。新增独立 `acp_platform_omp_contract_cli_local` 从隔离配置调用正式安装及发现接口，真实 ACP 握手后以保留的原命令再次启动，断言被替换的制品在启动前被拒绝。两架构此独立子路径可进入回归。
- 复核同一工作区新接入的 Gemini/Kilocode/Kimi 与 Cortex。Gemini 的调度器掌握真实调用 ID，在执行同一个 invocation 前消费授权。Kimi 的命令环境限制和 Kilocode 的目录校验无法约束 OMP 的原生进程或补齐其 MCP 调用身份。Cortex 的 Linux 制品补丁也未替换其 stdio Seatbelt。通用 Linux 私有状态路径校验只能验证目录，不能约束原生进程。当前 Bubblewrap 配置下私有网络隔离也隔开宿主模型端口，复用 OpenClaw 的自有网关模式仍需要 OMP 固定进程允许读取的会话私有传输和 Linux 子进程隔离，不能通过 `--share-net` 解决。
- 精确协议断点已定位。上游即时和延迟 `MCPTool.execute` 均握有 `toolCallId`，但 `callToolWithAuthRetry`、直接重试及 `client.callTool` 最终只向传输发送 `name` 和 `arguments`。ACP 的 `tool_execution_start` 平行通知确实保留 ID 与实参，但 HTTP 端收到的请求和它没有共同标识，同名同参并发及重试无法可靠区分。禁止按名字、实参或到达顺序猜配。两架构的原生及外挂 MCP 需要带调用身份的受审构建，或上游提供等价执行前审批钩子。可在 OMP 的即时、延迟及重试路径携带同源调用 ID，按预选原生/外挂路由消费一次证明，拒绝重放和撤销；构建后需分别固定两份受审产物及验证其 Linux 隔离。未经该协议与受管隔离输入，状态继续为已阻塞。任务 48 可直接保留现有普通安装接线和门控，任务 49 分别记录普通路径待回归与两架构受管阻塞。[固定版工具调度](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.0/packages/coding-agent/src/mcp/tool-bridge.ts) · [固定版 MCP 执行帧](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.0/packages/coding-agent/src/mcp/client.ts) · [固定版 ACP 更新映射](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.0/packages/coding-agent/src/modes/acp/acp-event-mapper.ts)
- 后续对两架构分别执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_omp_contract_cli_local -- --ignored`，应各命中 1 项。待受管路径可用后，以 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_omp_contract -- --ignored --skip acp_platform_omp_contract_cli_local` 仅运行受管探针。此轮仅做限定差异、Rust 格式和 Python AST 检查，未运行安装、编译或测试。
