# 任务 27 适配 hermes

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 hermes 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/hermes.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/hermes.rs`、`crates/libs/acp-access-policy/src/access_policy/hermes.rs`、`patches/acp-model-source/hermes`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_hermes_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/hermes.md`、`docs/dev/acp/skill-coexistence/hermes.md`。

## 上下文

审计基线版本为 0.19.0 / MCP SDK 1.26.0。MCP 现状｜缺 extra 时接收声明后提前退出注册；补依赖后目录由 15 到 23 并真实回传。模型来源现状｜原版有本地配置；受审 v2 私有绑定；通用。原生工具现状｜默认目录保留官方工具；挂载历史 59 次 MCP，官方同轮执行未取证。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v2 安装 MCP extra，隔离状态、选模及 Skill），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_hermes_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_hermes_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 固定制品及静态证据 | 受管运行状态 |
| --- | --- | --- |
| x86_64 | Hermes 0.19.0、MCP SDK 1.26.0、ACP SDK 0.9.0 均提供 `py3-none-any` wheel。PyYAML、Pillow、cryptography、psutil 和 pydantic-core 有适用 Python 3.12 的 x86_64 manylinux wheel。 | 缺少等价受限启动与原生执行审批。 |
| aarch64 | 同一组通用 wheel。上述原生依赖均有适用 Python 3.12 的 aarch64 manylinux wheel。 | 同一实现缺口，未获得 ARM64 运行证据。 |

- 固定 URL 与 SHA-256 见 [Linux 清单](../../patches/acp-model-source/hermes/linux.json)。已下载到临时目录并比对三个归档摘要。Hermes wheel 的入口元数据声明 `hermes = hermes_cli.main:main`，`acp` extra 固定 ACP SDK 0.9.0，`mcp` extra 固定 MCP SDK 1.26.0 和 Starlette 1.0.1。MCP extra 缺失时旧注册函数会因 `_MCP_AVAILABLE` 返回空目录。两个目标仍需原生 Python 3.12、uv、patch 和 glibc。已核对列出的原生依赖制品存在性，完整传递依赖解析与冷装仍待回归。[Hermes 0.19.0](https://pypi.org/project/hermes-agent/0.19.0/) · [MCP SDK 1.26.0](https://pypi.org/project/mcp/1.26.0/) · [ACP SDK 0.9.0](https://pypi.org/project/agent-client-protocol/0.9.0/)
- 任务 48 在 `managed_runtime/install.py` 的 Python 分支按目标消费上述清单。下载三个固定 wheel 并在安装前验证归档 SHA-256。沿用 `uv venv --python 3.12 --relocatable` 和 `uv pip install --python <venv>/bin/python --index-url https://pypi.org/simple`，给固定 Hermes wheel 带上 `[acp,cli,mcp]` 并将固定 MCP/ACP wheel 一起传入。先应用 `hermes-agent-0.19.0-session-state.patch`，再应用 `coexistence.patch`，按共享 v3 合约核对 12 个 site-packages 文件。在 `managed_runtime::{reviewed_agent_command,verify_reviewed_agent_executable}` 让 Linux 受管发现、显式覆写和运行都绑定同一已验绝对 `bin/hermes`，并校验 Python 目标和依赖身份。当前通用 Python 显式覆写分支只验同级环境，缺少返回入口与传入路径的等值判断。Linux 完整身份及隔离未完成前保留 `agent_registry.rs` 普通 `hermes acp` 和现有发布门控。
- 受管模型沿用 `drivers::hermes::configure` 追加 `--session-model-binding`，主模型、辅助任务和 delegation 统一指向私有 Chat Completions 回环。北向协议沿现有中继选择。原生技能来自工作区 `.hermes/skills`，选择技能使用 `SHIJU_HERMES_SKILL_ROOT` 私有视图。上述扫描、模型和补丁代码按源码可复用。Apple Silicon 仍使用 v3、`HermesSandbox::command` 的 Darwin 路线和现有 Darwin 夹具。
- 外挂 stdio MCP 当前由 `request.rs` 的 `restricted_hermes` 经宿主转成同一回环 HTTP relay。供应商补丁在调用前请求一次 ACP 审批，宿主 relay 在执行前按当前提示轮、工具与参数消费授权。供应商登记保留进程全局连接，同进程 load/resume 的空列表会跳过刷新。宿主的受管请求按 `launch_profile.is_some()` 关闭客户端，恢复需新进程、从当前声明重建 MCP 和技能视图、重新授权。保留此路径，禁止复用旧连接或旧审批。当前供应商审批使用独立 UUID，尚缺执行端原始调用 ID 绑定，原生静态 MCP 也缺少相同中继约束。
- 两个 Linux 目标继续拒绝受限运行。原因是 `drivers::capabilities` 的平台检查、`request.rs::restricted_hermes` 条件和 `acp_client::HermesSandbox::command` 都只接 Darwin arm64。现有 Bubblewrap/Docker 的网络隔离会同时封住宿主回环，现有 Landlock 实现没有按端口约束，不能直接替换 `sandbox-exec`。启用前需要等价私有文件根、受限进程执行、只到当前认证回环的通信通道，以及原生文件写入、命令和静态 MCP 的执行前审批。Linux 原生读取和技能视图还需在该边界中验证。由任务 49 按架构登记这些实现阻塞。
- 任务 48 在 `acceptance_support/mod.rs` 按 `cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))` 注册 `platform_hermes_tests`。用真实网关、明确指定的测试专用受审安装和本地模型/stdio 夹具校验制品、鉴权 Chat 请求、原生 `read_file`、两类 `skill_view`、MCP 的同 ID 模型续轮、一次批准和拒绝。在无挂载轮主动调用旧 MCP 名称并要求错误结果与服务审计零新增。四相位沿现有网关脚本使用不同会话，仍需补同一宿主会话撤除后恢复、原生静态 MCP 和原生写入/命令审批的正反例。接线后旧产品预期在 Linux 能力或沙箱门控处失败。
- 本轮仅做静态检查。两层补丁在已验 Hermes wheel 的解包目录均通过原应用器校验，与 MCP wheel 文件合计匹配生产合约的 12 个摘要。定向 `git diff --check`、Rust 格式检查及 Python 语法检查通过。未安装、启动、编译或运行测试。后续授权且接线就绪后，在各自 Linux Docker 中设置 `SHIJU_ACP_HERMES_GATEWAY_BIN` 和 `HERMES_COEXISTENCE_RUNTIME`，执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_hermes_contract -- --include-ignored`。当前没有退出码或实际测试数。
- 制品清单和验收源码已提交 `bd0d177d44`。保留本任务卡供任务 48、49 取证。

## 续查与追加交付

- 两个 Linux GNU 目标现在有独立 v4 受管叠加层。按固定摘要安装 v3 基础后，用已有 `apply_patch.py` 将 [Linux MCP 清单](../../patches/acp-model-source/hermes/linux-mcp.json) 作为同目录 `patch.json`，应用 `linux-mcp.patch`。仅替换两个文件的 v3 摘要，其余十个受审文件继续用现有合约。任务 48 在共享安装与验证入口消费 `linux.json` 的 `revision` 和 `mcp_overlay`。Linux 验收夹具已要求 v4 目录和两处新摘要。Apple Silicon 和普通 `cli_local` 继续原路由。
- Linux 受管 MCP 入口限制为模型中继同一回环源下的授权 HTTP 投影。直连 stdio、SSE、其它源、重复服务、无凭据声明、注册异常和空工具目录均不可进入受管提示。进程绑定首次成功的会话与挂载集合。原集合可重复读取，改变会话或集合会让旧进程拒绝提示和新工具调用，宿主令新进程用当前声明恢复。访问资源和提示的四个旁路入口显式拒绝，传输失败后不自动重试获批调用。
- 固定 Hermes 原生 dispatcher 已向工具上下文传递模型调用 ID，MCP SDK 1.26.0 支持在 `ClientSession.call_tool(meta=...)` 发出扩展键。叠加层在 Linux 受管调用上用同一 ID 请求 ACP 一次审批，并通过 `params._meta.shijuCallId` 发出执行帧。缺 ID、审批拒绝和等候执行时范围已撤销均在供应商边界拒绝。任务 48 的宿主接线还需让 `acp-client/src/client/event_client.rs` 和 `vw-agent/.../model_source/mimo_mcp.rs` 给 Hermes 的授权签发及消费传递并核对同一调用 ID。当前宿主仍只用提示有效性、工具与实参消费，不能为并发相同工具和实参证明精确匹配。该元数据只在宿主受理投影时用于核验，无需下发给原生服务。
- 复核共享宿主发现第二个独立门控。`mimo_mcp::prepare` 不仅在函数入口限定 Apple Silicon，也用 `/usr/bin/sandbox-exec` 隔离选中的每个 stdio peer。直接更改 ACP 进程条件也无法安全开启会话外挂 MCP。现有工具沙箱实现只能断网或复用整个宿主网络，Landlock 当前实现仅做文件约束。两架构都需要宿主提供允许 Hermes 只访问当前认证回环的执行约束，以及独立的 stdio peer 文件和进程隔离。源端叠加层解决的是声明绑定和调用传递，已经送到宿主的请求仍由宿主租约决定是否执行。
- 原生审批替代方案也已复查。上游 `edit_approval.py` 只覆盖部分写入提案且允许模式自动批准，`terminal_tool` 的回调只对上游发起的确认生效。通用工具 dispatch 挂钩不能约束供应商直接的文件、进程和 MCP 路径，配置钩子无法代替执行隔离。两种架构的原生写入、命令与静态 MCP 仍须已审宿主执行接口或等价沙箱能力，禁止把内部注册或模型工具列表作为审批证据。普通模式继续现有门控。
- 新增 `linux_mcp_tests.py` 使用受审 Python 环境，可在未来 Linux 容器运行 `<venv>/bin/python patches/acp-model-source/hermes/linux_mcp_tests.py`。断言同进程撤除与不同会话后拒绝、直连拒绝、缺 ID 拒绝、审批后撤回和 MCP 执行帧保留原 ID。通过原摘要应用器将叠加层静态应用到临时解包的 v3 源码，确认补丁和结果摘要，并用本机 Python 3.12 解析供应商及新增夹具源码。Rust 格式与限定差异检查通过，未执行任何测试、安装或启动。当前状态保留已阻塞，原因是上述宿主执行约束和精确租约输入仍缺，Linux x64 和 ARM64 都受影响。
- v4 叠加层与验收源码已提交 `9381790fb`。
