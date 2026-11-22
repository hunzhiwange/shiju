# 任务 40 适配 qoder

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 qoder 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/qoder.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/qoder.rs`、`crates/libs/acp-access-policy/src/access_policy/qoder.rs`、`patches/acp-model-source/qoder`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_qoder_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/qoder.md`、`docs/dev/acp/skill-coexistence/qoder.md`。

## 上下文

审计基线版本为 1.1.88。MCP 现状｜受管本地框架能用 mcp_list/get/call；原版供应商框架待验证。模型来源现状｜受审私有绑定；通用，其他平台浮动 npm。原生工具现状｜普通及挂载无额外官方目录裁剪；网络工具仍受回环限制；历史三项转发器仅限验收。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（绑定私有模型及本地 ACP，关 mcp.lazyLoad 以直出，保留原生权限），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_qoder_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_qoder_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

实现提交 `b204e7eb3`。本编号文件保留给任务 48、49。

- 两个目标共用 [npm 原包](https://registry.npmjs.org/@qoder-ai%2fqodercli/1.1.88) `@qoder-ai/qodercli@1.1.88` 和 `bundle/qodercli.js`，原 bundle SHA-256 为 `1858e47c…`。固定合约 [linux.json](../../patches/acp-model-source/qoder/linux.json) 记录 SRI、打补丁后的文件摘要、包元数据及两个独立 helper。GNU x64 使用 `@qoder-ai/qodercli-ripgrep-linux-x64@1.1.88`，ELF x86-64、静态 PIE，SHA-256 `19390667…`。GNU ARM64 使用 `@qoder-ai/qodercli-ripgrep-linux-arm64@1.1.88`，ELF AArch64、静态链接，SHA-256 `e152ea68…`。两个 helper 均来自上游可选依赖。已分别核对三份归档内容及 SRI。上游独立安装器的制品不用于此处，既有受管补丁以 npm JS bundle 为锚点。
- 入口复用私有 `P7e/ahe`、原生会话循环及原权限模式。Linux 覆盖层只将绑定限定到 GNU x64/ARM64，并将受管 Glob/Grep 解析固定到同安装树的架构 helper。闭合回环、私有文件权限、模型引用重校验、原生 ACL、技能元数据限制和模型选择保留。沿 `model_source::prepare_profile`、`drivers::launch_route`、`drivers::qoder::configure` 走 `POST /v1/chat/completions`，`mcp.lazyLoad=false`。原生用户技能来自 `~/.agents/skills`，外挂技能沿 `request.rs` 的 `skill_mounts::qoder::publish` 放入请求私有视图。现有 Apple Silicon v2 摘要、安装分支及普通 `cli_local` 的 `qodercli --acp`／npm 发现保持原路径。
- 任务 48 的安装接线在 `managed_runtime.rs::install` 注册 Linux GNU x64/ARM64 v3 修订 `qoder-1.1.88-model-source-v3`，发布资产 `qoder/apply-linux.py` 和 `qoder/linux.json`。`managed_runtime/install.py` 使用 Python 3、Node ≥20、npm、glibc。生产分支可在私有暂存目录使用 `npm install --global=false --package-lock=true --include=optional --ignore-scripts --save-exact @qoder-ai/qodercli@1.1.88` 并沿用既有 registry 参数。检查主包锁记录及版本，先应用 `qoder/apply.py` 并校验 v2 文件，再用 `python3 <assets>/qoder/apply-linux.py <runtime>/node_modules/@qoder-ai/qodercli` 校验当前架构 helper 的锁项、路径、执行位和摘要，应用 Linux 层并用 Linux 合约复核最终文件。不要用 Darwin 摘要验证覆盖后的目录。普通未受管安装仍走原有 npm 路径。传递依赖树尚未固定，包括 sharp 的版本范围和平台可选二进制，不能据此声称完整依赖图已审核。
- 在 `managed_runtime.rs` 对上述 GNU 目标用 `#[path = "managed_runtime/qoder.rs"] mod qoder;` 接入 `qoder::verify(directory)`。安装完成及已存在目标的检查、`reviewed_agent_command`、`verify_reviewed_agent_executable` 与恢复时统一使用 Linux v3 验证器。进入 Linux Qoder 分支后仅接受已验证的绝对 `node_modules/@qoder-ai/qodercli/bundle/qodercli.js` 与 `["--acp"]`，运行时根目录可以先解析宿主路径，但拒绝修订目录、包内容及 helper 重定向。显式覆写须复用此验证，失败直接返回且不继续尝试 v2；驱动负责追加私有绑定参数。安装身份和发现可独立接线。保持 `model_source::drivers::capabilities` 和 `skill_mounts::validate` 的 Linux 对外门控，待两种目标各自取得能力证据再发布。
- 验收在 `acceptance_support/mod.rs` 对 Linux GNU x64/ARM64 用 `cfg(test)` 注册 `platform_qoder_tests`。通过生产网关和仓库 Qoder 夹具测试选模回读、实际 HTTP 路径、原生 `Read` 和 `Skill`、两类技能、外挂 stdio MCP、同 ID 续轮，以及经批准、拒绝和撤除时服务调用审计分别为 1、0、0。旧源码的首个失败点为 Linux v3 摘要不匹配，其后为挂载门控。撤除轮使用新网关会话，现有脚本审计 `tools/call`；同会话恢复时的撤权、发现和重放还需补验。原版供应商 ACP 登录门禁保持，普通供应商模型与静态服务需另用获授权的独立账号验证。
- 两种架构的完整受管合同仍阻塞。原生工作区读取可由 Qoder 默认 ACL 直接执行，随后各自的读、写、终端及静态 MCP 没有经任务 4 审核的统一执行前边界。现有一次批准由 Qoder 原生 ACP 控制消费，尚无证据把该批准和任务 4 的原调用 ID、来源、参数、撤销租约绑定到 Linux 上的实际工具执行。现有宿主中继只覆盖受选服务，stdio 不能替代静态、HTTP/SSE 或供应商内置网络工具审批。原生权限同步到 `default` 也不等价于逐调用宿主授权。补齐上述绑定之前继续拒绝完整能力，任务 49 对 x64 和 ARM64 都登记该限制。
- 本轮通过原包与两架构 helper SRI、ELF 架构、原补丁及 Linux 层产物摘要、生成 JS 语法、Python AST、定向 rustfmt 及局部 Git 差异检查。补丁应用仅在临时解包素材中静态复现，未安装或启动 Qoder，未运行 Cargo、Docker、网关或真实模型。获授权并完成接线后，两架构 Docker 分别设置 `SHIJU_ACP_QODER_GATEWAY_BIN` 为当前 Linux 网关绝对路径，运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_qoder_contract -- --ignored`，预期命中 1 项、零失败。暂无红绿退出码或运行命中数。

## 继续排查与追加交付

追加提交 `67d5e3d52`。

- 受管 Linux 层现在从 Qoder 原生调用执行器 `Oot` 的 `Mki({callId:s.callId,...})` 上下文获取调用 ID，并在挂载 MCP 的 `tools/call.params._meta.shijuCallId` 传递。缺少非空 ID 时在实际 MCP 请求前拒绝。ACP 审批端原有 `toolCallId:i.toolUseId`、`rawInput:i.input` 及 `_meta.qoder.toolName` 可用于宿主关联。Qoder 在执行端通过 `NNi(t.args,this.propertyKeyMap)` 映射参数名称。任务 48 若接入宿主中继，须核对或拒绝不能无歧义映射的实参，并将该字段绑定到原 ACP 审批、来源及中继当前租约。
- Linux 受管入口只在模块私有变量中保留模型绑定路径和启动引用，启动后子进程及 MCP 环境变量模板拿不到这两个变量。已触发的原生 MCP 审批仅提供单次批准及拒绝，拒绝未知或持久选项，并保留原 ACL 的拒绝。绕开 ACP 审批的原生路径和同一系统用户对子进程的文件访问仍需要宿主执行边界。Apple Silicon v2 与普通入口不采用这些改动。
- Linux 网关夹具的 stdio 服务在真正读取附件之前校验 `shijuCallId` 与模型夹具创建的调用 ID 相同，并检查两个绑定变量没有经继承或模板插值暴露。对缺少 ID、错误 ID 或泄露给服务环境的固定版，服务不会写调用回执。服务仍复用仓库的真实读盘及调用审计逻辑。
- 已排查现有替代方案。通用 `ManagedMcpApproval` 和 `mimo_mcp.rs::VtcodeApproval` 的授权和消费只匹配工具名、参数、超时和提示有效性，无法核对执行帧中的调用 ID，也不能监管 Qoder 自己启动的静态 MCP。将现有动态 stdio 改接 HTTP 中继还会撞到 Qoder 受管模块只允许模型端口的传输约束。适配器内按同名工具存一份批准无法证明同工具同参并发、撤销、重启时消费的是原授权。将 native ACL 固定为 `default` 或关闭全部工具堵不住已允许的工作区副作用，或无法交付要求的原生能力。
- 两个 GNU 架构的完整受管模式仍为实现阻塞。继续需要共享宿主提供按调用 ID 消费、按原提示撤销的 MCP 执行协议，以及可拦截原生静态 MCP 和本地操作的执行边界。面向正式发布的 Linux 门控保留。固定安装、入口身份和验收测试可以独立接线，普通供应商登录仍独立。任务 48 使用现行 `linux.json` 的最终摘要，先安装校验，再在具备执行边界后开放受管候选验证。x64 和 ARM64 均需另行运行网关验收。
- 离线对两份架构素材重新应用两层补丁，最终 SHA-256、生成 JS 语法、Python AST 和限定差异检查通过。未启动 Qoder、运行测试或调用 Cargo。
