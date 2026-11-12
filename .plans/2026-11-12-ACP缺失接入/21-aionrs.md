# 任务 21: 对接 aionrs

批次：【批次 21】 依赖批次 20

线性顺序：任务 21；本批只有此任务，与其他批次串行，公共注册与分派文件不可并行写入。

状态：未开始

目的：将 aionrs Rust CLI 通过最小 JSON Stream→ACP 桥接接入现有智能体目录，显示名称和稳定 backend key 均为 aionrs。

来源任务：用户明确追加 aionrs 接入；替代此前排除 aionrs 的范围约定，不使用 shiju-agent-cli 名称。它不同于任务 20 的 Pi/Node.js 包。

预计会话范围：一个引擎的外部进程协议桥接；复用现有 SessionBackend/serve_with_io 和 ACP 客户端，不引入 aionrs crate 到 workspace、不移植其引擎、不新增运行引擎枚举。先通过常驻进程、审批与取消的闭环验证，之后才开放目录。复杂可选能力保持明确支持边界。

## 来源与可行性

- 官方仓库：https://github.com/iOfficeAI/aionrs 。版本发行：https://github.com/iOfficeAI/aionrs/releases 。协议：https://github.com/iOfficeAI/aionrs/blob/main/docs/json-stream-protocol.md 。
- 可复现基线 v0.2.11，revision `8e61a90329fa9f67c4fdf7e97fe02c24dba33f75`，来自参考项目 `packages/desktop/shiju/backend/manifest.json`；对应本地源码包 `packages/desktop/shiju/sources/aionrs-8e61a90329fa9f67c4fdf7e97fe02c24dba33f75.tar.gz`。
- 已读固定源码包 `crates/aion-cli/src/cli.rs`、`crates/aion-cli/Cargo.toml`、`docs/json-stream-protocol.md`。二进制名 aionrs，启动参数 `--json-stream`，使用独立 NDJSON 协议；不能伪造 `aionrs acp` 或将原生 stdout 直接交给 ACP JSON-RPC 客户端。
- 来源协议覆盖 ready、message、text_delta、stream_end、tool_request/tool_approve/tool_deny、cancel、set_mode、set_config、动态 MCP 及会话恢复。协议说明证明可设计桥接，不代表当前 ShiJu 已真实运行成功。
- 优先固定上游版本和独立子进程；参考项目补丁仅作调查材料，不能假设未打补丁的官方二进制具备补丁能力。若官方行为不足，明确记录证据和必要最小修补，不引入整套 AionCore。

## 文件

- 新建：`crates/vw-acp/src/bin/aionrs_acp.rs`，只负责参数、标准输入输出与桥接服务启动；`crates/vw-acp/Cargo.toml` 显式注册 `aionrs-acp` 二进制，`test = false`。
- 新建：`crates/vw-acp/src/aionrs/mod.rs`、`protocol.rs`、`process.rs`、`backend.rs`，分别负责组织、强类型 NDJSON、进程/读写和 SessionBackend 实现；对应测试独立在 `protocol_tests.rs`、`process_tests.rs`、`backend_tests.rs`。
- 修改：`crates/vw-acp/src/lib.rs`，导出该桥接二进制实际所需的最小入口；复用 `server::SessionBackend`、`BackendEvent`、`BackendPermissionRequest`、`serve_with_io`。
- 修改：`crates/vw-acp/src/agent_registry.rs`、`crates/vw-config-types/src/acp_install.rs` 和对应独立测试，注册桥接入口与固定安装契约；本地 aionrs 主 CLI 与桥接程序分别探测。
- 修改：`crates/vw-acp/src/install.rs`、`crates/vw-acp/src/managed_runtime.rs` 仅在确需受控来源构建/部署分支时修改；安装源码或脚本调用 Cargo 也遵守共享槽位规则。
- 新建：`crates/vw-acp/tests/onboarding_aionrs.rs`；修改：`crates/vw-agent/src/agent/catalog_tests.rs`，验证 `acp-aionrs` 官方预设、runtime backend 和显示名称。
- 按已验证能力修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs`，新增 `aionrs.rs`、`aionrs_tests.rs`；模型选择通过既有中继和会话私有环境配置。
- 新建证据：`docs/dev/evidence/acp-onboarding-aionrs.md`，记录固定版本、来源、协议帧形状、拒绝与取消、真实两轮任务、平台和未支持能力。

## 上下文

- 先读现有 Codex/Claude 启动、模型来源与审批链路，以及 vw-acp/src/server 下的后端 trait、事件和权限测试。aionrs 是新增外部引擎，通过 ACP 桥接复用产品链路，不替换当前 Rust 原生引擎。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-20`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–20。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 20 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 测试计划

- crate/features：vw-acp、vw-config-types、vw-agent 使用默认 features；workspace clippy 使用所有 features。外部 aionrs 基线使用其 Cargo.toml 默认 features，不新增 ShiJu feature。
- 行为：ACP 客户端可通过桥接控制一个长期存活的 aionrs 会话，流式输出归属正确、拒绝阻止副作用、取消后无越轮事件。
- 失败测试：`onboarding_aionrs.rs` 中 `acp_onboarding_aionrs_registry_contract`、`acp_onboarding_aionrs_two_turn_session`、`acp_onboarding_aionrs_denial_prevents_write`、`acp_onboarding_aionrs_cancel_keeps_turn_identity`；配置/目录测试也用 `acp_onboarding_aionrs` 前缀。
- 失败验证：`cargo test -p vw-acp --test onboarding_aionrs -- --test-threads=1`；先让注册缺失断言失败，再逐项添加桥接行为测试，不能把未实现 API 的编译错误当红灯证据。
- 通过验证：上述命令至少 4 个有效测试；`cargo test -p vw-acp --lib aionrs -- --test-threads=1`；`cargo test -p vw-config-types --lib acp_onboarding_aionrs -- --test-threads=1`；`cargo test -p vw-agent --lib acp_onboarding_aionrs -- --test-threads=1`。每个目标有效测试至少 1 个，不接受零测试。
- 模拟边界：生产桥接+真实 ACP 编解码对接可控的外部 NDJSON 子进程，覆盖分片、错误、审批与取消；模型网络使用受控本地 HTTP 服务。最后使用实际 aionrs 固定二进制和同一本地模型服务证明真实工具写文件与拒绝，无需借用用户第三方账号。

## 步骤

1. 阅读当前任务、只读 plan.md、差异清单、前一批次结果与现有工作区差异。重新核对固定源码和官方地址，分清 aionrs、AionCore、shiju-pi-cli，记录版本和许可证。
2. 先添加注册失败测试。审阅 aionrs 真实 NDJSON 输入循环、取消处理、配置合并和权限 allow-list；利用实际二进制与本地受控模型做最小两轮/审批/取消协议探针。若官方实现不能安全关闭权限旁路，先解决必要相邻问题并保留证据，不能先发布目录入口。
3. 实现最小桥接：一个宿主会话绑定一个子进程；等待 ready 后接收消息；映射文本、思考、工具状态、usage、终止和错误。严格检查 session/msg/call id，处理超时、异常帧、EOF 和 stderr；不把 stderr 原文或工具敏感输入落日志。
4. 权限从宿主当前授权解析；tool_request 转为一次性允许/拒绝，拒绝和取消发 tool_deny，关闭通道默认拒绝。核查原配置 allow-list、auto_approve、set_mode 可能绕过审批的路径；会话私有配置和启动策略不能继承更宽权限。默认不传 auto-approve、不设置 yolo；未经版本与协议回读证明不宣称 Native 完全访问。
5. 映射 cancel 到当前 aionrs 轮次并等终止，必要时结束子进程树；重复取消幂等，后续轮次不能收到旧帧。新/恢复会话检查租户、项目、工作目录和会话归属。恢复历史只能来自真实保存会话，不能把摘要注入伪装为完整恢复；不能安全实现的可选能力明确拒绝并不广告支持。
6. 安装与发现分开处理：本地发现 aionrs 主 CLI；桥接程序从当前发行包/受控安装目录解析，不写死开发机路径。官方发行页未确认预编译包，先核对实际资产；不存在则采用固定 revision 的受控源码构建，目标目录继承领取的 CARGO_TARGET_DIR，用户级部署保留许可证与摘要。禁止猜测 npm 包或任意下载 URL；没有 Rust 构建环境必须给出可操作错误，不能悄然安装其他引擎。
7. 注册稳定 key/display name `aionrs`，ACP 启动指向 `aionrs-acp`，由桥接启动 `aionrs --json-stream`；参数类型化构造。只在探针证明基础闭环安全可用后开放默认停用的官方预设，不新增 shiju-agent-cli 别名，不改现有原生引擎。
8. 按实际支持接入模型来源与 MCP：私有环境和既有模型中继传递所选 provider/model，密钥不进入 argv；不能修改用户全局 .aionrs.toml。MCP 只在 ready 后、首条 message 前注入，等待 mcp_ready；更新时明确处理协议窗口限制。技能只挂载选中隔离视图，未经证实不广告任意目录支持。被拒绝的已选能力必须在发 prompt 前报错。
9. 先 `cargo fmt --all`，仅保留本任务格式改动；执行聚焦测试与 `cargo clippy --all-targets --all-features -- -D warnings`。验证新二进制的打包/定位路径，实际桌面暗亮主题使用 aionrs 发两轮、写临时文件、拒绝另一次写入和取消后继续；不依赖只构建 Element 的测试。
10. 审查注册仍保留原有与前 20 项，证据记录来源、版本、构建槽位、二进制摘要、确切命令、退出码、测试数量、真实请求/文件副作用、限制与平台覆盖。只在单一引擎端到端证据齐全后完成。

## 验证

- 按测试计划执行所有聚焦命令；额外执行 `cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1` 保护注册契约。
- 必须执行 `cargo clippy --all-targets --all-features -- -D warnings`；修复本次错误，既有无关失败保留上下文；不执行全量测试、fmt check、wasm check 或 Makefile。
- 实际 aionrs + 桥接 + ShiJu ACP 客户端证明 ready、首轮/第二轮、真实写入、拒绝无写入、取消与进程退出清理；协议帧映射测试成功不能替代此证据。
- 安全证据包括预先配置的 allow-list/auto-approve 不能越过宿主拒绝，异常审批 ID 不被接受，密钥不进入 argv/日志/持久化；未证实前不宣称完全访问支持。
- 记录安装/构建、二进制定位、Unix/Windows 契约测试；没有实际跨平台运行就保留明确待回归项，不把本机编译当全平台可用。
- 最终证据包含实际槽位、绝对 CARGO_TARGET_DIR、确切命令、退出码和有效测试数，以及真实受控模型和文件副作用的脱敏回执。

## 完成

- 标记 `已完成` 前须证明实际 aionrs 可通过现有产品入口完成上述安全会话闭环；可选能力上游不支持则证明明确拒绝。
- 先调查并解决必要依赖；仍缺关键外部条件则只在本文件标记 `已阻塞`，记录已尝试路径和所需输入。只有无关既有问题阻止完成验证时可标记 `已完成，待回归`，不得伪称接入通过。
- 有实际变更则使用 `$git-commit` 只提交本任务，排除其他任务改动。成功后报告证据并删除本文件；阻塞/待回归保留。只有任务文件状态或删除不单独提交；plan.md 只读。
