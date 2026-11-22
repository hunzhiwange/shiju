# 任务 14 适配 codebuddy

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 codebuddy 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/codebuddy.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/codebuddy.rs`、`crates/libs/acp-access-policy/src/access_policy/codebuddy.rs`、`patches/acp-model-source/codebuddy`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_codebuddy_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/codebuddy.md`、`docs/dev/acp/skill-coexistence/codebuddy.md`。

## 上下文

审计基线版本为 2.179.3。MCP 现状｜原生发现链 ToolSearch/DeferExecuteTool 执行并回传；完整原包对照待查。模型来源现状｜受审，要求 --acp；通用，原生权限覆盖有限。原生工具现状｜默认普通可搜索及检查就绪，挂载可延迟执行；私有开关改直接声明，转发器只裁剪服务。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（技能及配置隔离、Shell 请求适配、私有模型环境；默认受管参数形状需核对），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_codebuddy_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_codebuddy_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 固定制品与入口 | 未完成验证 |
| --- | --- | --- |
| x86_64 | `@tencent-ai/codebuddy-code@2.179.3`，`node_modules/.bin/codebuddy --acp`，映射到主包 `bin/codebuddy`。固定 x64 rg、ripgrep.node、genie-trash、sandbox-cli 和 PTY 助手 | Docker `linux/amd64` 未执行 |
| aarch64 | 同一主包与入口。固定 ARM64 rg、ripgrep.node、genie-trash、sandbox-cli 和 PTY 助手。ARM64 PTY 由 `@lydell/node-pty@1.2.0-beta.14` 的可选依赖引入 | Docker `linux/arm64` 未执行 |

- 使用 [npm 固定归档](https://registry.npmjs.org/@tencent-ai/codebuddy-code/-/codebuddy-code-2.179.3.tgz) 与现有 v2 受管补丁。主包 SHA-512 匹配仓库合约，三个 JS 的原始摘要、补丁替换点和五个受管文件摘要均通过。保留 Apple Silicon v2 及其既有行为。上游原生 Beta 安装器跟随最新发行，现有补丁面向固定 npm JS。两个目标使用 [Node.js 18.20+](https://www.codebuddy.cn/docs/cli/installation)、npm、Python 3 和 /bin/sh。原生扩展仍需在相应 GNU 容器上验证 ABI 和沙箱支持。
- [Linux 制品清单](../../patches/acp-model-source/codebuddy/linux-artifacts.json) 固定了主包、PTY 装载器、两个架构的 PTY 与沙箱归档完整性，记录原生可执行文件和扩展摘要。两个架构的 ELF 类型及各归档 SHA-512 已静态核对。主包直接依赖还包括 `@openai/agents-core@0.20.0` 和 `@agentclientprotocol/sdk@1.9.0`。传递依赖锁及主包懒加载的所有组件仍未完整固定。
- 任务 48 在 `crates/libs/acp-agent-registry/src/managed_runtime.rs` 用 `#[path = "managed_runtime/codebuddy.rs"]` 并按 Linux GNU x86_64/aarch64 声明 `mod codebuddy`。在共享 `verify` 的 npm CodeBuddy 分支完成现有主合约后调用 `codebuddy::verify(directory, contract)`，使暂存发布、已安装分支、默认发现及覆写校验共用。Linux 的 `verify_reviewed_agent_executable` 须将相同入口校验贯穿显式覆写和恢复，启动前沿用已审入口校验。维持 v2 固定目录 `codebuddy-2.179.3-model-source-v2`，Linux 入口参数仍为 `["--acp"]`。校验失败后普通 `cli_local` 仍可通过独立发现走原生路径。
- 任务 48 在 `managed_runtime::install` 增加两个 Linux GNU 目标的受管安装子集，继续物化 `codebuddy/apply.py` 与 `patch.json`。Linux 建议使用 `npm --global=false --package-lock=true install --include=optional --ignore-scripts --save-exact --registry=https://registry.npmjs.org @tencent-ai/codebuddy-code@2.179.3`，保留公共重试及无审计/募资参数。必要时在安装命令显式追加目标架构的 sandbox-cli 与 node-pty 预构建包及其已审版本，保证置于根 `node_modules`。启动校验核对这些包的 lock 身份和已安装摘要，并拒绝已审路径重定向、缺少执行位及主包/PTY 下的嵌套依赖。
- 模型路径复用 `model_source::prepare_profile`、`drivers::codebuddy::configure` 与私有回环 `/v1/chat/completions`，明确选定主、快、慢及子代理模型，保留 `--setting-sources none` 与温度能力判断。技能复用 `skill_mounts::codebuddy::configure` 的空视图限制和已验证投影视图，保留绑定项目目录及 headless bundle 选择。v2 先完成供应商原生技能发现，再追加当次会话技能视图，并将 Shell 调整为宿主 terminal 的结构化请求。会话外挂走标准 ACP `mcpServers`，保留 `WaitForMcpServers`、`ToolSearch`、`DeferExecuteTool` 的原生链及同调用 ID 回传。旧审计发现首次精确查询可能先得到模糊候选，验收夹具按原逻辑重试并校验完整工具名。
- 完整运行契约仍阻塞。默认 CodeBuddy 原生 reads 可自动执行，现有模式控制仅为 partial，不能承诺逐调用宿主批准。供应商静态 MCP 未接入宿主执行边界，私有模型设置还关闭供应商设置层。外挂 stdio 的原生审批能提供宿主决定，但尚无对任务 4 的按提示、服务、参数及原调用 ID 的执行前消费与撤权接线。两个 Linux 目标继续拒绝发布完整受管能力。任务 48 须保持现有对外门控；在启用完整能力前需要可校验的原生 I/O 和静态/外挂 MCP 执行边界，并核对 `session/load`、取消及挂载撤销后无法复用旧授权。HTTP/SSE MCP 仅有 ACP 能力声明。任务 49 分别登记两架构缺口。
- 任务 48 在 `acceptance_support/mod.rs` 以 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]` 注册 `mod platform_codebuddy_tests;`。验收从生产网关安装与默认发现入口确认固定制品及 `--acp`，篡改 rg 后要求默认发现拒绝原入口。随后复用共存夹具经生产网关跑 baseline、mounted、removed、rejected 四个新会话。模型夹具验证实际 Chat Completions 路径、精确模型别名、两类技能目录与正文、原生 Read 返回，以及批准后同 ID MCP 续轮。拒绝可由匹配调用 ID 的原生取消终态报告，服务审计要求零新增执行，撤除轮同样零新增。测试还复用私有插件控制。新会话撤除与单次拒绝样例尚未覆盖同原生会话恢复、原生静态 MCP 和执行时撤权。旧共享安装走通用 npm，默认入口断言应失败。仅开通安装时，篡改断言仍会失败。
- 已通过主包与五个辅助归档完整性校验、既有补丁的临时源码应用、产物摘要、四份 JS 的 Node 语法检查、Rust 格式检查和 Python AST 解析。未安装、启动供应商、执行 Cargo 或运行 Docker。后续在两个非 root 目标容器分别提供当前源码的 `shiju` 网关，设置 `SHIJU_ACP_CODEBUDDY_GATEWAY_BIN` 为绝对路径并运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_codebuddy_contract -- --ignored --nocapture`。预期每目标命中 1 项且零失败，当前无退出码和实际命中数。
