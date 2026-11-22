# 任务 9 适配 glm-acp-agent

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 2】 依赖：批次 1

状态：已阻塞

来源任务：无

目的：完成 glm-acp-agent 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/glm_acp_agent.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/glm_acp_agent.rs`、`patches/acp-model-source/glm-acp-agent`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_glm_acp_agent_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/glm-acp-agent.md`、`docs/dev/acp/skill-coexistence/glm-acp-agent.md`。

## 上下文

审计基线版本为 1.17.1。MCP 现状｜原版 stdio 真实执行及同 ID 回传，回环目录 0/8/0。模型来源现状｜原生环境变量选模；受审私有状态；macOS/Linux/Windows 代码覆盖，Windows Node 和私有 DACL。原生工具现状｜当前空挂载、无名单、仅 deny、全 allow 或混合均保留官方工具；旧全 allow 才触发的全目录裁剪已移除；带策略的宿主全拒绝仍生效。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

1. 保留 1.17.1 v2 在 Linux x64 和 ARM64 上的安装契约与显式拒绝优先规则，检查工具结果来自选定服务、Linux 容器证据的覆盖边界。在本编号文件记录可供批次 3 复用的实现与接线模板，覆盖已有跨平台受管补丁。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_glm_acp_agent_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_glm_acp_agent_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 安装、启动与身份源码证据 | 未完成验证 |
| --- | --- | --- |
| x86_64 | npm `glm-acp-agent@1.17.1`，固定 `dist/index.js`，由宿主解析绝对 Node 路径启动，无供应商参数 | Docker `linux/amd64`（非 root）未运行 |
| aarch64 | 同一 npm tarball 和 JS 入口，包无 `os`、`cpu`、可选平台依赖或原生模块 | Docker `linux/arm64`（非 root）未运行 |

- 固定源为 [npm 1.17.1 tarball](https://registry.npmjs.org/glm-acp-agent/-/glm-acp-agent-1.17.1.tgz)。实际下载字节匹配 `contracts.json` 中的 SHA-512 SRI。对临时源码副本核对五个原始文件摘要、应用 v3 技能补丁，六个安装后文件和补丁自身均匹配现有 SHA-256。入口 SHA-256 为 `d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98`。未运行下载的源码。
- 安装复用 `acp_agent_registry::install_builtin_agent` 及其 `managed_runtime::install` GLM 分支。Linux 需要 Python 3、Node（上游引擎 `^20.19.0 || ^22.13.0 || >=24`）及 npm。现行安装使用 `npm --global=false --package-lock=true install --registry=https://registry.npmjs.org --ignore-scripts --prefer-offline --fetch-timeout=20000 --fetch-retries=2 --fetch-retry-mintimeout=1000 --fetch-retry-maxtimeout=5000 --no-audit --no-fund --save-exact glm-acp-agent@1.17.1`，核对 npm lock 中主包 SRI，Python 应用补丁后校验受审文件。上游依赖为 `zod ^3.25.76`、`openai ^6.34.0`、`undici ^6.29.0`、`@agentclientprotocol/sdk ^0.20.0`。生成的 lock 每次安装解算依赖，仓库尚未固定传递依赖图。当前安装目录为 `glm-acp-agent-1.17.1-model-source-v3`。v2 的服务名单规则由 v3 保留，现行注册表识别 v3 的摘要与目录。
- 普通 `cli_local` 保留现有注册项及 `npx -y glm-acp-agent@1.17.1` 原生回退。现行 `session_model` 的 GLM 驱动在两个 Linux 目标均走 Unix 私有目录校验，绑定本机 `/v1/chat/completions` 中继与明确模型名。继续复用 `model_source::prepare_profile`、`drivers::launch_route`、`configure_profile` 和 `acp-client::spawn_child_at` 的固定入口验证。
- v3 补丁沿用供应商原生工具、原生斜杠发现和选择后的技能视图。验证过的工作区内技能由 `skill_mounts::glm_acp_agent::configure` 发布为 `SHIJU_GLM_SKILL_VIEW`。空视图为 `[]`，同样启用工作区原生技能目录及逐次会话 MCP 审批。目录仅发布名字及路径，正文和附件由原生 `read_file` 读取。工作区外的技能继续受原有范围限制。外挂 MCP 走供应商 `connectSessionMcpServers`，按所属服务处理 `shiju.glm.toolPolicy`，显式拒绝优先；带策略的宿主 `DenyAll` 由 `acp-client::validate_mcp_contract` 拒绝。挂载工具从当前服务绑定调用，v3 支持批准后执行、拒绝时不分发、同 ID 回传和错误状态。保留 Apple Silicon 现行入口。
- 任务 48 接线点。以稳定 key `glm-acp-agent` 复用现有 v3 合约、`reviewed_agent_command` 和 Node 固定入口。技能候选平台使用独占模块的 `supports_session_view_target()`。待受审后，将该判定与已审入口、版本及有效权限共同接入 `skill_mounts/mod.rs::validate` 和 `request.rs::do_stream_request_acp` 的 GLM profile 分支。保留 `session_model`、禁用工具和 `DenyAll` 检查，以及会话级撤权和审批注册。Linux 须保持对外技能与审批门控，直至解决下列阻塞并取得目标证据。现行模型能力判断仍在 `model_source/drivers/mod.rs::capabilities`，不据此扩大技能或 MCP 能力。
- 测试接线在 `acceptance_support/mod.rs` 增加 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_glm_acp_agent_tests;`。Rust 验收复用生产网关和既有 GLM 夹具，记录固定入口、实际模型 API 路径及选模读回。夹具断言原生与外挂技能目录及文件读取、按同一调用 ID 返回的选中服务随机结果、批准后一次调用、拒绝及撤除轮零新增审计。供应商直接对照另覆盖同一工具同时列入允许和拒绝名单。旧 Linux 路由在无挂载轮缺模型侧原生技能目录，挂载轮还会在技能门控失败。网关撤除轮使用新会话。补充的供应商直连测试通过同一会话 ID 检查 `session/load` 后的撤权。
- 阻塞。上游内建视觉 MCP 由 `visionClient` 懒加载 `@z_ai/mcp-server@latest`，该制品及供应商账户未受审。中继绑定使用私有令牌，不能据 stdio 挂载证据认可视觉路径，也没有固定制品及安全等价的本机接入可以复用。原生 `read_file` 仍可按其工作区边界复用。原生写入/命令审批及同会话取消、恢复、撤权尚无本任务的 Linux 回执。两种架构均需独立核对；Apple Silicon 既有回执、源码和旁证不计为任一 Linux 运行结果。

### 本轮验证

- 通过归档、补丁和产出摘要核对，Python AST 解析、定向 Rust 格式及 Git 空白检查。没有运行安装器、Node、Cargo 或 Docker。
- 后续在每种 Linux 架构容器中先提供当前源码构建的 `shiju` 网关，设置 `SHIJU_ACP_GLM_GATEWAY_BIN` 为其绝对路径。测试标记为 ignored 以免常规单测隐式安装上游包，实际命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_glm_acp_agent_contract -- --ignored`。接线后预期恰好命中 1 项，零失败。当前退出码及命中数未获取。

### 继续排查与收敛

- `RuntimeHost::prepare_stdio_mcp_launch` 使用 `mimo_mcp::prepare` 把宿主选定的 MCP 服务器挂到中继路由。Cortex 的 stdio 桥也只转换宿主声明的会话服务。GLM 的 `visionClient`、附件预处理、`web_search`、`web_reader` 和 `/usage` 均由供应商进程内部直接调用，两个现有桥无法拦截或替代。v2 受管补丁和 v3 技能补丁只处理会话 MCP，历史 MCP-only 补丁会移除本地原生工具，不能复用其目录裁剪。
- 针对可独立交付的 Linux 子集，新增 `patches/acp-model-source/glm-acp-agent/glm-acp-agent-1.17.1-linux-session-model.patch`。它叠加在已校验的 v3 上，仅在 Linux 且宿主发布了 `VW_ACP_LAUNCH_REFERENCE` 时生效。供应商网页及视觉 MCP 从目录移除，强制调用也失败，普通附件的视觉回退和供应商额度查询沿显式不可用路径。会话 MCP、原生写入与命令沿现有 ACP 逐次审批，供应商权限模式不能短路审批。本地读取和技能目录继续复用。启动 stdio 或命令子进程时移除从上级继承的中继令牌及引用，已选 MCP 的显式环境维持原语义。此环境处理不提供进程级沙箱。Apple Silicon 和普通 `cli_local` 保留现有行为。
- 任务 48 可以将该层作为两种 Linux GNU 目标的独立 v4 修订（建议目录 `glm-acp-agent-1.17.1-model-source-v4`）。在校验 v3 摘要后，用同一 `apply_glm_patch(root, linux_patch, v3_files)` 叠加并校验下表。将 `managed_runtime.rs::installed_directory`、`reviewed_agent_command`、`verify_reviewed_agent_executable` 及安装素材选择统一绑定该 Linux 修订；受管模型启动不得回退旧 v3。其余 v3 文件、npm SRI、固定 JS 入口和平台门控沿用。网关验收夹具须同步复制第二补丁、读取 Linux 摘要并期望 Linux 修订；供应商直连测试如已通过正式安装应用该层，则删除夹具内重复的临时 patch 步骤。技能与会话 MCP 仍按上节接线。
- `managed-coexistence.test.mjs` 增加同一会话撤除并恢复、强制调用被撤除工具后服务零新增、Linux 供应商工具隐藏和强制调用拒绝、获得 ACP 审批的原生命令不继承中继环境的断言。Node 静态语法检查通过，未执行该测试。两个架构仍须在各自 Docker 容器中复跑网关验收；供应商直接对照后续入口为 `node --test patches/acp-model-source/glm-acp-agent/managed-coexistence.test.mjs`。
- 完整合同仍为已阻塞。要恢复 Linux `session_model` 的供应商原生 MCP，至少需要固定且可审计的供应商视觉运行时，以及独立于会话中继的供应商授权或等价本机协议。当前安全子集仍待任务 48 接线及双架构运行证据。本轮保持对外门控，未运行 Node 测试、Cargo、Docker 或运行时安装。

| Linux 叠加产物 | SHA-256 |
| --- | --- |
| `glm-acp-agent-1.17.1-linux-session-model.patch` | `0c1bfc5d1cc48fbd0ce82bde59880da7b497e906438f8510b6d41ab5be7a3770` |
| `dist/protocol/agent.js` | `643c2f13a66fc18df8ff8e7aabcab5aa5052370fa863111a992dff07f6a9a561` |
| `dist/tools/executor.js` | `2c83a349eb3cd272a4d8762ffa40c4f910185a725dc36acc95e3e35fd7931c42` |
| `dist/tools/session-mcp-client.js` | `e6f448f8e275791edf1b5e2ccacd8ccfe394be56ca3cb9c75de79888530e8ef6` |

静态证据基于 npm 原版、现行 v3 合约和生产 Python 精确补丁函数。两次应用及修改后 JavaScript 的语法检查均通过。上一提交 `db91472de` 提供独占技能候选入口和网关测试源码，本次提交提供 Linux 受管叠加层、供应商测试及接线证据。

### 宿主投影与本机子集

- 两种 GNU 目标已接入 v4 私有选模。本次保留 v4 文件与门控，新增从已审 v3 直接应用的累计叠加层 `glm-acp-agent-1.17.1-linux-host-mcp.patch` 和 `linux-host-mcp.json`。任务 48 将其接到独立安装目录 `glm-acp-agent-1.17.1-model-source-v5`，沿用安装素材名 `glm-linux-contract.json`、`glm-linux-session-model.patch` 与内部参数 `--glm-linux-v4`（参数选叠加格式，目录标识制品修订）。Linux 会话 MCP 须校验 v5 入口。Apple Silicon 和普通 `cli_local` 沿用现有路径。
- v5 受管分支在任何 MCP 初始化前只接受与模型中继同源的 `http://127.0.0.1:<port>/shiju-vtcode-mcp/<service>`，需要独立的投影 bearer，拒绝原始 HTTP、SSE、stdio、重定向和模型 bearer。工具名调用 `drivers::glm_acp_agent::projected_mcp_tool_name(service, source_tool)`，得到最长 64 字符的 ASCII 名称 `mcp__<service>__<source_tool>`，构建目录时拒绝碰撞。ACP 审批携带同一模型调用 ID、工具名与对象参数，批准后通过 HTTP `tools/call.params._meta.shijuCallId` 交回宿主消费。新建、加载和分叉复用该检查。
- 任务 48 宿主接线。通过 `prepare_stdio_mcp_launch` 走请求所有的投影入口，在 `model_source/mod.rs` 注册同级 GLM 专有 `glm_mcp`，将投影结果装入 `AcpLaunchProfile::managed_mcp`。在 `mimo_mcp::prepare` 对两种 GNU 目标复用 `Projection::approval_config`、`dispatch` 与 `VtcodeApproval::new(true)`，绑定当前提示轮、真实服务、工具名、参数和 call ID，单次消费。按原始服务的 `shiju.glm.toolPolicy` 筛选目录，显式拒绝优先。缺省名单只表示没有项目工具筛选，仍须有宿主显式选择和逐次准入。无审批、非交互 allow、停用工具和 DenyAll 均拒绝。撤权时撤销 relay binding，缓存继续按照当前选择和权限确定复用。共享 `validate_linux_session_mcp` 和 ACP client `validate_mcp_contract` 仅对通过平台与 v5 校验的投影分支开放，ACP client 还须校验已有 `managed_mcp_approval`。技能沿用 `supports_session_view_target()` 和独占 `configure`，保留技能视图失效检查与现有对外门控。
- `glm_mcp::open_selected_peer` 只接宿主已选定义。HTTP 复用宿主无交互且同源的连接入口，禁止重定向和 HTTP 重试，接收 relay 取消令牌。stdio 由宿主通过 `open_isolated_stdio_peer` 启动绝对路径 `/usr/bin/bwrap`，只读挂载运行所需系统目录，隔离 PID、网络和临时目录。仅接受 `/usr/bin` 下的绝对系统程序，清空环境，不挂载宿主的 HOME、项目或临时目录。无 bwrap、namespace 失败、非系统入口、环境覆盖或其他传输均失败。仅覆盖自包含 stdio 服务，依赖项目文件或插件私有入口的服务仍拒绝。
- 网关夹具验证 v5 摘要、实际 HTTP fixture 独立认证、真实资源审计和同 ID 续轮，批准执行一次，拒绝与撤除为零。供应商直连夹具检查原始传输被拒、模型 bearer 无效、投影收到匹配的单次授权、拒绝无派发及同一会话恢复后的撤权。两个架构的 stdio 仍须在支持 bwrap 的容器做宿主启动、拒绝及取消审计。
- 静态应用原始 npm → v3 → v5 补丁，用生产精确补丁函数逐次核对 SRI、文件和补丁摘要。JavaScript 语法、Python AST、Rust 格式及 Git 空白检查通过。未运行安装器、测试、Docker 或 Cargo。本机子集待任务 48 共享接线和双架构回归，供应商视觉 MCP 单独阻塞于固定可审制品和隔离授权。
