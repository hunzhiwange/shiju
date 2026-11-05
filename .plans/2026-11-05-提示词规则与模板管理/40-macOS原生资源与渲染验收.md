# 任务 40: macOS原生资源与渲染验收

批次：【批次 40】 依赖批次39。线性顺序1→2→…→40；本批次仅此任务，无可并行任务。

状态：未开始

目的：Pi原生变量模板调用与中文暗黑渲染有macOS实测证据。

来源任务：`.plans/2026-11-05-CC能力完整融合/13-提示词规则与模板管理.md`；具体来源：母任务13需求3/4、Pi原生组件。源目录为`/Users/xiangmin/worker/code/cc-switch/`，实施优先核对母任务1固定来源快照。

预计会话范围：约30–60分钟，只交付本项行为与测试；不混入相邻任务。缓存首次构建耗时单独记录，不因此合并任务或省略证据。

## 文件

- 新建（本计划拟新增）：`docs/shiju/agent-apps/prompts/macos-native-evidence.md`。
- 读取：`docs/shiju/agent-apps/prompts/coverage.md`和本计划前置任务证据；若需修复，限定本场景并新增对应独立回归测试，不扩大为其他母能力实施。

## 上下文与边界

- 实施前必须已有母任务1–12所需底座验收证据，特别是母任务1来源快照、2存储/权限/主机/操作合同、3应用目录及4的MCode事务能力。缺依赖记录具体产物，不重造。
- 实测Pi AGENTS状态、SYSTEM/APPEND读改删、中文模板slug/改名/变量命令调用和reload；记录Pi版本与$1/$2/$@/$ARGUMENTS输出；暗/亮主题、长中文正文/路径/emoji滚动编辑不panic；OMP只验真实AGENTS能力，不虚报模板支持。
- 当前仓库不存在vw-agent-apps；本计划新文件/模块/测试名是拟新增契约，不能把未落地依赖当已存在。对应Cargo.toml由母任务1/2交付，不在此任务假设已注册。

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

## 验证计划

- 测试驱动开发不适用：本任务对前置已实现链路作macOS实测，不新增生产行为；若发现本项缺陷，先补独立失败测试再修复。
- 在macOS先运行前置已有聚焦测试（默认features，不新增脚本）：
  - `cargo test -p vw-agent-apps pi_agents_selection_follows_file_and_rejects_drift -- --nocapture`。
  - `cargo test -p vw-agent-apps template_metadata_roundtrips_unknown_fields_and_arguments -- --nocapture`。
  - `cargo test -p vw-agent-apps template_rename_conflict_keeps_original -- --nocapture`。
- 实窗：`cargo run -p vw-desktop --bin shi-ju`，使用隔离存储/测试授权主机根目录。原生应用调用使用母任务3已交付的受控启动入口，先记录实际版本、解析后的命令和测试目录，不凭空假设CLI参数，不使用真实账号文件。
- 运行`cargo clippy --all-targets --all-features -- -D warnings`；如本任务修复Rust，先`cargo fmt --all`。
- 所需证据：每场景平台/应用版本、步骤、API请求与状态、SDK/实窗结果、文件前后哈希、测试数和退出码、Cargo槽位及绝对路径；不记录凭据或敏感正文。外部应用未安装或无隔离运行条件，记未验证并保留任务。
- 将coverage.md中本组映射指向新证据；全部组通过后才可处理母任务13完成状态，不能修改母plan.md。

## 完成与阻塞

- 本项只有上述当前证据齐全才标记`已完成`；长期证据写入`docs/shiju/agent-apps/prompts/`，至少包含本项来源、实现/API/SDK/UI或不适用原因、测试与实测。
- 真正缺前置产物/权限/实机条件时尽力定位，仍无法突破只在本编号文件记`已阻塞`、缺失产物和已运行检查；不承接整个其他母能力。因无关既有编译错误无法回归，可记`已完成，待回归`并保留本文件，不能伪报验证通过。
- 有实际代码、测试、配置或文档变更时，使用`$git-commit`审查并仅提交本项变更；只有编号文件状态更新或删除不单独提交。已完成后报告证据并删除本编号文件；待回归/阻塞保留。
- 不修改只读plan.md、不把prompt.md当进度来源、不提交无关工作区变化；不执行无过滤全量测试、fmt check、wasm检查或Makefile。
