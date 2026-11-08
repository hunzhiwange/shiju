# 任务 2 · 迁移Git提交表单

**批次**　【批次 2】依赖批次 1，公共入口来自批次 1

**状态**　未开始

**目的**　提交类型从原生 pick_list 切换为语言同款选择框，和作用域、摘要输入等高，描述复用多行构造。

**来源任务**　无外部任务编号。依据用户对 Git 提交表单与同类控件的要求。

**预计会话范围**　仅处理本任务的控件构造及视图接线、必要局部布局和真实渲染回归。

## 文件

- 修改 `crates/vw-desktop/src/app/components/git_panel/left_panel.rs`。
- 接线核对 `crates/vw-desktop/src/app/components/git_panel/mod.rs`。
- 接线核对 `crates/vw-desktop/src/app/message/git/stage_commit.rs`。
- 接线核对 `crates/vw-desktop/src/app/state/presentation.rs`。
- 测试 `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs`。

## 上下文

- left_panel::view 由 Git 面板在 show_git_diff_summary 时挂载，帮助弹窗单独叠放。类型、作用域和摘要现在各自写死字号、内边距。
- 以常规设置「语言」与自定义模型弹窗为参照。执行前检查当前差异、相邻测试、调用者及本任务的文件占用，保留其他改动。

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

按 任务 1 → 任务 2 → 任务 3 → 任务 4 → 任务 5 → 任务 6 → 任务 7 串行执行，每批一个任务。各批共享检出、格式化和界面回归资源。后续批次先核对公共入口已可用，前序待回归问题若影响本任务依赖，先在前序任务文件中定位。

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

仓库在 `.cargo/config.toml` 另设 `build-dir = "target"`。领取槽位后同时执行 `export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`，让 Cargo 中间产物留在同一槽位。格式化前核对共享检出的并发写入和已有差异，格式化后审核差异，按提交技能只纳入本任务的变更。

## 测试计划

- 验证对象为 `vw-desktop` 的库测试。当前该 crate 默认 features 为空，聚焦测试采用默认配置，Clippy 使用工作区全部 features。
- 先在 left_panel_tests.rs 编写 git_commit_form_matches_general_language_select，渲染真实面板和公共控件参考。断言同高、选中项右侧勾选及焦点 Enter 打开。旧视图在共享菜单特征或键盘打开断言上失败。
- 基线或失败验证命令为 `cargo test -p vw-desktop --lib git_commit_form_ -- --test-threads=1`。
- 补充 git_commit_form_preserves_messages_and_validation，验证 Tab 顺序、键盘和鼠标选择、Escape 关闭、摘要/作用域/描述输入及原提交就绪条件。
- 通过验证命令为 `cargo test -p vw-desktop --lib git_commit_form_ -- --test-threads=1`。
- 采用真实 Iced 布局、事件、overlay 和绘制。参考公共选择框测试和自定义模型弹窗的 headless 渲染路径，加载默认中文字体。隔离网络、用户目录和持久化，使用虚构数据。

## 步骤

1. 核对现有生产入口，先运行基线或录制预期失败。验证用例需要覆盖真实控件的绘制或行为。
2. 把 commit_type_pick 接到 settings_select，沿用 ConventionalCommitType::all 的 19 项顺序、类型枚举、当前选中值、翻译占位和 CommitTypeSelected。宽度由提交面板决定。
3. 将约定式和简化分支的摘要、作用域及描述改为公共输入构造，沿用原消息与 88px 描述区高度。实际 Git 容器内检查窄宽时输入可用，按需局部调宽或换行。
4. 在真实表单打开菜单并绘制滚动末项，覆盖中文占位和英文类型、明暗两套主题、普通和窄宽。保留帮助弹窗及按钮优先级。使用隔离 App 测试夹具，只断言提交消息或状态，禁止触发真实提交。
5. 运行验证，核对绘制结果与消息，再审查本任务差异。

## 验证

- Rust 修改后运行 `cargo fmt --all`，再执行聚焦测试和 `cargo clippy --all-targets --all-features -- -D warnings`。
- 周边回归运行 `cargo test -p vw-desktop --lib git_panel::left_panel_tests:: -- --test-threads=1`。
- 周边回归运行 `cargo test -p vw-desktop --lib message::git::stage_commit_tests:: -- --test-threads=1`。
- 运行 `git diff --check`，审阅本任务差异。记录测试和 Clippy 的确切命令、退出码、有效测试数量、主题及语言和窗口宽度的验收结果。发现无关失败时记录出错位置和验证证据。
- 记录实际 Cargo 槽位与绝对 `CARGO_TARGET_DIR`。有效测试数量大于零，按本任务测试计划提供基线或失败证据和修复后证据。

## 完成

- 获得验证证据后标记为 `已完成`，有实际变更时按 $git-commit 技能单独提交。最终回复报告证据并删除本编号任务文件。仅有本文件状态更新或删除时无需提交。
- 无法突破的阻塞只在本文件记录，标记 `已阻塞` 并写清命令和错误。若仅无关问题阻断验证，标记 `已完成，待回归`，按相同提交规则处理并保留本文件。
- `plan.md` 为只读架构指南。
