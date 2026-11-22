# 任务 32 适配 kiro

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 kiro 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/kiro.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/kiro.rs`、`crates/libs/acp-access-policy/src/access_policy/kiro.rs`、`patches/acp-model-source/kiro`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_kiro_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/kiro.md`、`docs/dev/acp/skill-coexistence/kiro.md`。

## 上下文

审计基线版本为 3.0.0 V3。MCP 现状｜已审制品就绪后原生 stdio 接纳、发现及执行；原包同版挂载全链待查。模型来源现状｜原版显式 V3 可选模；受审 V3、禁外部运行时覆盖；通用，Windows 有原生查找。原生工具现状｜默认可用 Power 等，三工具由转发器限制；历史普通有 Power，挂载同轮缺证。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（私有 V3/Node、Skill 视图和空参数 terminal 转发；每次 open/load 携带当前授予），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_kiro_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_kiro_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | Kiro 固定制品 | 静态证据 |
| --- | --- | --- |
| x86_64 | [官方 3.0.0 ZIP](https://prod.download.cli.kiro.dev/stable/3.0.0/kirocli-x86_64-linux.zip) | ZIP 内 `BUILD-INFO` 为 `x86_64-unknown-linux-gnu`。原生 CLI 与随包 Node 均为 ELF64 little-endian，machine 62。CLI ELF 最高 GLIBC 符号为 2.34。 |
| aarch64 | [官方 3.0.0 ZIP](https://prod.download.cli.kiro.dev/stable/3.0.0/kirocli-aarch64-linux.zip) | ZIP 内 `BUILD-INFO` 为 `aarch64-unknown-linux-gnu`。原生 CLI 与随包 Node 均为 ELF64 little-endian，machine 183。CLI ELF 最高 GLIBC 符号为 2.30，安装仍采用 [官方 glibc 2.34+ 基线](https://kiro.dev/docs/cli/installation/)。 |

- 两包的 ZIP SHA-256、原生 CLI 和 Node SHA-256、ELF machine、入口和启动参数见 `patches/acp-model-source/kiro/linux.json`。从官方固定 HTTPS URL 获取实际字节计算摘要，固定版本的 `manifest.json` 返回 HTTP 403，未取得供应商签名摘要。静态解开原生 ELF 内的 Zstandard KAS tar 和 Node，两个平台的原始 V3 JavaScript 均匹配 Darwin v2 已审源摘要。分别在临时副本应用原 v2 补丁和 Linux 叠加层，核对最终摘要，`node --check`、Python AST、定向 Rust 格式和 Git 空白检查通过。
- Linux 叠加层保留 `SHIJU_KIRO_SKILL_ROOT` 原生扫描和空参数 terminal 转发，在受管 session_model 的 ACP terminal 请求中滤除 `KIRO_API_KEY` 和 `VW_ACP_LAUNCH_REFERENCE`。普通 `cli_local` 继续走 `agent_registry.rs` 的原生 `kiro-cli acp`。Apple Silicon v2 的入口、补丁与注册只读审查。
- 安装交给任务 48 的 `managed_runtime/install.py` standalone 分支。依赖 Python 3、非 root GNU/glibc 环境和归档自带 ELF/Node，native CLI 还链接 `libgcc_s.so.1`。按 Linux GNU 架构选取 `linux.json.targets`，核对 ZIP SHA-256，从 ZIP 固定条目 `kirocli/bin/kiro-cli` 取原生 CLI，核对 ELF machine 与摘要后设 0700。复用 `native/kiro-cli _ extract-runtime --dir <私有暂存目录>`。要求有唯一带 `node` 和 `kas/node_modules/@kiro/agent/dist/server/acp-server.js` 的提取目录，校验 Node、原始 JS 后保留完整 KAS 依赖树并将该目录放到 `engine/`。按顺序应用 `kiro/apply.py <server>` 和 `kiro/apply_linux.py <server>`。复制 `launcher_linux.py` 为 `kiro-cli`，设 0700，校验四项文件摘要后发布到 `acp-runtime/kiro-3.0.0-model-source-v3`。提取命令沿用 Darwin，Linux 实际产出路径待运行核实。
- 任务 48 注册点。用稳定 key `kiro`，在 `managed_runtime.rs` 素材清单加入 `kiro/linux.json`、`kiro/apply_linux.py`、`kiro/launcher_linux.py`，原 `apply.py`、`patch.json` 复用。使 `install`、`installed_directory`、`reviewed_agent_command`、`verify_reviewed_agent_executable` 对两种 Linux GNU 目标使用同一 v3 契约和四项完整摘要。保留旧平台契约。共享启动接线沿用 `model_source::prepare_profile`、`drivers::launch_route`、`drivers::kiro::configure`、`skill_mounts::kiro::configure`，受管入口基参 `acp`，模型驱动追加 `--agent-engine v3 --auth-method cli`，源协议 Responses。技能根由宿主发布到请求视图。会话创建与加载继续经 `acp-client` 的 `mounted_session_meta` 绑定当前 `policyPreset`、`mcp_servers`，授权用当前会话策略，限已审 stdio。Linux 对外能力保持门控，只有固定身份、进程隔离、审批和目标回执均齐备才开放。
- 任务 48 测试注册。在 `acceptance_support/mod.rs` 加入 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_kiro_tests;`。设置 `SHIJU_ACP_KIRO_GATEWAY_BIN` 和 `SHIJU_ACP_KIRO_RUNTIME_DIR` 为同架构网关二进制及受管安装的绝对路径。验收走生产网关聊天和审批 API。外部模型替身要求实际 `/v1/responses` 流、指定模型、原生 Powers 列表和原生 Skill 的真实结果，外挂 Skill 另有私有视图路径及附件断言。四轮区分未挂载、挂载、撤除、拒绝，stdio 服务审计须为 0/1/0/0，模型按调用 ID 收到结果。旧生产路径仍受 Darwin standalone 安装和模型能力门控。安装为显式前置条件，避免普通单测安装或运行上游。
- 两架构均存在实质缺口。原生工具 Power 列表与会话 stdio MCP 已有历史 Darwin 旁证，但独立的原生 MCP 共存、原生写入与命令审批、进程级继承环境隔离、同会话撤除后加载恢复和取消缺少完整 Linux 实现或证据。上游 V3 的独立 MCP 配置与 `autoApprove` 需要在宿主管理场景验证不会越过当轮授权，也需核实 V3 原生子进程继承令牌的边界。终端叠加层只覆盖 ACP 转发。当前验收的撤除轮建立新会话。应先补齐隔离、审批和恢复路径，再分架构开放对应能力。
- 两种 Docker 架构的 Cargo 红灯、绿灯、上游安装与实际运行均未执行。接线、隔离与运行授权齐备后，各执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_kiro_contract -- --ignored --nocapture`，预期每架构命中 1 项、零失败。当前无命中数、退出码或槽位。
- 实施提交 `51ae12f302`。本卡保留给任务 48 接线和任务 49 入账。

### 二次排查

- 复核固定 V3 原始摘要，确认两架构共享相同 JS。受管 Linux 启动改用隔离 Python 解释器和原生环境白名单，HOME 指向宿主私有状态。V3 捕获受管标记和显式 API key 后清除对应环境变量，保留原 v2 的空参数 terminal 转发及请求环境过滤。该隔离覆盖进程继承环境，仍需任务 48 在 `acp-client/src/client/process.rs` 将 Linux Kiro 受管启动并入 `managed_private_provider`，清理运行启动器之前的宿主环境，并在已有受管入口复核点校验安装身份。普通 cli_local 与 Darwin 不受此叠加层影响。
- V3 `fGo` 通过真实 consent desk 处理 shell、写入、MCP 的有效能力。受管分支保留原判定中的 deny，把其余结果收紧到显式 ask。由 `uWo` 禁止持久同意，V3 `r7o` 只有 allow_once、reject_once 两项。供应商 classifier 只处理 evaluate，不再能放行这些能力。宿主仍通过当前会话和工具调用接收、校验和拒绝请求。生产网关验收的 stdio 外部服务增加中继环境隔离断言。
- 恢复路径复用 `acp-client::resume_session_info` 转 `load_session_info`，每次重新传 `mcp_servers` 和 `mounted_session_meta`。宿主 skill view 的变更走既有缓存替换和撤权。V3 自身 `loadSessionAdmitted` 使用当前请求进行 hydrate，历史 Darwin 对照已收到同一原生会话、旧历史、零新增执行。此项无需新的供应商制品，Linux 同会话撤除、取消和恢复仍待回归，网关夹具的撤除对照当前是新会话。
- 剩余协议阻塞限于 x86_64、aarch64 受管场景中 Kiro 自行发现的 `mcp.json` 和 agent profile 原生配置式 MCP。V3 `readConfigFile` 和 `sessionMcpInputs` 先向 MCP 管理器供给连接配置，之后才进入 `fGo` 工具审批。固定入口仅定义了 `mcp/connect` 字符串，未使用该 ACP 请求将独立服务器连接交给宿主。宿主 `prepare_stdio_mcp_launch` 和现有审批投影只接收宿主解析后的会话服务器，不能拦截 V3 自行加载的配置，也不能在当前提示所有权建立前代替宿主批准外部进程。因此 Linux 叠加层在受管场景抑制文件和 profile 两个配置来源，保留上游固定内建和当前 ACP 客户端服务器。要恢复这两类独立配置，需要供应商提供连接前的宿主审批协议，或由宿主接入经过租户授权的原生 MCP 配置来源和启动代理。两架构普通本地 CLI 仍保留其原生配置路径。编号卡为完整合同保留已阻塞，受管模型、原生工具、技能和会话 MCP 子集交任务 48 接线并待 Linux 回归。
- 当前 `linux.json` 的 `server_sha256`、`launcher_sha256` 已更新。重新对固定 JS 顺序应用两层补丁并核对最终摘要，两架构共用相同原始和补丁后 JS，`node --check`、Python AST 和定向空白检查通过。未安装或启动供应商程序，未执行 Cargo、Docker。
- 二次排查实施提交 `5bb25c7eed`。
