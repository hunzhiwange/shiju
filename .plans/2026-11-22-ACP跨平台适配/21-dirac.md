# 任务 21 适配 dirac

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 dirac 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/dirac.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/dirac.rs`、`patches/acp-model-source/dirac`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_dirac_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/dirac.md`、`docs/dev/acp/skill-coexistence/dirac.md`。

## 上下文

审计基线版本为 0.5.24。MCP 现状｜new 接受后忽略三种声明，原生探针零连接。模型来源现状｜原生 DIRAC_MODEL，空挂载无需 MCP 补丁；挂载验受审私有入口；通用，驱动非 Unix 拒绝。原生工具现状｜普通及挂载保留原生目录，MCP 追加；无法落实的逐服务名单显式拒绝。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v1 注入逐轮 stdio 快照和执行器，单次批准、结果、退出清理；挂载 load/resume/fork 拒绝），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_dirac_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_dirac_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

实施提交 `6f8d6a00d`。

| Linux 宿主 | Dirac 与 SDK | 原生搜索 |
| --- | --- | --- |
| GNU x64 | `dirac-cli@0.5.24` 与 `@modelcontextprotocol/sdk@1.32.2`，平台共用 npm 归档 | Microsoft ripgrep-prebuilt `v13.0.0-10` 的 `x86_64-unknown-linux-musl` 静态 ELF |
| GNU ARM64 | 同上，发行包明确声明支持 Linux arm64 | 同一发行的 `aarch64-unknown-linux-musl` 静态 ELF |

- 已从 npm 核对两份归档的 SHA-512 完整性，均匹配现有 `managed_runtime/contracts.json`。从上游 Microsoft 发行下载两份 ripgrep 归档，分别核对 ELF 架构、归档和二进制摘要。URL、摘要和 Linux 入口均在 [linux-artifacts.json](../../patches/acp-model-source/dirac/linux-artifacts.json)。该版本与 Dirac 依赖的 `@vscode/ripgrep@1.15.9` 的 postinstall 目标吻合。受管理安装禁用 npm 生命周期脚本，因此将已验证的 `rg` 安装为 `dist/shiju-rg`，此入口只使用同目录固定搜索助手。无需执行供应商 postinstall，也无需依赖 PATH 中另一个 rg。[上游 Microsoft 发行](https://github.com/microsoft/ripgrep-prebuilt/releases/tag/v13.0.0-10)
- 任务 48 在 Linux GNU x64/ARM64 选择独立目录 `dirac-0.5.24-model-source-v3`。复用 `managed_runtime::{install, reviewed_agent_command, verify_reviewed_agent_executable}` 及现有 Dirac 安装分支。依赖 Node >=22.13.0 且 <25、npm、Python 3，并需访问 npm 与 Microsoft 的固定归档。使用 `npm --global=false --package-lock=true install --registry=https://registry.npmjs.org --ignore-scripts --save-exact --no-audit --no-fund dirac-cli@0.5.24 @modelcontextprotocol/sdk@1.32.2`，保留生产重试参数。核对两个 npm 主包的锁记录和现有 SRI。传递依赖仍经 npm 安装解析，当前仓库无 Dirac 全量传递 lock；两份直接归档摘要不能替代传递依赖校验。
- 先从固定原版应用现行 `dirac/apply.py` 生成 v2，保留 v2 输入摘要供叠加层校验。把 `dirac/apply-linux.py`、`dirac/linux-session.mjs` 与同目录 JSON 加入 `managed_runtime.rs::install` 嵌入资源。在生产安装脚本 Dirac 分支于 v2 之后运行 `python3 apply-linux.py <暂存运行时>/node_modules/dirac-cli`，以清单的 `files` 和架构对应的 `rg` 摘要替换 Linux 终验。重建现有 `node_modules/.bin/dirac` 到清单入口。固定 Linux 受管参数 `--acp`，经模型驱动追加 `--config <私有目录>`。
- Linux 发现、显式覆写、挂载及重启都使用同一个 v3 终验。注册表需拒绝入口、桥或 helper 任一缺失、摘要不符、异常符号链接或缺执行位。限制 Linux 显式覆写为当前经验证的受管入口，封闭通用祖先扫描接受 v2 的回退。用 `spawn_child_at` 在创建受管子进程前验证。常规 `cli_local` 保留现有 `dirac --acp` 与 `npx -y dirac-cli@0.5.24 --acp` 路径及原生选模。macOS arm64 保留同目录隔离的 v2 合约、两个 v2 文件原摘要及已有能力。
- 模型继续由 `drivers::dirac::configure` 通过权限为 0700 的 Unix 私有目录、`DIRAC_MODEL` 和回环 OpenAI Chat Completions 中继绑定。空挂载不需要 MCP 注入。技能候选目标为 `skill_mounts::dirac::supports_session_view_target`；任务 48 可用该判定加 v3、已审核权限和证据接到 `skill_mounts::validate` 对应分支。保留 `request.rs` 的 `session_model`、禁用工具及 `DenyAll` 限制。外挂视图由供应商原生目录扫描器按本会话追加，通过原生 `use_skill` 和 `read_file` 读取，相同名称拒绝。
- MCP 子集沿用 `acp-client::validate_mcp_contract`。仅接受绝对路径 stdio、无自定义 env、私有会话模型和逐次批准。每轮构造工具快照，保持供应商原生目录，执行结果按模型工具 ID 返回。v3 在发送执行前再查取消信号，并在关闭旧客户端后才释放该会话运行状态。同进程带挂载或显式携带挂载的 load/resume/fork 拒绝。Linux 修正了 resume 丢弃技能元数据、fork 对活跃源只检查新请求列表的问题。逐服务名单按现有宿主规则拒绝。HTTP/SSE 继续拒绝。
- 两个 Linux 目标的完整合同仍阻塞。固定 0.5.24 原生 ACP `newSession` 未消费声明，也没有独立可验的上游静态 MCP 执行路径。现有原生工具有本地自动审批分支（`requestAutoApprover`）；工作区内读和宿主批准外挂 MCP 的效果不能证明每次原生修改及命令都经过宿主约束。外挂 stdio 执行仍由适配器调用 SDK，尚未消费任务 4 的宿主租约；跨进程挂载限制已在下节补齐。继续开启完整能力须接入宿主执行前租约、完成宿主撤销接线，并取得源自同一实际调用的原生写入、命令与撤销证据。任务 48 保留对外门控，任务 49 分别登记两目标的源码子集和阻塞。历史 Apple Silicon 回执不计为 Linux 结果。
- 任务 48 在 `acceptance_support/mod.rs` 注册 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_dirac_tests;`。验收从真实网关 `/desktop/acp-install/dirac` 与 `/global/config/acp/defaults` 核对入口及篡改 fail-closed，然后用隔离网关和 Chat Completions 夹具验证原生搜索、两种技能读取、外挂 MCP 按一次性批准执行并关联续轮、拒绝及撤除的服务调用数。原路由应在安装或技能门控失败。现有直接探针增补 Linux 输入为空的同进程 resume/fork 拒绝断言，保留取消场景。网关撤除轮使用新会话，尚缺同一会话重启及原生修改/命令执行的 Linux 证据。

### 本轮验证

- 两架构均用各自官方归档静态生成 v2、应用 Linux 叠加并逐件核对最终摘要。两架构 JS 均通过 `node --check`，Python AST、定向 Rust 格式和限定路径 Git 空白检查通过。
- 未安装或运行供应商、编译、执行 Cargo 或 Docker。后续在两种非 root GNU 容器分别设置 `SHIJU_ACP_DIRAC_GATEWAY_BIN` 为当前网关绝对路径，运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_dirac_contract -- --ignored`。预期恰好命中 1 项，零失败。未取得红绿退出码、测试命中数或任何 Linux 运行结果。


### 继续排查与追加交付

追加提交 `6343e2994`。

- Linux v3 增加 `dist/linux-session.mjs`。建会话若带外挂技能或 MCP，在创建供应商会话及其历史前向驱动绑定、当前用户独占的 0700 私有状态目录写入 0600 空标记。标记仅含在文件名里的随机 ACP 会话 ID。load、经 load 的 resume 和 fork 都先检查标记，目录不安全或读取意外失败时拒绝。没有标记的原生会话继续使用供应商恢复。更换启动代际时，旧目录和历史不会进入新私有配置。技能视图、MCP 连接及批准都不持久化。普通 `cli_local` 和 Darwin v2 不使用新增标记。任务 48 随安装资源嵌入新文件，并在 Linux 的终验、发现和启动处核对清单全部三项脚本及每架构搜索助手。
- Linux stdio 执行使用发送给 ACP 一次性审批的配对实参快照，调用帧追加 `params._meta.shijuCallId`，与权限请求和工具事件共用同一 UUID。已检查固定 SDK 1.32.2 的 `Client.callTool` 直接序列化该参数。适配器先检查取消，再要求 `approve_once`，随后在 SDK 调用前复查取消。此改动提供宿主执行端所需的调用身份，当前没有连接到宿主授权消费。
- 核查 `RuntimeHost::prepare_stdio_mcp_launch`、`mimo_mcp::{prepare, Projection::approval_config, VtcodeApproval::consume}` 与 `acp-client::ManagedMcpApproval`。现行共用中继仅 Darwin arm64，底层使用 Darwin 沙箱，提供 HTTP 投影，Dirac 目前只接收 stdio。其消费接口按工具名、精确实参和轮次有效性判断，类型没有审批调用 ID。aionrs 的 Linux 补丁同样在 MCP 调用帧传 `shijuCallId`，但没有可直接复用给 Dirac 的宿主消费端。扩 Unix cfg、启用不受约束的 HTTP 或只凭同名同参批准都会留下权限缺口。需要 Linux 受限 stdio 转发端绑定调用 ID 与宿主现有租约，且把工具身份和原实参贯穿权限请求、执行和撤销。共享接线由任务 48 负责。
- 核查供应商原生审批后确认两个架构都仍有实质缺口。0.5.24 的 `Mdn` 会依据 YOLO、自动批准、持久命令和路径规则跳过宿主，原生工具执行管线还可执行项目 hooks 和子代理。只关闭 YOLO 或调整默认配置无法覆盖其它入口，切换只读工具白名单会排除验收要求的原生编辑和命令。Dirac 上游原生静态 MCP 缺独立注册及执行接口，现有补丁只追加宿主传入的会话工具。完整合同继续标记已阻塞，等待原生静态 MCP 能力和受宿主约束的原生写入/命令执行边界。已交付的制品、选模、限定 stdio、技能及防重入子集可供 48 接线和 49 入账。两个架构的发布门控保持。
- 直接 ACP 探针增补重启两类带挂载会话及普通会话对照，Linux 环境使用合成启动引用。已重新对 x64 和 ARM64 从上游归档生成补丁产物，核对三份脚本和搜索助手的 SHA-256，Node 静态语法、Python AST 和限定差异检查通过。未运行探针、编译、测试、安装或启动供应商。
