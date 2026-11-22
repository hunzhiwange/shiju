# 任务 25 适配 grok

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 grok 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/grok.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/grok.rs`、`patches/acp-model-source/grok`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_grok_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/grok.md`、`docs/dev/acp/skill-coexistence/grok.md`。

## 上下文

审计基线版本为 1.0.77；另审 1.0.24。MCP 现状｜1.0.77 原生 search_tool/use_tool 路径可调用和续轮。模型来源现状｜原版私有 TOML；驱动校验参数及模型，通用，Linux 源码有 Unix 分支，Windows ACL 拒绝。原生工具现状｜直出触发后替换 MCP 发现入口，其他原生定义保留；原生回退保留动态发现。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（Unix 有 Python 时运行时直出已发现目录、重映射调用、按服务过滤与撤销；混合传输无法执行名单时拒绝），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_grok_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_grok_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

| Linux GNU 宿主 | 固定上游包 | 解包入口 |
| --- | --- | --- |
| x86_64 | [@xai-official/grok-linux-x64 1.0.77](https://registry.npmjs.org/@xai-official/grok-linux-x64/-/grok-linux-x64-1.0.77.tgz) | ELF64 x86-64，无 ELF 解释器、NEEDED 动态库或 GLIBC 版本依赖 |
| aarch64 | [@xai-official/grok-linux-arm64 1.0.77](https://registry.npmjs.org/@xai-official/grok-linux-arm64/-/grok-linux-arm64-1.0.77.tgz) | ELF64 AArch64，无 ELF 解释器、NEEDED 动态库或 GLIBC 版本依赖 |

- 两包下载字节通过 npm SHA-512 SRI 核验，Brotli 解包后通过 ELF 和 SHA-256 核验，二进制均包含审计修订号 `50ca579fa996`。版本、规格和摘要见 [Linux 制品清单](../../patches/acp-model-source/grok/linux-artifacts.json)。
- 任务 48 在 `acp_agent_registry::managed_runtime::{install_builtin_agent, reviewed_agent_command, verify_reviewed_agent_executable}` 以稳定 key `grok` 接入清单。按宿主架构下载固定 URL，先核验 SRI，仅流式取归档常规成员 `package/bin/grok.br`，用 Brotli 解压（可复用 Node 内建 `node:zlib`），验证解压 SHA-256 后原子安装为 `grok-1.0.77-model-source-v1/bin/grok` 并设置可执行权限。受管入口为绝对 `bin/grok`，参数仅 `agent stdio`，复核同一安装根、规范路径和内容摘要。顶层 npm 包的 bootstrap 优先复用 `GROK_HOME/bin/grok`，因此受管入口应直接使用目标 ELF。普通 `cli_local` 保留 PATH 上的 `grok agent stdio` 与固定 npm 后备，推理无需 Brotli 解压器。
- `model_source::prepare_profile` 验证原生命令，`drivers::grok::configure` 复用 Unix 私有状态与会话模型 TOML，使用 `chat_completions`、回环认证及已解析的明确模型。Linux 受管组合需要可执行的 `/usr/bin/python3`。任务 48 在 `request.rs::do_stream_request_acp` 对有名单的 MCP 检查实际 `session_model`、受审绝对入口、投影及全 stdio，缺任一项均返回 ACP 选项错误。该检查须覆盖普通、恢复、重启与缓存复用，原生 `cli_local` 或 Python 缺失回退均无法兑现名单。混合传输带名单继续拒绝，现有投影会在绑定时中止。保持现有 Linux 对外门控。
- 技能候选路径在独占 `skill_mounts::grok::configure`，Linux GNU 两架构按清单流式校验 ELF SHA-256，然后验证宿主技能视图并仅追加 `[skills].paths`。保留 Apple Silicon 既有摘要、入口及策略。任务 48 只在可执行受管 Linux 验收前提下接通 `skill_mounts::validate` 和配置分支，仍需保留 `session_model`、禁用工具、`DenyAll` 检查及已有宿主技能读租约。
- 全 stdio 投影保留原生工具，发现后按所属服务名单过滤。对外工具名还原为 Grok `use_tool`、保留调用 ID，并走现有 ACP 权限回调。Linux 上每次 `session/new`、`load`、`resume`、`close` 和输入结束都会撤销旧代的侧车策略文件。仍存活的旧连接在转发 `tools/call` 前验证策略文件，锁序列化撤销与转发。撤销前已送出的调用可能继续完成。
- 完整合同在两架构均阻塞。原生配置的 MCP 由 Grok 自行读取和连接，现有投影只从 ACP `mcpServers` 获得目录与策略，替换 `search_tool/use_tool` 后原生服务无法共存，保留动态发现又不能兑现附挂名单。需要受审的上游目录、项目配置和逐调用执行钩子，或等价宿主约束，才能在同一会话中安全区分两类 MCP 并执行各自权限。现有外部服务侧车仅验证名单和代次，尚未在 `tools/call` 边界消费任务 4 的原调用身份、实参与逐次批准，不能覆盖 Grok 内部权限短路、原生写入/命令执行及提示取消后的全部撤权。原生 HTTP/SSE 仅有无名单动态回退源码，暂无 Linux 宿主授权证据。任务 49 对两个目标均登记这些缺口。原生本机账号的运行结果也待后续验证。
- 任务 48 在 `acceptance_support/mod.rs` 以 `cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))` 注册 `platform_grok_tests`。网关验收复用真实 `/chat/stream`、宿主一次批准和文件/服务审计。夹具核验架构摘要和原生选模协议，断言原生和选择技能正文与附件、原生读工具、外挂 stdio 同 ID 回传，以及拒绝或无选中服务时零新增。旧宿主在挂载轮的能力门控拒绝。网关撤除轮使用新会话。新增侧车测试保留旧连接，通过空表重绑验证旧调用在代理内被拒绝。原生 MCP 同轮及宿主同会话撤权尚缺网关正例。
- 已通过两个归档 SRI、解包 SHA-256 与 ELF 静态核验、Python AST、修改文件 Rust 格式和 `git diff --check`。无安装、编译、运行时或测试执行。后续按计划在 Linux 两架构容器分别设置 `SHIJU_ACP_GROK_GATEWAY_BIN`（当前源码构建的网关）和 `SHIJU_ACP_GROK_RUNTIME`（对应已校验的 ELF），运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_grok_contract -- --ignored`，预期命中 1 项。另两架构分别运行 `PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -m unittest discover -s patches/acp-model-source/grok -p projection_test.py`；当前无红绿退出码与命中数。

## 续查与追加交付

- 已核对共享实现。任务 48 已通过 `acceptance_support/linux_platform_tests.rs` 注册 Grok 验收；受管安装及能力放行仍未接通。`RuntimeHost::prepare_stdio_mcp_launch` 的现有投影由 `mimo_mcp::prepare` 创建，仅控制 ACP 提交给宿主的会话服务。它消费精确动作和当前提示轮的单次许可，但现有 stdio 请求 ID 属于 MCP 连接，无法还原 Grok 的原始 ACP 调用 ID。借用 DeepAgents 的路径还需要调用方显式写入原始 ID，固定 Grok 二进制无该接线。即使改用宿主 HTTP 投影，Grok 原生配置、插件和工作区进程仍可绕过该入口。
- 1.0.77 制品发现上游的连接器网关标识（x64 有 `GROK_MANAGED_MCP_GATEWAY_TOOLS_ENABLED` 和 `/mcp/tools/{list,call}`，ARM64 有相应调用路径）。上游可见源码的 `ShellManagedGatewayToolClient` 从供应商 `AuthManager` 取凭据，且 `GatewayTool.call_id` 属于目录项，执行请求复用该目录 ID，仅另传实参。它无法给宿主区分相同工具的两次 ACP 请求，也未将原生配置的 stdio/HTTP/SSE 接入执行消费。禁止改写用户云端凭据或将目录 ID 当作 ACP 授权 ID。[上游执行客户端](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell/src/session/acp_session.rs) · [上游目录与请求结构](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell-session-support/src/managed_mcp.rs)
- 上游 GitHub 未发布 `1.0.77` 或 `v1.0.77` 标签，构建标记 `50ca579fa996` 无法解析为公开提交，公开 `SOURCE_REV` 为 `559751fdcec02d413e4c57c8832ab275e4f44980`。已审二进制上的设置名只能作为线索。文件信任、工具规则及 PreToolUse hooks 由 Grok 按来源合并，缺固定版本的执行前来源与调用标识保证。仅隐藏工具、设置 ask 规则、匹配工具名称和实参或拦截 ACP 回复无法证明工具副作用已经受逐次审批约束。[来源合并](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell/src/session/managed_mcp.rs) · [上游源码版本](https://github.com/xai-org/grok-build/blob/main/SOURCE_REV)
- Linux `session/new`、`load`、`resume` 遇到带非空名单的混合/远端传输时，投影先撤除旧代，再返回对应 JSON-RPC `-32602`，拒绝转发给 Grok。独立回归夹具让真实投影入口接收混合请求和下一条 ACP 帧，验证拒绝 ID、错误码以及只有下一帧实际进入供应商替身。纯 stdio 仍沿用原 Linux 撤权实现。Python AST 和定向 Git 空白检查通过，未执行测试或 Rust 构建。
- 两种 Linux 架构的带原生 MCP、会话外挂 MCP 和宿主执行前审批的完整组合仍标记已阻塞。可推进前提是取得具有可溯源源码或受管部署契约的上游版本，并在其 native/ACP MCP、文件及终端执行点建立可验证的来源/原调用 ID/实参门禁，或提供等价进程隔离与受管分派。无这些前提时，固定制品、私有模型、技能与全 stdio 投影的静态证据可独立交付，Linux 对外门控保持现状。
