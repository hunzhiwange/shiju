# 任务 21 Harn 适配动态挂载 MCP

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 1】  
依赖：无  
状态：待回归  
来源：内置 ACP 注册 key `harn`

## 目标和复用请求

使用[新版复用请求](plan.md#新版复用请求)，将 `agent` 换成 `acp-harn`。按[统一验收合同](plan.md#统一验收合同)验证无挂载、挂载和撤除，允许官方工具在现有权限内共存，要求三个目标工具实际成功并回传结果。

## 文件

- 核对入口 `crates/libs/llm-runtime/src/acp/model_source/drivers/harn.rs`。仅修改已定位的目标后端路径。
- 若断点在受管入口，检查 `patches/acp-model-source/harn/` 及 `crates/libs/acp-agent-registry/src/managed_runtime/contracts.json` 中的 `harn` 键。
- 聚焦测试 `crates/vw-acp/tests/onboarding_harn.rs`。
- 记录 `docs/dev/acp/mcp-acceptance/harn.md`。

## 后端落点

核对私有 provider 与代理 pipeline，以及现有规则对 MCP 子进程不继承模型中继凭据的承诺。验证实际 stdio 进程和调用结果。

## 验证和收尾

- 从仓库根目录按公共计划复用 [新版复用请求](plan.md#新版复用请求)，完成三轮并记录首个失败边界。
- 现有保护或公共转发用例使用 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_harn acp_onboarding_harn_session_lifecycle`。仅实际涉及对应边界时运行。若需改变行为，先将本任务复现的因果条件落实为聚焦断言，再用同一目标与过滤条件取得红、绿证据。原样通过时以三轮真实证据收尾。
- 按新版合同重判既有证据，补齐默认共存路径的缺项。下列执行记录及其中的旧复跑条件用于追溯历史，以本节和新版合同为当前指令。
- 验收记录须含本后端实际版本、所选传输、会话和终态、三种工具的真实事件与输出、答复准确性、撤除对照及权限行为。依公共规则提交和清理。

## 执行记录（2026-11-19）

- 私有回环网关使用指定租户、模型及原提示完成三轮。无挂载与撤除均返回 `MCP_NOT_AVAILABLE`、各有一个 `chat.done`、零工具事件。挂载轮在 `session/new` 返回 `chat.error`。原生 ACP 独立复现 `-32602: this agent does not support session mcpServers intake`。
- Harn 宿主回调的独立探针能启动真实 RustCodeGraph stdio，子进程无测试中继变量，工具发现随即被 `tool:mcp:read` 效果上限拒绝。直接 MCP 对照的三个图谱工具通过，已有原生宿主写拒绝测试 1 项通过。未发生公共转发缺陷，未运行 Cargo。
- 保留权限门禁和任务卡。取得受审的会话级 MCP 授权入口后复跑原请求。脱敏结果见 [Harn 验收记录](../../docs/dev/acp/mcp-acceptance/harn.md)，私有原始证据位于 `/tmp/shiju-harn-mcp.pM2kwJ/`。

## 共享工作区复验（2026-11-19）

- 使用共享工作区较新的 shiju 0.6.0 完成第二组三轮，仍为两轮对照通过、挂载轮在 `session/new` 拒绝。验证官方新版 0.10.202 及其无 pipeline 入口，stdio、HTTP、SSE 均返回相同 `-32602`。
- 默认 `ask` 权限下，两版 `AgentSpec.mcp_servers` 聚焦探针各 1 项通过，均断言 `tool:mcp:mutate` 拒绝、模型请求数为 0。宿主 MCP 回调再次被 `tool:mcp:read` 拒绝，子进程未继承模拟中继变量。原有宿主写拒绝测试 1 项通过。
- 继续标记已阻塞。需要 Harn 提供原生会话 MCP 接入，或提供在 `ask` 下按会话和服务限定 MCP 权限且保留既有审批的宿主入口。获得后用固定请求复跑。未修改生产代码、权限或受管版本，未执行 Cargo。第二次私有证据目录为 `/tmp/shiju-harn-retry-mcp.yH7DoJ/`。

## 受管 v2 正式接入（2026-11-19）

- 提交 `ddb9994ab`。接入 Harn 会话级 stdio 工具、宿主批准和工具管理策略，沿用 Harn `ask`、请求私有中继及只读沙箱。安装路径升为 v2。既有 v1 由同一原生用例复现 `-32602`，v2 的安装摘要、8 个真实 stdio 场景、3 项宿主读写保护、2 项安装测试及 1 项自然语言测试通过。
- 注册契约定向测试通过 1 项。执行 `python3 scripts/cargo_slot.py cargo test --locked -j 1 -p acp-agent-registry harn_reviewed_natural_language_runtime_is_pinned`，槽位 3，两个缓存路径均为仓库的 `target/rust-slot-3`，退出 0。
- 共享工作区的重型构建槽仍有进行中的其它任务。现有网关产物未包含这次最终的受管摘要，尚不能跑指定模型三轮。待串行执行 `python3 scripts/cargo_slot.py cargo test --locked -j 1 -p session-engine acp_mcp_mount_carries_resolved_tool_policy`，随后使用 `python3 scripts/cargo_slot.py cargo build -j 1 --locked -p vw-cli --bin shiju` 取得包含新摘要的网关，按原合同三轮复验。暂保留任务卡和私有证据。

## v3 流式参数回归（2026-11-19）

- 提交 `dfd1fb056`。指定模型在 v2 发送分片调用，开始事件为空，完整参数在 `in_progress` 更新出现。v3 调整审批绑定并移除重复纯文本终态。分片用例 v2 失败、v3 的 8 个场景通过。原有 6 项 Python 安装和文件宿主保护检查通过，注册槽位 3 测试 1 项通过。
- 私有原生 ACP 使用同一真实模型和固定提示三轮均为 `end_turn`。挂载轮 112 个调用和批准、成功完成一一配对，仅三个目标工具。两轮对照精确返回 `MCP_NOT_AVAILABLE` 各一次，工具为零。保留原生和网关验收边界，指定模型的新版 HTTP 网关三轮及宿主默认配置 Rust 定向检查等待共享重型构建完成。证据目录 `/tmp/shiju-harn-managed.sFasYE/`。

## 新验收标准复验（2026-11-19）

- 采用新版请求，真实模型原生三轮 `end_turn`。无挂载与撤除既无图谱声明，也未调用或收到服务结果。挂载轮三个目标均成功，17 次结果按调用 ID 与实际模型请求的工具回复逐项对应。官方读取三轮为 3、2、6 次，沿用独立诊断宿主的只读许可。
- 验收记录已补新版证据。共享工作区当前链接的网关仍包含旧版 Harn 摘要，后续正在编译的目标继续占用 Rust 重型编译资源。保留状态待回归，须用含 v3 摘要的网关补齐 HTTP SSE、UI summary 和 ACP options。
- 共享网关构建在 `vw-agent` 因同期 MiMo/VT Code 接口缺失失败并未生成产物。后续其他重型编译连续占用，撤销了仅等待的探针，未启动本任务的重型编译。新标准原生证据见 `9a411ea16`。
