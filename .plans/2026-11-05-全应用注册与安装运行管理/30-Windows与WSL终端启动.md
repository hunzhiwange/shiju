# 任务 30: Windows与WSL终端启动

批次：【批次 30】 依赖：批次 29（传递依赖前述全部批次）。线性顺序1→2→…；本批次只执行本任务，不与任何其他批次并行。

状态：未开始

目的：Windows终端与WSL启动不跨发行版且支持含中文路径。

来源任务：母任务3需求；`misc.rs::launch_provider_terminal及Windows/WSL终端分支`；详见来源与接口.md对应映射。

预计会话范围：只实现本标题的一个行为和聚焦失败/通过验证；通常30–60分钟有效编码，构建等待单列。不要顺手迁移相邻领域或整份misc.rs。

## 文件

- 新建（本计划拟新增）：`crates/vw-agent-apps/src/apps/launch/windows.rs`。
- 测试（独立文件）：`crates/vw-agent-apps/src/apps/launch/windows_tests.rs`，测试名/共同过滤前缀 `cc_apps_windows_terminal_launch`。
- 修改：`crates/vw-agent-apps/src/lib.rs` 及本领域最小模块声明；如产物已由依赖建立则修改而非覆盖。
- 验证记录：`docs/shiju/agent-apps/task-30.md`（完成后保留，不随任务删除）。

## 上下文

- 迁入cmd/PowerShell/Windows Terminal与WSL选择规则；正确引用参数/编码；使用已校验shell/flag；平台不支持返回显式错误。
- 基础依赖：母任务1来源快照与母任务2正式包/网关接入契约；禁止假设未交付符号已存在。
-  首次涉及apps子模块时新建相应mod.rs并在既有lib.rs注册；不改变upstream。
- 接口与职责以 `来源与接口.md` 为准；新需求或范围变化先问用户是否记录回来源，不自动修改来源快照与母计划。

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

- 行为：Windows终端与WSL启动不跨发行版且支持含中文路径。
- 失败验证测试：在 `crates/vw-agent-apps/src/apps/launch/windows_tests.rs` 新增 `cc_apps_windows_terminal_launch` 前缀行为用例，至少一个有效测试；覆盖成功、明确失败与上述边界，断言真实结果/副作用而非私有函数调用次数。
- 失败验证命令：`cargo test -p vw-agent-apps --lib cc_apps_windows_terminal_launch -- --nocapture`。
- 预期失败原因：当前尚无本领域实现/接线，新增用例应因缺失目标契约或本任务行为不满足而失败。先补最小可编译签名可接受；不能以工具缺失、未交付底座或无关编译错误充当预期失败。
- 通过验证命令：`cargo test -p vw-agent-apps --lib cc_apps_windows_terminal_launch -- --nocapture`。
- crate/features：`vw-agent-apps`，聚焦测试使用该crate默认features；全仓Clippy使用全部features。Windows/WSL规则用显式环境夹具在macOS运行，不将全部测试cfg掉导致零测试。
- 模拟策略：真实临时文件、数据库/HTTP与子进程优先；只替代外网元数据和会修改全局安装的包管理器/OS终端边界。元数据提供真实dist-tags/engines/release形状，进程夹具记录argv、cwd、env并可控制退出码/延迟；Windows规则夹具不能冒充实机执行证据。不修改HOME或打印凭据。

## 步骤

1. 检查AGENTS.md、当前差异、来源符号、相邻测试及真实调用者；确认母任务1、2与本批次依赖，读取来源与接口.md。
2. 编写上述独立失败测试，领取槽位后执行失败命令；记录预期失败原因。
3. 仅实现本任务行为：迁入cmd/PowerShell/Windows Terminal与WSL选择规则；正确引用参数/编码；使用已校验shell/flag；平台不支持返回显式错误。
4. Rust修改后先运行 `cargo fmt --all`，审查并排除无关格式化差异；运行通过命令及全仓Clippy。
5. 保持聚焦用例通过再整理局部代码；记录持久证据、释放自己的锁，再按完成规则处理本任务。

## 验证

- 聚焦运行：`cargo test -p vw-agent-apps --lib cc_apps_windows_terminal_launch -- --nocapture`；有效测试数量必须大于0。
- 必须运行：`cargo clippy --all-targets --all-features -- -D warnings`。
- 预期：本任务行为用例通过且无新增警告；不运行无过滤全量cargo test、cargo fmt检查、wasm检查或Makefile。
- 所需证据：失败与通过的确切命令/退出码/有效数量、本任务副作用或拒绝场景、实际槽位及绝对CARGO_TARGET_DIR、审查差异；记录进docs/shiju/agent-apps/task-30.md。无关既有错误单列，不能悄悄修复或假报通过。

## 完成

- 只有获得当前行为和必要UI证据才能标记 `已完成`。真实依赖不足且无法突破则只在本文件记 `已阻塞`、原因及已做检查；不修改plan.md。无关既有错误阻止回归时可记 `已完成，待回归`，保留文件及未验证项。
- 若有实际代码、测试、配置或文档变更，使用 `$git-commit` 审查并提交本任务实际变更；排除他人和其他任务差异。只有编号文件状态更新/删除不单独提交。
- 最终报告证据和提交；完整通过后删除本编号文件，证据留在docs。母任务3必须等全部子项及跨能力收口验收后才能完成。
