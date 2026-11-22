# 任务 34 适配 nova

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 nova 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/nova.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/nova.rs`、`patches/acp-model-source/nova`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_nova_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/nova.md`、`docs/dev/acp/skill-coexistence/nova.md`。

## 上下文

审计基线版本为 1.1.65。MCP 现状｜owner approval 下 new 拒三种声明；旧扩展缺真实审批且目录可能残留。模型来源现状｜原版有模型配置；受审模型表与私有状态；通用。原生工具现状｜原生按权限共存，历史挂载 5 次图谱与 2 次官方；显式会话名单及服务 allow/deny 限制外部目录。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v2 用独占连接的扩展和只读沙箱提供目录、单次批准、结果及撤销），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_nova_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_nova_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| GNU 目标 | 固定制品和实际 ACP 候选入口 | 本轮证据 |
| --- | --- | --- |
| x86_64 | `@compass-ai/nova@1.1.65`，`scripts/bin-wrapper.mjs acp` 固定到受管 `dist/cli.js` | 归档与补丁产物摘要已核对。Docker `linux/amd64` 未运行。 |
| aarch64 | 同一归档及 JS 入口，无需 Nova SEA 平台二进制 | 归档与补丁产物摘要已核对。Docker `linux/arm64` 未运行。 |

- [固定 npm 归档](https://registry.npmjs.org/@compass-ai/nova/-/nova-1.1.65.tgz) 实际 SHA-512 SRI 匹配现存 v3 合约。1.1.65 要求 Node >=22.13.0，无顶层可选平台包。上游 wrapper 列出两个 Linux 平台包名，但 npm 对两者的 1.1.65 版本查询均返回 404。wrapper 会优先运行 Node 解析到的 SEA，存在脱离受审 JS 的分支。Linux 覆盖层固定 JS 子进程并保留该 wrapper 的 stdio 和环境语义。Node 的 `--no-addons` 沿用现有驱动，npm 传递依赖仍含浮动版本，须核对锁及实际启动。
- [Linux 覆盖层](../../patches/acp-model-source/nova/apply_linux.py) 在完整 v3 产物上应用。它固定入口并复用 v3 独占连接和按活动回合暴露技能目录。Linux 受管模型的原生和外挂技能均采用 inline，禁止 shell 替换和 fork 绕出权限循环。普通 CLI 发现保持现有路径。Apple Silicon 继续用现有 v3 入口及摘要。
- 任务 48 安装和身份输入。在 `acp-agent-registry::managed_runtime` 仅对 GNU x86_64/aarch64 使用 [linux.json](../../patches/acp-model-source/nova/linux.json) 确定 `installed_directory` 为 v4，并合并其中三个 SHA-256 到原六项合约。显式安装用 Node >=22.13.0、npm 和 Python 3，在私有前缀运行 `npm --global=false --package-lock=true install --registry=https://registry.npmjs.org --ignore-scripts --no-audit --no-fund --save-exact @compass-ai/nova@1.1.65`，校验 lock 的主包 SRI。按现有 `managed_runtime/install.py` 的 Nova 分支运行 v3 apply、复制两个辅助文件、核验全套 v3 摘要，再执行 `python3 nova/apply_linux.py <node_modules/@compass-ai/nova>`，核验合并后的全套 v4 摘要。将覆盖脚本和清单列入 `managed_runtime::install` 的嵌入素材；`reviewed_agent_command` 与 `verify_reviewed_agent_executable` 全部 Linux 入口（含显式覆写和祖先目录校验）使用 v4 摘要和固定参数 `["acp"]`，避免受管会话回落 v3。
- 任务 48 路由输入。选模复用 `model_source::prepare_profile`、`drivers::source_protocol` 和 `drivers::nova::configure` 的 Anthropic Messages 中继、明确选模及私有状态。技能的平台判定为 `skill_mounts::nova::supports_session_view_target()`。对外能力继续沿现有 `model_source/drivers/mod.rs::capabilities`、`skill_mounts/mod.rs::validate` 门控，待 Linux 目标证据后再接通。新建和恢复使用 `acp-client::client::protocol` 的 owner approval 协商，恢复走 `session/load` 并传当前技能元数据。保留 `request.rs::do_stream_request_acp` 对选模、禁用工具和 DenyAll 的检查。
- 两个架构的实质阻塞相同。外挂 MCP 的 `session-mcp.mjs::dispatchMcp` 仍在 Linux 拒绝；其 Python stdio helper 使用 `cortex-code/mcp_bridge.py::Stdio.start` 的绝对 `/usr/bin/sandbox-exec`。需要 Linux 下等价的只读、无网络、受控进程生命周期执行边界及获批后分发；现有 Nova 线路不能直接复用宿主 HTTP MCP 中继。保留声明、调用和恢复的默认拒绝。原生 ACP owner approval、原生读工具及挂载技能有上述源码路径和 Darwin 历史证据，Linux 尚缺原生写入、命令、供应商静态 MCP 的独立执行／拒绝证据，也没有同会话撤权、取消和恢复回执。未将四个独立网关会话视为同会话恢复。
- 任务 48 测试接线。在 `acceptance_support/mod.rs` 按 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]` 注册 `mod platform_nova_tests;`。后续在两个非 root Docker 目标各使用 `SHIJU_ACP_NOVA_GATEWAY_BIN`、`ACP_NOVA_REVIEWED_PREFIX` 和 `ACP_NOVA_TEST_CONSENT` 指向本目标网关、私有 v4 安装和已授权的测试同意文件。运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_nova_contract -- --ignored --nocapture`，预期命中一个显式忽略用例。网关夹具核验 Messages 协议、原生与所选技能目录和附件、同调用 ID 的 MCP 续轮、批准一次及拒绝和撤除零新增服务审计。旧 Linux 路由会在制品或模型平台门控失败，接通 v4 后 MCP 挂载仍在 Linux 拒绝。
- 静态核对 v4 全部六项摘要，生成的 JS 三文件通过 Node 语法检查，Python 源码通过 AST 解析，修改的 Rust 文件通过 rustfmt 解析。未安装运行时、未启动供应商代码、未运行 Cargo、网关测试或两个 Docker 目标。
- 代码提交为 `b909a8c9f`，限定路径的工作区和暂存区差异检查通过。

### 续查与 Linux v5 宿主投影候选

- GNU x86_64 与 ARM64 共用 [v5 清单](../../patches/acp-model-source/nova/linux.json)。本节取代上文 v4 的 Linux 安装修订及外挂客户端阻塞。仍从已校验的 v3 应用 `nova/apply_linux.py`，将 `nova/linux-mcp.mjs` 同目录嵌入安装素材，最终校验四项 Linux 摘要与三项继承摘要。安装、发现、显式覆写、启动和恢复统一使用 `nova-1.1.65-model-source-v5`。普通 `cli_local` 与 Apple Silicon 沿用原路径。
- v5 复用 Nova 已有 `compass.host-tools.v1`、独占连接、会话及活动回合校验。在 `session/new` 和 `session/load` 对整个选集预校验，只接受当前模型中继同源的 `/shiju-vtcode-mcp/<service>`、独立的宿主投影 bearer 和明确的 owner approval。拒绝原始 stdio、SSE、外部 HTTP、重定向、共享模型令牌和缺少启动租约的声明。Linux 扩展移除了本地子进程执行入口。目录沿用已解析工具名单和 Nova 的 `mcp_shiju_<service>__<tool>` 工具名。需要当前回合中同 ID 的 `allow_once` 才会转发原参数及 `params._meta["shiju.nova.toolCallId"]`。取消、拒绝或服务失败时无本地回退。服务端还须验证并消费该授权，不能依据客户端的 HTTP 凭据直接执行。
- 任务 48 的宿主接线。复用 `request.rs::do_stream_request_acp` 的 `Host::prepare_stdio_mcp_launch` 与 `RuntimeHost::prepare_stdio_mcp_launch` 的 `mimo_mcp::Projection`。选集与 `shiju.nova.toolPolicy` 的 allow/deny 在下游列表发布前处理，保留现有 `session_meta.rs` 的选中工具名单。由宿主签发上述同源 HTTP 定义。仅在 GNU 两目标已验证 v5、用户允许运行工具且存在活动会话时，让 `acp-client::client::protocol::validate_mcp_contract` 接受由宿主生成的投影，继续拒绝外部 HTTP 和原始 stdio。Apple Silicon 的受审 stdio 分支单独保留。将 `event_client.rs` 的显式单次回调扩展到 Nova 的精确投影工具身份。签发和消费须同时匹配启动与提示轮租约、原调用 ID、服务及工具、完整参数。恢复重建投影和当前选集。撤权／取消使旧令牌、已发审批及旧工作进程失效。
- 两个架构的外挂 stdio 仍缺共享 Linux 隔离执行边界及授权消费合同。现有 `mimo_mcp::prepare` 限 Darwin arm64，依赖 `/usr/bin/sandbox-exec`，未注册 Nova。现有回调和 `VtcodeApproval::consume` 只绑定工具名、参数和有效性，不核对调用 ID。通用 Bubblewrap 将宿主 `/tmp` 暴露给子进程，也未实现敏感目录过滤和进程派生约束。现有 Landlock 文件约束、直接 stdio、HTTP 与供应商原生 MCP 均无法提供同等限制。保留公开门控及默认拒绝。若隔离和授权由共享层接通，Linux 客户端已有完整投影调用路径。
- 两个架构的供应商原生 MCP 仍有协议阻塞。固定 1.1.65 的 owner approval 分支禁止新建／载入原生 MCP 声明、跳过 Nova 集成初始化，并在原生直连 MCP 执行点明确拒绝。去掉该门控会失去逐次审批边界。当前没有在 owner approval 下枚举 Nova 原生静态服务并由宿主独立校验、分发、撤销的上游协议。需供应商提供此协议，或另行交付等价的宿主静态配置迁移与隔离执行入口。原生文件写入和命令走供应商 owner approval 的单次回调，仍待独立 Linux 验收。
- 验收继续使用 `acp_platform_nova_contract -- --ignored --nocapture`，前缀换成私有 v5 npm 安装。Linux 夹具校验 npm 完整性、包名及版本、真实 `.bin/nova` 与固定入口，并将制品复制到网关私有的 v5 发现目录。修复平台和同意文件变量重名导致报告无法生成的问题。新增 `node --test patches/acp-model-source/nova/linux-mcp.test.mjs` 后续入口，覆盖非法来源、审批调用 ID 传递、拒绝与取消后零重试。当前只应用补丁于已核验归档的临时副本，核对七项安装摘要、Node 语法、Python AST、Rust 格式及定向 Git 差异。未运行测试、Gateway、Nova、Docker 或 Cargo。

- Linux v5 投影候选与验收修订提交为 `307a1f533`。
