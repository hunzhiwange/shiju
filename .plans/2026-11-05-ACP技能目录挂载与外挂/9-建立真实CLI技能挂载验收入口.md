# 任务 9: 建立真实CLI技能挂载验收入口

批次：【批次 9】 依赖批次 8；依赖任务 8。

状态：未开始

目的：建立逐agent使用真实固定CLI、生产ACP路径及本地模型服务的验收工具。

来源任务：本次用户确认的“技能目录挂载、支持agent逐个适配、ACP服务端外挂/更新/恢复及实际验收”。

预计会话范围：围绕本任务唯一行为完成代码、独立测试和证据；先读取现有实现、调用者与相邻测试。禁止顺手重构模型来源、桌面表单、品牌或其他并发改动。

## 文件

- 新建：scripts/verify_acp_skill_mounts.py
- 新建：scripts/acp_skill_mount_fixture.py
- 新建：scripts/tests/test_verify_acp_skill_mounts.py
- 新建：crates/vw-agent/src/session/llm/acp/skill_mounts/acceptance_tests.rs
- 修改：crates/vw-agent/src/session/llm/acp/skill_mounts/mod.rs

本文件列出的“新建”均是拟实现产物；测试前缀同样是本任务要求新增的行为测试，并非宣称当前仓库已有。若执行时已存在，先读实际内容后做最小修改。

## 上下文

参考verify_acp_model_source.py的固定发行物识别、私有环境和共享槽位校验；不复用旧模型来源通过标记作为技能证据。

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

- 行为：未知/缺失CLI和零模型请求不能计通过；读取工具结果后才能生成随机验收答案。
- 失败验证测试：acp_skill_mount_harness_及脚本入口测试；真实CLI测试标记ignored，函数分别命名为 acp_skill_mount_real_codex、acp_skill_mount_real_claude、acp_skill_mount_real_qwen、acp_skill_mount_real_gemini、acp_skill_mount_real_codebuddy、acp_skill_mount_real_goose、acp_skill_mount_real_vibe、acp_skill_mount_real_hermes、acp_skill_mount_real_snow、acp_skill_mount_real_copilot、acp_skill_mount_real_kilocode、acp_skill_mount_real_kimi、acp_skill_mount_real_kiro、acp_skill_mount_real_opencode、acp_skill_mount_real_openclaw、acp_skill_mount_real_pi、acp_skill_mount_real_auggie、acp_skill_mount_real_cursor、acp_skill_mount_real_droid、acp_skill_mount_real_qoder、acp_skill_mount_real_trae、acp_skill_mount_real_iflow。
- 失败验证命令：`cargo test -p vw-agent --lib acp_skill_mount_harness_ -- --nocapture --test-threads=1`。
- 预期失败：新增行为尚未实现，或当前实现提前拼接/忽略目录/无法更新恢复；断言应失败而非零测试。首先验证失败原因对应本任务。
- 通过验证命令：同上，使用默认features及`--lib`聚焦过滤，不执行全量测试。真实CLI命令另列于本任务步骤。
- 模拟边界：只有外部模型HTTP及明确的协议错误对端可用fixture；标为真实验收时，不mock技能发现、文件系统、ACP生产后端、真实CLI或MCP工具执行。

## 步骤

1. 新增CLI命令 --agent、--manifest、--output；脚本只定位已核实固定版本，不借npx/npm临时下载，不覆盖HOME等通用环境根。
2. 本地模型fixture只替代外部模型HTTP服务；真实CLI、ACP适配器、技能注册、文件读取和生产请求入口不得mock。
3. 建立随机技能及references/scripts样本；随机值仅存附件，不放入prompt或硬编码模型最终答案；记录实际工具请求与结果、原生发现事件及模型请求次数。
4. 断言首次prompt无技能全文；目录发现和附件工具读取必须发生，执行无害脚本以读取fixture文件；不支持脚本的固定契约需明确记录，不将附件文字伪装成执行。
5. 覆盖添加、移除、内容V1/V2、第二会话隔离、取消、空闲更新、CLI重启/会话恢复及权限拒绝；输出脱敏JSON。
6. 框架测试故意使用伪CLI/零请求/错误哨兵时必须失败；注册22个ignored入口，未实现agent不可返回通过。

另运行 `python3 -m unittest discover -s scripts/tests -p test_verify_acp_skill_mounts.py`；脚本内部所有Cargo/子进程必须继承槽位。生成的JSON必须包含status、原因、版本/散列、平台、挂载来源模式、真实工具与模型请求次数及生命周期断言。

## 验证与完成证据

- 所需证据：框架红绿测试和Python入口测试通过；真实执行命令为 python3 scripts/verify_acp_skill_mounts.py --agent codex --manifest docs/vibewindow/acp-skill-mounts/agents.json --output "$CARGO_TARGET_DIR/acp-skill-mount-evidence/codex"。这是任务11的Codex具体入口；其余agent的字面命令分别写在对应任务。
- Rust修改后先按槽位规则运行 `cargo fmt --all`，审阅差异，只保留本任务相关格式变化；不得提交无关修改。然后运行本任务聚焦验证，以及 `cargo clippy --all-targets --all-features -- -D warnings`。
- 文档独立任务不运行Cargo；其他AI或既有改动导致构建失败时记录确切错误，不处理无关问题。因该原因未能验证，最多标记“已完成，待回归”，不宣称通过。
- 测试必须位于独立文件，记录真实执行次数；编译成功、零测试、跳过或ignored数量不算通过。不得暴露凭据、完整MCP载荷、用户真实文件或生产模型日志。
- 只有本任务行为证据齐全后，才能标记“已完成”。实现完成但被无关验证问题挡住则“已完成，待回归”；本任务自身需求/环境无法突破则“已阻塞”，在本文件记录原因、已做尝试和下一步。
- 有实际代码、测试、配置或文档变更，使用 `$git-commit` 分析并提交本任务变更，排除其他任务及用户改动；只改本任务状态或删除任务文件时，不调用提交技能。
- 完成后最终回复报告证据并删除本任务文件；待回归/阻塞则保留。不要修改plan.md或prompt.md来记录进度。新发现若超出已授权范围，先向用户说明并确认是否记录到来源产物，不自行加入新需求。
