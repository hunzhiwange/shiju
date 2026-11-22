# 任务 12 适配 auggie

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 auggie 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/auggie.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/auggie.rs`、`patches/acp-model-source/auggie`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_auggie_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/auggie.md`、`docs/dev/acp/skill-coexistence/auggie.md`。

## 上下文

审计基线版本为 0.37.1。MCP 现状｜受管复用原生 stdio，同 ID 续轮；未修改包同版闭环待查。模型来源现状｜受审；通用，Windows 私有历史 ACL 拒绝，Linux 有本地入口。原生工具现状｜默认保留，历史同轮 46 次图谱及 4 次原生读取；私有 removedTools 才清空 20 项，试验转发器独立缩服务。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（私有模型桥、目录校验、远程索引权限隔离），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_auggie_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_auggie_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 制品与入口 | 运行状态 |
| --- | --- | --- |
| x86_64 | npm `@augmentcode/auggie@0.37.1`，`augment.mjs --acp` | 受限启动缺 Linux 沙箱，未运行 |
| aarch64 | 同一 JS 归档、入口和摘要，主包无架构过滤字段 | 受限启动缺 Linux 沙箱，未运行 |

- 核对了[固定版元数据](https://registry.npmjs.org/@augmentcode/auggie/0.37.1)和[实际归档](https://registry.npmjs.org/@augmentcode/auggie/-/auggie-0.37.1.tgz)。SRI、原入口 SHA-256 及应用现有 v2 补丁后的双文件摘要均匹配 `managed_runtime/contracts.json`。归档只有包元数据、README、LICENSE 和 JS 入口。固定版 LICENSE 明列修改限制，发布前须核实适用许可。
- 依赖 Node.js 22+（上游 README，包元数据为 >=20）、npm、Python 3。唯一可选依赖 `node-pty@1.2.0-beta.14` 由入口按需加载，未验证两架构的 PTY。安装沿用 `acp_agent_registry::install_builtin_agent` 和 `managed_runtime/install.py`，使用 `npm install --registry=https://registry.npmjs.org --ignore-scripts --prefer-offline --fetch-timeout=20000 --fetch-retries=2 --fetch-retry-mintimeout=1000 --fetch-retry-maxtimeout=5000 --no-audit --no-fund --save-exact @augmentcode/auggie@0.37.1`。后续独立 Linux 受限修订可使用 `--global=false --package-lock=true --omit=optional` 并验证无需 PTY，保持归档、包身份及补丁前后摘要校验。
- 复用 `managed_runtime::{reviewed_agent_command,verify_reviewed_agent_executable}` 和 `auggie-0.37.1-model-source-v2/node_modules/.bin/auggie`；受限路径须再走 `verify_auggie_restricted_executable` 拒绝 v1。现有 `prepare_profile` 和 `drivers::auggie::configure` 绑定回环 Chat Completions、明确模型、私有令牌及远程索引拒绝。本轮在 Linux 复用宿主祖先校验与严格 0700 历史目录权限，沿用原路径、会话引用及 Apple Silicon 行为。普通 `cli_local` 沿用稳定 key、原生命令发现及既有 npx 回退。
- 任务 48 保留 `request.rs` 的 `restricted_auggie`、`skill_mounts/mod.rs::validate` 和 `acp-client` 的平台门控。后续受限入口为 `skill_mounts::auggie::sandbox` 与 `AuggieSandbox::command`。现有 Darwin 沙箱限定工作区和选中技能只读、私有 home/history 可写及进程和网络。仓库现有 Landlock 配置允许工作区写入且无中继端口约束，Bubblewrap 配置隔离中继网络并挂载宿主 `/tmp`。两个目标仍需经审查的等价 Linux 隔离，以及真实运行证据。
- 外挂 stdio MCP 候选复用 `RuntimeHost::prepare_stdio_mcp_launch`、`mimo_mcp::prepare`、`shiju.auggie.toolPolicy` 和 `mcp__auggie__工具_服务` 宿主审批键。现有受限 v2 保留原生 `view`、原生技能和外挂技能，跳过项目及插件原生 MCP。仍需受审的项目原生服务宿主投影与来源区分，以及逐调用审批、原生写入及命令、拒绝、取消与同会话恢复后的撤权证据。任务 49 对两个架构分别登记缺口。
- 任务 48 在 `acceptance_support/mod.rs` 注册 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_auggie_tests;`。真实网关夹具验证固定制品、实际 Chat Completions 路径及选模、原生和外挂技能正文及附件、同 ID 续轮、批准后一次下游调用及拒绝和撤除后零新增。旧路由在受限网关基线遇平台门控，旧历史权限检查会接受 01700。撤除轮使用新网关会话，同会话恢复和项目原生 MCP 尚无 Linux 正例。
- 静态通过归档、原始及补丁后摘要核对，JS 语法、Python AST、局部 Rust 格式解析及 Git 空白检查。未安装或启动供应商运行时，未执行 Cargo 或 Docker。任务 48 接线后，原命令预期命中 1 项历史验证、1 项忽略。在两个非 root Linux Docker 目标分别提供 `SHIJU_ACP_AUGGIE_GATEWAY_BIN`，用 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_auggie_contract -- --ignored` 执行网关验收，预期命中 1 项。当前无运行退出码及命中数。

### 续查与逐调用身份交付

- 新增 `patches/acp-model-source/auggie/apply-linux.py`。固定上游的 `onToolApprovalRequired` 与原生 MCP 执行入口均持有相同原始 tool-use ID，但 SDK 的 `tools/call` 参数原先只有工具和实参。叠加层仅在 Linux 受限启动中向宿主发送 `params._meta["shiju.auggie.toolCallId"]`，缺 ID 或不在宿主的工具名单时在供应商侧拒绝。SDK 的 `callTool` 将完整参数交给 `request`。普通 CLI 与 Darwin 行为保持原样。对临时归档副本通过两次补丁应用、结果 SHA-256、Node 静态语法及重复应用拒绝检查。
- 两目标候选入口共用 `augment.mjs` SHA-256 `3ddbaf2938bba2d28391cb4169d6d201e7daea0317b780794a1f1351cfff87c5`，`session-model.mjs` 继续使用 v2 摘要。叠加脚本 SHA-256 为 `32a43455af5d8caab63909c171b08a20ae41a71005188f046da223ea2d75886c`。宿主条件具备后，任务 48 可在现行补丁与双摘要验证完成后应用此层，分别为 Linux x86_64/aarch64 选用独立 v3 修订，并同步受管发现、启动、恢复及验收夹具摘要。Apple Silicon 继续使用现有 v2。
- 当前 `event_client.rs` 在活动提示内检查 ACP 调用 ID，但 `ManagedMcpApproval` 只传工具与参数，`mimo_mcp.rs::VtcodeApproval` 也仅按工具与参数消费。任务 48 需要让 Auggie 审批发放携带 ACP 原始 ID，并让 `mimo_mcp.rs::dispatch` 在转发前消费与 ID、工具、原始实参及活动提示匹配的一次批准。对缺失或错配 ID 保持拒绝，并在宿主消费后移除私有元数据。供应商的审批回调通过 `NK` 生成展示实参；出现与实际调用参数不一致时继续拒绝，待双方证明同一原始参数绑定。当前 Linux 叠加层仍缺宿主消费接线，保留发布和运行门控。
- Linux 两目标还需要两处受审宿主隔离。`AuggieSandbox::command` 约束供应商进程，`mimo_mcp.rs::prepare` 又用 `/usr/bin/sandbox-exec` 隔离 MCP 子进程。即使供应商主进程改用 Linux 启动器，外挂 MCP 仍会在第二处启动失败。Node 权限和模型工具名单不能提供单端口中继及全部文件系统隔离。现成 Landlock 与 Bubblewrap 包装未提供同等合同。在宿主提供双进程隔离、上述审批消费和获准项目原生 MCP 投影前，两架构均保留已阻塞。实际运行验证仍受本轮静态限制。
- 复核任务 48 已通过 `acceptance_support/linux_platform_tests.rs` 注册 Auggie 验收。当前共享 `mimo_mcp.rs::prepare` 在启动 stdio 前还显式拒绝非 Darwin arm64。任务 41 的 Qwen 随包 Landlock 助手只用于供应商工具执行，其公开合同要求网络开放，无法替代 Auggie 的独占回环或宿主 MCP 的网络关闭策略。任务 39 的 Poolside 容器同样只覆盖供应商工具执行，宿主侧发现与 stdio 仍依赖 Seatbelt。两者均无可直接接入 Auggie 的固定宿主启动器。[Qwen 沙箱合同](https://github.com/QwenLM/qwen-code/blob/main/docs/users/features/sandbox.md)
