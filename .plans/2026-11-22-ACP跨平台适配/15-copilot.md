# 任务 15 适配 copilot

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 copilot 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/copilot.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/copilot.rs`、`crates/libs/acp-access-policy/src/access_policy/copilot.rs`、`patches/acp-model-source/copilot`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_copilot_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/copilot.md`、`docs/dev/acp/skill-coexistence/copilot.md`。

## 上下文

审计基线版本为 1.0.105。MCP 现状｜原生拒绝 stdio；受控 HTTP/SSE 已发现、执行并回传。模型来源现状｜原生可选模；受审 Darwin 平台包；通用。原生工具现状｜默认保留；挂载历史 74 次目标和 2 次官方 grep；显式字面 --available-tools 可裁剪全目录，空表沿默认。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（私有 Provider、技能/插件发现隔离，沿用目录重建守卫），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_copilot_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_copilot_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

| Linux GNU 目标 | 固定平台包 | 制品与静态验证 |
| --- | --- | --- |
| x86_64 | [@github/copilot-linux-x64 1.0.105](https://registry.npmjs.org/@github%2fcopilot-linux-x64/1.0.105) | npm SHA-512 SRI、ELF machine 62、SEA 及提取文件 SHA-256 匹配 |
| aarch64 | [@github/copilot-linux-arm64 1.0.105](https://registry.npmjs.org/@github%2fcopilot-linux-arm64/1.0.105) | npm SHA-512 SRI、ELF machine 183、SEA 及提取文件 SHA-256 匹配 |

- 两个目标复用 [@github/copilot 1.0.105](https://registry.npmjs.org/@github%2fcopilot/1.0.105) 主包。Linux 源为二进制内的 `copilot.tgz`。仅静态解压并检查，两个平台的 `index.js`、`app.js`、原始 `cli-main.js` 三者各自与 Darwin v2 合约一致。复用原补丁后，`cli-main.js` 摘要仍为 `5233b13e859de96f86a21bd8806611dc0bf534e4b63d45e6d81475f92100bd64`。双目标的 npm SRI、SEA 摘要、提取后的启动文件和本机关键载荷见 [Linux 制品清单](../../patches/acp-model-source/copilot/linux-artifacts.json)。GNU SEA 及核心运行时的 glibc 符号最高版本为 2.28，SEA 还链接 libstdc++ 和 libgcc_s。运行入口需要支持 `node:sea` 的外部 Node，静态语法检查使用 24.22.3，Linux ABI 尚待验证。
- 任务 48 安装。在 `managed_runtime.rs::install` 仅为 `target_os=linux,target_env=gnu,target_arch=x86_64|aarch64` 选择 Linux 候选合约，目录严格使用清单中的 `copilot-1.0.105-model-source-linux-v2`。将 `copilot/install-linux.py`、`copilot/linux-artifacts.json` 与现有 `apply.py`、`patch.json` 写入私有资产目录。共享 `install.py` 以 `--global=false --package-lock=true --include=optional --ignore-scripts --save-exact` 固定安装主包与对应 GNU 平台包，复用主包完整性检查。Linux Copilot 分支调用 `python3 -I <assets>/copilot/install-linux.py <staged-runtime> <x86_64|aarch64>`。脚本校验平台包锁定版本、SRI 与 SEA 文件摘要，再在私有缓存使用供应商原有 `COPILOT_SEA_EXTRACT_ONLY=1` 路径，应用 v2 补丁，校验提取文件并将 `.bin/copilot` 指向固定入口。Linux 的末尾 `check_files` 使用 Linux 两组 `files` 的并集。安装必须通过现有 staging 与发布后校验。Darwin arm64 保留现有安装分支和修订目录。
- 任务 48 发现与启动。`managed_runtime.rs::{installed_directory,reviewed_agent_command,verify_reviewed_agent_executable}` 用同一 Linux 合约，在私有安装位置校验所选架构的文件集。入口为绝对 `node_modules/.bin/copilot`，必须解析到校验过的 `node_modules/@github/copilot/embedded/index.js`。运行时文件校验拒绝重定向并流式检查大文件。本机插件、WebView、语音、Foundry 等配置仍受原有隔离限制。新 Linux 受管分支应在 `acp-client::spawn_child_at` 唯一启动边界再次核验入口，固定可信 Node，清除继承的 Copilot、Node、代理及账号环境，并仅注入宿主生成的受管配置。关键文件摘要不构成进程沙箱。稳定 key `copilot` 和原生 `cli_local` 发现继续保留，原生启动参数仍为 `--acp --stdio`。
- 任务 48 模型与技能。候选路径复用 `model_source::prepare_profile`、`drivers::{launch_route,copilot::configure}`、`skill_mounts::copilot::configure`、`access_policy::copilot::validate_launch`。启用条件绑定已审入口、版本、架构、会话模型和有效权限。私有 Provider 沿用 OpenAI Chat Completions 及原生 Anthropic Messages，Responses 保持拒绝。本轮验收源码覆盖 Chat Completions。保留 `--no-auto-update`、离线配置、私有历史目录和挂载变化后的客户端重建。原生项目技能来自匹配当前租户和项目的授权与三项私有 settings 投影，再追加经过校验的会话技能视图。默认工具目录沿用原生，现有显式字面 `--available-tools` 仅作为目录筛选，保留单次审批。保持能力发布门控，并在 Linux 限制 `DenyAll`、禁用工具、未经授权的挂载及 stdio 路径。HTTP/SSE 使用宿主挂载入口，仅 SSE 有本任务的回归源码。
- 测试接线。任务 48 在 `acceptance_support/mod.rs` 增加 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_copilot_tests;`。编译后用同架构网关设置 `SHIJU_ACP_COPILOT_GATEWAY_BIN`，用 `COPILOT_COEXISTENCE_RUNTIME` 指向同架构私有受管安装。回归入口为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_copilot_contract -- --ignored`。追加 `--ignored` 因此用例需要真实受管安装和私有网关。探针经真实网关配置、聊天和会话读取，回环 Chat Completions 校验模型、协议、目录和工具调用 ID，回读获准的 SSE 结果并核对续轮，拒绝与撤除轮服务审计必须为零。原 Linux 代码在测试注册后会因受管平台门控或 Darwin 文件校验失败。撤除夹具使用新会话。
- 两架构的实质阻塞相同。当前 v2 以 `enableConfigDiscovery=false` 禁止供应商既有 MCP 配置和插件发现。私有技能设置仅投影技能字段，不能借放开全量配置引入既有 MCP。现有宿主仅接通会话外挂的原生远端 MCP，受审版本拒绝 ACP stdio。还缺独立的既有 MCP 配置来源、宿主服务归属与逐调用授权接线，以及同会话恢复后撤权的审计用例。原生工具已有 Darwin 历史证据，Linux 原生文件写入/命令审批、原生 Messages、HTTP 和恢复仍缺各自的行为证据。两架构保持既有对外门控。下一步由任务 48 接入独立可用路径，待确认既有 MCP 的授权来源和撤权合同后再扩展。
- 本轮静态检查已通过。对两架构核对归档 SRI、SEA 与 ELF，静态提取并对临时源码副本应用现有补丁，校验清单全部文件及 `node --check`。Python 使用 AST 解析，新增 Rust 测试经单文件 `rustfmt --check`。未执行 Cargo、上游运行时、安装或 Docker。两种架构均无红绿运行结果。
- 独占代码已提交为 `1a34529675`，保留本卡供任务 48 和 49 使用。

## 后续复查

- 任务 48 的 Linux 集中测试模块已注册 `platform_copilot_tests`。共享安装、发现及受管启动仍待接线。上文 Linux v2 路径已被清单中的 `copilot-1.0.105-model-source-linux-v3` 取代。GNU x86_64/aarch64 父模块接入新增的 `managed_runtime/copilot.rs`，其中 `installed_directory(parent)` 返回固定修订，`verify(directory, contract)` 同时用于暂存、发现和启动复核，返回确切的 `.bin/copilot`。该入口核对主包和架构包锁定版本及 SRI、关键文件权限及流式 SHA-256，拒绝文件和启动别名重定向。Linux 验证失败不得尝试 Darwin 合约或任意祖先目录。继续保持公开能力门控。
- 固定版 `PVo` 独立调用 `B8n`，再由 `F8n` 把 MCP 配置交给本机 `S.acpServe`。`B8n` 在 SDK 技能隔离生效前读取用户 MCP 与插件配置。Linux v3 在受管启动标记存在时跳过该读取并禁用内建 GitHub MCP，清除额外启动 MCP 配置。隔离失败停止启动。宿主经 `session/new` 与 `session/load` 传递的显式远端清单仍由既有目录重建与单次审批处理。双架构 Linux 补丁摘要为 `0806d4bf433fa33fb284676b31e0b7d9a79013cdd0f425ca947ae5fd8d4d1f98`。Darwin v2 摘要未变，普通 Linux `cli_local` 沿用原生发现。
- 可交付接线覆盖两个架构的固定入口、Chat Completions 私有模型、原生读工具、两类显式授权技能和会话 SSE MCP。历史直接 ACP 证据覆盖同一会话传空清单恢复后服务零访问，宿主有 `load_session` 清单重传及目录重建。当前 Linux 网关测试源码覆盖新会话撤除。HTTP、原生 Messages、原生写入/命令审批及同会话撤除仍需各自运行证据。
- 两架构的 `session_model` 加供应商既有 MCP 仍需外部输入。固定版支持 `mcpServers` 和 `additionalMcpConfig` 启动映射，SDK 另有实验性 `managedMcpServers`，需要动态凭据回调。现有 ACP 启动没有可信既有服务清单、所有者和撤销版本输入。读取供应商文件会把项目、账号或插件设置带入受管权限域。现有 `AcpMountLeases` 对显式选中的目录记录核对租户与修订，可服务外挂场景。既有服务需提供当前租户明确授权且可撤销的服务来源，或可传递等价身份和动态凭据的 ACP 协议入口。本地 stdio 还需要上游支持或 GNU 可用的隔离桥。现有 `prepare_stdio_mcp_launch` 桥接仅支持 Darwin arm64，依赖 `sandbox-exec`，而本版 Copilot ACP 拒绝 stdio。`--available-tools` 仅筛选目录。完整合同保留已阻塞。
- 重新核对 ARM64 npm SRI、SEA、内置源码及关键文件可执行模式，并复用前次 x64 同源摘要。Darwin 与 Linux 补丁应用、摘要和 `node --check` 均通过。未运行 Cargo、安装器、Docker 或上游运行时。
