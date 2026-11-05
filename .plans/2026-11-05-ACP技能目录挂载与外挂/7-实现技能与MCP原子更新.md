# 任务 7: 实现技能与MCP原子更新

批次：【批次 7】 依赖批次 6；依赖任务 6。

状态：已完成，待回归

目的：在空闲会话中完整替换外挂集合，失败保留旧集合。

来源任务：本次用户确认的“技能目录挂载、支持agent逐个适配、ACP服务端外挂/更新/恢复及实际验收”。

预计会话范围：围绕本任务唯一行为完成代码、独立测试和证据；先读取现有实现、调用者与相邻测试。禁止顺手重构模型来源、桌面表单、品牌或其他并发改动。

## 文件

- 修改：crates/vw-acp/src/server/mounts.rs
- 修改：crates/vw-acp/src/server/state.rs
- 修改：crates/vw-acp/src/server/agent.rs
- 修改：crates/vw-acp/src/server/backend.rs
- 修改：crates/vw-agent/src/acp/mounts.rs
- 修改：crates/vw-agent/src/acp/backend.rs
- 修改：crates/vw-agent/src/acp/mcp.rs
- 新建：crates/vw-agent/src/acp/mount_update_tests.rs
- 新建：crates/vw-acp/src/server/mount_update_tests.rs

本文件列出的“新建”均是拟实现产物；测试前缀同样是本任务要求新增的行为测试，并非宣称当前仓库已有。若执行时已存在，先读实际内容后做最小修改。

## 上下文

复用reserve_prompt占用机制，但更新必须覆盖准备至发布窗口，防止提示启动与挂载更新竞争。更新不是只调用MCP connections.synchronize。

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

- 行为：expectedRevision冲突/忙碌/任何准备失败不改变旧集合；空数组清空；成功返回新revision。
- 失败验证测试：acp_mount_update_：失败第二个MCP、技能链接失败、并发prompt、双更新竞争、清空、移除后工具不可调用。
- 失败验证命令：`cargo test -p vw-agent -p vw-acp --lib acp_mount_update_ -- --nocapture --test-threads=1`。
- 预期失败：新增行为尚未实现，或当前实现提前拼接/忽略目录/无法更新恢复；断言应失败而非零测试。首先验证失败原因对应本任务。
- 通过验证命令：同上，使用默认features及`--lib`聚焦过滤，不执行全量测试。真实CLI命令另列于本任务步骤。
- 模拟边界：只有外部模型HTTP及明确的协议错误对端可用fixture；标为真实验收时，不mock技能发现、文件系统、ACP生产后端、真实CLI或MCP工具执行。

## 步骤

1. 实现_shiju/session/set_mounts扩展及后端更新调用，获取与prompt/load共享的会话操作租约，检查归属和revision。
2. 先准备完整技能视图和全部MCP真实连接，成功才原子替换；任何失败清理候选资源并保留旧revision和旧工具。
3. 切换后清除旧技能目录/已调用技能状态与工具发现缓存，撤销旧MCP连接，旧资源不再影响后续回合。
4. busy不等待无限时长，明确拒绝；实现同revision重复请求的明确冲突语义，避免双写。
5. 启用更新能力声明；返回实际名称和revision，不记录令牌、headers或原始MCP载荷。

## 验证与完成证据

- 所需证据：真实MCP子进程的旧/新工具调用及退出可观察；失败回滚和竞争测试通过，资源无泄漏。
- Rust修改后先按槽位规则运行 `cargo fmt --all`，审阅差异，只保留本任务相关格式变化；不得提交无关修改。然后运行本任务聚焦验证，以及 `cargo clippy --all-targets --all-features -- -D warnings`。
- 文档独立任务不运行Cargo；其他AI或既有改动导致构建失败时记录确切错误，不处理无关问题。因该原因未能验证，最多标记“已完成，待回归”，不宣称通过。
- 测试必须位于独立文件，记录真实执行次数；编译成功、零测试、跳过或ignored数量不算通过。不得暴露凭据、完整MCP载荷、用户真实文件或生产模型日志。
- 只有本任务行为证据齐全后，才能标记“已完成”。实现完成但被无关验证问题挡住则“已完成，待回归”；本任务自身需求/环境无法突破则“已阻塞”，在本文件记录原因、已做尝试和下一步。
- 有实际代码、测试、配置或文档变更，使用 `$git-commit` 分析并提交本任务变更，排除其他任务及用户改动；只改本任务状态或删除任务文件时，不调用提交技能。
- 完成后最终回复报告证据并删除本任务文件；待回归/阻塞则保留。不要修改plan.md或prompt.md来记录进度。新发现若超出已授权范围，先向用户说明并确认是否记录到来源产物，不自行加入新需求。

## 本次实施与验证记录（2026-11-05）

- 依赖任务 6 已提交：`45a5423c4`，原生技能目录发现、调用与 MCP 准备入口已具备。
- 实现命名空间 set_mounts、独立更新能力声明、连接及后端两层 prompt/load/update 租约、expectedRevision 检查、完整候选准备后发布、旧技能句柄撤销、旧 MCP 连接关闭及调用 checkpoint 清理。load 保留当前 revision；MCP 部分准备失败通过租约销毁候选连接。
- 新增独立测试覆盖真实 stdio MCP 文件工具调用、第二个 MCP 失败回滚及候选 PID 退出、技能路径失败、旧句柄撤销、空集合清理、revision 冲突和双写竞争；生产 Agent/后端测试在真实 MCP 准备窗口检查 prompt/load/第二次更新立即 busy。
- Cargo 实际动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；不删除编译缓存，阶段进程结束后释放本任务锁。
- `cargo fmt --all`：退出 0；`git diff --check`：退出 0。
- 红灯：`cargo test -p vw-agent -p vw-acp --lib acp_mount_update_ -- --nocapture --test-threads=1`，退出 101，实际 1 项测试失败，断言 setMounts=false 与预期 true 不符（非零测试）。
- 中间版本同一聚焦命令：退出 0，vw-acp 2 项、vw-agent 3 项，共 5 项通过、无 ignored；真实 old/new 工具调用、候选及旧 MCP PID 退出、busy 竞争均实际观察。此后补强了后端租约、load revision 保留、checkpoint 清理和同名技能源替换测试，不能用中间版本报告替代最终版本验收。
- 最终聚焦命令退出 101，执行行为测试 0 项（编译阻塞，不能算通过）；另有与 Clippy 相同的 E0599。无关并发修改的 E0308：`crates/vw-agent/src/session/processor/runner.rs:878,1020` 的 `request_catalog` 需要 `&ToolUseContext`，实传 `Arc<ToolUseContext>`。未修改该无关文件；最终 vw-agent 行为验证未通过，不宣称验收完成。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出 101，除上述 E0308，`crates/vw-agent/src/gateway/api/handlers/dynamic_tools_tests.rs:63,64,276,277` 对 PlatformId/CompanyId 调用不存在的 as_str，报 E0599；均在本任务范围外，未处理。
- 恢复条件：相关并发任务修复上述编译错误后，按共享槽位规则重跑最终聚焦命令及要求的 Clippy。通过后才能改为已完成并删除本任务文件；本次保留供回归。
- 最终独立协议验证：`cargo test -p vw-acp --lib acp_mount_update_ -- --nocapture --test-threads=1`，退出 0，2 项通过、0 ignored；不替代 vw-agent 的最终回归。

## 即时回归（2026-11-05，用户要求立即执行）

- 继续使用执行计划技能，未修改其他任务源码，状态保持 `已完成，待回归`，保留本任务文件。
- 动态领取槽位 **4**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`；另设置 `CARGO_BUILD_BUILD_DIR` 为同一绝对路径，确保中间产物及测试进程也使用共享槽位目录。
- `cargo test -p vw-agent -p vw-acp --lib acp_mount_update_ -- --nocapture --test-threads=1`：退出 **101**，实际执行测试 **0** 项（编译阶段失败，不计通过）。
- 本轮新阻塞：`crates/vw-agent/src/session/llm/native_tools_tests.rs:46,111` 引用 `aisdk::providers::openai::client::types::InputItem`，但当前固定 SDK 中 `types` 为 `pub(crate)`，编译报 **E0603**。该未跟踪文件来自本任务外的并发改动，未修改或绕过它。
- 下一步：对应任务修复私有 SDK 类型引用后，再运行本任务聚焦验证及 Clippy。不得将本轮编译失败记为真实行为验收通过。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出 **101**，同样因上述两处 E0603 失败，无本任务新错误。所有验证进程结束后已释放槽位 4，保留缓存；本轮仅更新任务证据，未提交。
