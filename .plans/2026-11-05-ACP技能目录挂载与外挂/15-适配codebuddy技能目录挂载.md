# 任务 15: 适配codebuddy技能目录挂载

批次：【批次 14】 依赖批次 9；依赖任务 9、10。执行顺序按prompt.md串行，不依赖前一agent成功。

状态：未开始

目的：逐项证明codebuddy原生目录挂载；明确不支持则有证据跳过。

来源任务：本次用户确认的“技能目录挂载、支持agent逐个适配、ACP服务端外挂/更新/恢复及实际验收”。

预计会话范围：围绕本任务唯一行为完成代码、独立测试和证据；先读取现有实现、调用者与相邻测试。禁止顺手重构模型来源、桌面表单、品牌或其他并发改动。

## 文件

- 修改：crates/vw-agent/src/session/llm/acp/skill_mounts/codebuddy.rs
- 修改：crates/vw-agent/src/session/llm/acp/skill_mounts/codebuddy_tests.rs
- 修改：crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs
- 新建：docs/vibewindow/acp-skill-mounts/evidence/codebuddy.json
- 修改：docs/vibewindow/acp-skill-mounts/agents.json
- 按固定发行物需要修改：crates/vw-acp/src/managed_runtime/contracts.json、managed_runtime/install.py及该agent现有补丁；无必要不修改

本文件列出的“新建”均是拟实现产物；测试前缀同样是本任务要求新增的行为测试，并非宣称当前仓库已有。若执行时已存在，先读实际内容后做最小修改。

## 上下文

读取任务1中codebuddy固定发行版及原生ACP源码，按其真实目录/插件契约适配，不从Claude/Codex推断。

仓库根目录 `/Users/xiangmin/worker/code/shiju`。先读根AGENTS.md与 `./skills/rustcodegraph/SKILL.md`，通过代码图及必要原始读取核实调用链；只读 `.plans/2026-11-05-ACP技能目录挂载与外挂/plan.md`。目标crate为vw-acp和/或vw-agent（脚本生产进程验收还涉及vw-cli），具体命令如下。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-10`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–10。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 10 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 测试计划

- 行为：codebuddy真实ACP路径发现选中技能目录并读取随机附件；首次prompt没有技能全文。
- 失败验证测试：acp_skill_mount_codebuddy_：固定入口参数/协议确认及负向拒绝；acp_skill_mount_real_codebuddy为真实ignored验收入口。
- 失败验证命令：`cargo test -p vw-agent --lib acp_skill_mount_codebuddy_ -- --nocapture --test-threads=1`。
- 预期失败：新增行为尚未实现，或当前实现提前拼接/忽略目录/无法更新恢复；断言应失败而非零测试。首先验证失败原因对应本任务。
- 通过验证命令：同上，使用默认features及`--lib`聚焦过滤，不执行全量测试。真实CLI命令另列于本任务步骤。
- 模拟边界：只有外部模型HTTP及明确的协议错误对端可用fixture；标为真实验收时，不mock技能发现、文件系统、ACP生产后端、真实CLI或MCP工具执行。

## 步骤

1. 先复核任务1的codebuddy结论、固定版本/源码/平台与本机安装身份。候选入口存在就实施；确认不支持才走下述跳过流程。
2. 仅在codebuddy.rs中实现其目录入口和加载确认，继续复用任务3视图；保持session_model与cli_local原有模型、权限、凭据隔离。
3. 如确需改adapter，仅在该agent已审查补丁中最小修改，更新真实构建散列和安装校验；不改全局CLI、不绕过散列、不升级其他agent。没有真实注册入口时不为凑数新增通用文本伪适配。
4. 先运行本项负向/未注册失败，再运行 python3 scripts/verify_acp_skill_mounts.py --agent codebuddy --manifest docs/vibewindow/acp-skill-mounts/agents.json --output "$CARGO_TARGET_DIR/acp-skill-mount-evidence/codebuddy"。
5. 真实执行覆盖原生发现、附件/脚本、添加/移除、V1/V2内容更新、隔离、取消和进程恢复；可执行的两种模型来源分别验收，不可执行来源单独记录原因，不阻止已可用原生来源的挂载适配。
6. 只有工具与模型请求确实发生且所有必需断言通过才将本项标记verified并发布支持；记录版本与平台，其他平台不推断。

### 不支持与阻塞的处理

- 固定发行版确认没有目录/插件注册入口，或真实入口明确拒绝且无该版本原生支持方式：将 `codebuddy` 清单记 `unsupported`，保存源码/协议负向证据，保持拒绝分支，不保留正文拼接。此项任务可按“已完成：确认不支持，按用户要求跳过”收尾；这不是挂载成功，不能计入通过数量。
- 未安装、缺账号、缺依赖、权限或机器不可用属于 `unverified`，不能改写成 `unsupported`。尽量使用既有已授权环境和固定发行物继续；无法取得真实证据时在本任务记录阻塞，独立agent任务可继续，最终回归不能隐藏此缺口。
- `agent` 只有模型来源不支持时，不得直接跳过技能适配。已存在支持目录入口时，优先在既有已授权的可用来源完成验收；缺授权时记录阻塞，不伪造登录或改动用户账号状态。
- 本任务串行占用共享注册清单、acceptance_tests.rs和可能的安装校验文件；不能与其他agent任务并行。若只取得unsupported文档结论且未改Rust，不必运行Cargo；实际修改Rust或补丁则仍遵守完整槽位及验证规则。

## 验证与完成证据

- 所需证据：本项聚焦测试及真实CLI报告；不能用其他agent、模型路由旧报告、--help、握手或空测试替代codebuddy技能证据。
- Rust修改后先按槽位规则运行 `cargo fmt --all`，审阅差异，只保留本任务相关格式变化；不得提交无关修改。然后运行本任务聚焦验证，以及 `cargo clippy --all-targets --all-features -- -D warnings`。
- 文档独立任务不运行Cargo；其他AI或既有改动导致构建失败时记录确切错误，不处理无关问题。因该原因未能验证，最多标记“已完成，待回归”，不宣称通过。
- 测试必须位于独立文件，记录真实执行次数；编译成功、零测试、跳过或ignored数量不算通过。不得暴露凭据、完整MCP载荷、用户真实文件或生产模型日志。
- 只有本任务行为证据齐全后，才能标记“已完成”。实现完成但被无关验证问题挡住则“已完成，待回归”；本任务自身需求/环境无法突破则“已阻塞”，在本文件记录原因、已做尝试和下一步。
- 有实际代码、测试、配置或文档变更，使用 `$git-commit` 分析并提交本任务变更，排除其他任务及用户改动；只改本任务状态或删除任务文件时，不调用提交技能。
- 完成后最终回复报告证据并删除本任务文件；待回归/阻塞则保留。不要修改plan.md或prompt.md来记录进度。新发现若超出已授权范围，先向用户说明并确认是否记录到来源产物，不自行加入新需求。
