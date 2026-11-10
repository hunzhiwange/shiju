# 任务 12: MCode限长与事务写入

批次：【批次 3】 依赖：批次 2 的任务 3。同批实施与资源规则见 prompt.md。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：MCode按UTF-8字节限32KiB且写库失败恢复文件。

来源任务：`.plans/2026-11-05-CC能力完整融合/13-提示词规则与模板管理.md`；具体来源：upsert_mcode_prompt、enable_mcode_prompt、prompt_files.rs。源目录为`/Users/xiangmin/worker/code/cc-switch/`，实施优先核对母任务1固定来源快照。

预计会话范围：约30–60分钟，只交付本项行为与测试；不混入相邻任务。缓存首次构建耗时单独记录，不因此合并任务或省略证据。

## 文件

- 新建（本计划拟新增）：`crates/vw-agent-apps/src/prompts/mcode.rs`。
- 新建独立测试：`crates/vw-agent-apps/src/prompts/mcode_tests.rs`。
- 修改（前置交付后）：`crates/vw-agent-apps/src/lib.rs`、`crates/vw-agent-apps/src/prompts/mod.rs`，只做本模块导出与必要工厂接线。

## 上下文与边界

- 按任务 1 记录核对本任务实际消费的产物。母任务 1 提供来源，母任务 2 提供业务包及存储、授权、主机和操作合同。解析应用文件时核对母任务 3 的路径，涉及 MCode 写入时核对母任务 4 的事务。
- 复用母任务4的MCode write-and-commit等价事务能力；测试32768/32769字节和中文边界；同一目标串行化；只迁移MCode特殊路径，不改供应商配置业务。
- 当前仓库不存在vw-agent-apps；本计划新文件/模块/测试名是拟新增契约，不能把未落地依赖当已存在。对应Cargo.toml由母任务1/2交付，不在此任务假设已注册。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用原仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-20`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建原仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–20。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从原仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 20 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从原仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 测试计划

- 行为：MCode按UTF-8字节限32KiB且写库失败恢复文件。
- 失败验证测试：`crates/vw-agent-apps/src/prompts/mcode_tests.rs`中的`mcode_prompt_write_rolls_back_and_enforces_byte_limit`测试组，覆盖上述成功与拒绝/失败分支；通过独立测试模块注册，不内嵌业务文件。
- 失败验证命令：`cargo test -p vw-agent-apps mcode_prompt_write_rolls_back_and_enforces_byte_limit -- --nocapture`。
- 预期失败：当前缺少本项行为，新增的真实行为断言失败；先保证依赖已满足、测试可编译。无关编译错误和零测试不能作为预期失败证据。
- 通过验证命令：`cargo test -p vw-agent-apps mcode_prompt_write_rolls_back_and_enforces_byte_limit -- --nocapture`，使用crate默认features，不传额外features；Clippy使用全部features。
- 模拟策略：业务用真实临时目录与前置存储适配；API用真实router与存储/文件，SDK用本地HTTP记录器仅替代远端传输；UI只替代慢/不确定的网络响应以复现乱序与失败，并在macOS实窗复验。不mock核心规则，不修改HOME，不连真实用户目录。

## 步骤

1. 重读对应来源函数、现有工厂、调用者与相邻测试；确认上一批证据和外部依赖，不修改只读plan.md。
2. 在指定独立文件编写行为失败测试，领取槽位后运行失败命令并记录原因与有效测试数。
3. 最小实现：复用母任务4的MCode write-and-commit等价事务能力；测试32768/32769字节和中文边界；同一目标串行化；只迁移MCode特殊路径，不改供应商配置业务。
4. 运行`cargo fmt --all`，复查差异，剔除与本任务无关的格式化改动；执行通过验证命令。
5. 执行仓库Clippy并审查本项影响；只在测试通过后局部重构。

## 验证

- 精确聚焦命令：`cargo test -p vw-agent-apps mcode_prompt_write_rolls_back_and_enforces_byte_limit -- --nocapture`；macOS宿主默认target；默认features。有效测试数必须大于零，每个边界分支报告命中证据。
- 必须运行：`cargo clippy --all-targets --all-features -- -D warnings`。
- 所需证据：失败断言与原因、通过断言、有效数量、命令/退出码、实际槽位和绝对CARGO_TARGET_DIR、修改文件/符号及已审查差异。

## 完成与阻塞

- 本项只有上述当前证据齐全才标记`已完成`；长期证据写入`docs/shiju/agent-apps/prompts/`，至少包含本项来源、实现/API/SDK/UI或不适用原因、测试与实测。
- 真正缺前置产物/权限时尽力定位，仍无法突破只在本编号文件记`已阻塞`、缺失产物和已运行检查；不承接整个其他母能力。因无关既有编译错误无法回归，可记`已完成，待回归`并保留本文件，不能伪报验证通过。
- 有实际代码、测试、配置或文档变更时，使用`$git-commit`审查并仅提交本项变更；只有编号文件状态更新或删除不单独提交。已完成后报告证据并删除本编号文件；待回归/阻塞保留。
- 不修改只读plan.md、不把prompt.md当进度来源、不提交无关工作区变化；不执行无过滤全量测试、fmt check、wasm检查或Makefile。
