# 任务 3: 改为独立 CLI 技能契约

批次：【批次 3】 依赖批次 2

状态：已完成，待回归

目的：让打包与仓库内 RustCodeGraph 技能只描述外部 CLI，清除原生 Tool 承诺并完成全仓验证。

来源任务：无

预计会话范围：只更新 RustCodeGraph 技能副本、技能安装断言和直接依赖原生 Tool 的评测说明，随后执行仓库规定的最终 clippy。

## 文件

- 修改：`crates/vw-agent/assets/skills/rustcodegraph/SKILL.md`
- 修改：`skills/rustcodegraph/SKILL.md`
- 修改：`crates/vw-agent/src/skill/install_tests.rs`
- 修改：`evals/promptfoo/README.md`
- 修改：`evals/promptfoo/assertions.mjs`

## 上下文

- 仓库 skill 已是 CLI-first，打包 skill 仍优先原生 Tool 并内置多平台 CLI 定位逻辑；两份契约应统一为不依赖 VibeWindow 内置工具的外部 CLI 工作流。
- 技能与安装别名本身继续保留；不要删除 bundled skill、推荐技能入口或 shell PATH 对外部 CLI 的支持。
- Cargo crate 为 `vw-agent`，最终验证覆盖 workspace `--all-targets --all-features`；仓库禁止未授权运行 `cargo test`，因此测试过滤条件不适用，技能契约以静态检查和全量 clippy 验收。

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

- 测试驱动开发不适用：本任务以技能与评测契约删除为主，且仓库明确禁止未授权运行 `cargo test`；使用静态契约检查与全量 clippy 验证。
- 行为：新安装技能不会要求或宣传 VibeWindow 原生 RustCodeGraph Tool，仍能指导代理通过 CLI 使用已有索引。
- 失败验证：更新安装测试与静态检查，使其拒绝“原生工具优先”文案；旧 bundled skill 应产生命中。
- 失败验证命令：`rg -n '原生.*rustcodegraph|native.*rustcodegraph|direct_rustcodegraph_call|rustcodegraph_operation|rustcodegraph_call_success' crates/vw-agent/assets/skills crates/vw-agent/src/skill evals/promptfoo`
- 预期失败原因：当前 bundled skill、安装测试或评测文档仍以原生 Tool 为能力前提。
- 通过验证命令：`cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings`
- 模拟策略：不使用 mock；通过真实打包资源安装测试目标的编译检查和静态文案审查验证契约。

## 步骤

1. 对齐仓库与 bundled skill，保留 CLI 的索引检查、查询、回退和安全边界，删除原生 Tool 入口与双路径分支。
2. 更新技能安装测试，只验证 CLI-first 契约和未索引时回退行为。
3. 删除或改写只评估内置 Tool 直接调用的 promptfoo 指标与说明，不影响其它工具评测。
4. 全仓搜索残余引用，区分应保留的技能、CLI、PATH 和普通 MCP fixture 与应删除的原生 Tool 耦合。
5. 按槽位规则领取动态槽位、设置绝对 `CARGO_TARGET_DIR`，运行 `cargo fmt --all` 与仓库规定的全量 clippy；无论成功、失败或中断都在相关进程结束后释放自己的锁。
6. 若验证失败来自无关既有改动，记录证据并标记 `已完成，待回归`；随后使用 `$git-commit` 提交本任务变更。

## 验证

- 运行：`rg -n 'RustCodeGraphTool|register_rustcodegraph_tool|## RustCodeGraph|direct_rustcodegraph_call|rustcodegraph_operation|rustcodegraph_call_success' crates evals Cargo.toml || true`
- 运行：`cargo fmt --all`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：原生 Tool 专用实现、提示和评测零命中；技能与 CLI 引用继续存在；clippy 退出码 0，或只报告可明确归因于任务外既有改动的问题。
- 所需证据：静态零命中结果、保留技能/CLI 引用的审查清单、fmt 与全量 clippy 退出码、有效测试数量记为“未运行（仓库规则禁止）”、实际槽位、绝对 `CARGO_TARGET_DIR`、锁已释放、最终 diff 审查、提交哈希。

## 完成

完成时应确认 RustCodeGraph 只作为可选外部 CLI 技能存在，VibeWindow 不再提供或宣传内置 Tool，且仓库规定的 `cargo clippy --all-targets --all-features -- -D warnings` 已执行。

## 本次执行证据（2026-10-11）

- 依赖已满足：后端移除提交 13fd75115、目录清理提交 684d81ef0 已存在；前序验证受工具链问题影响。
- 两份技能完全一致，保留索引 status/sync、explore/query/node/调用图/affected、无索引与无 CLI 回退、不自动安装或初始化、shell 权限边界；bundled 别名与技能资源注册保留。通用 MCP fixture 和 shell PATH 未修改。
- 修改前 bundled 原生入口、安装测试与专用指标均命中；修改后指定全仓 rg 原生实现/注册/提示/指标零命中（rg 退出 1 表示无匹配）。移除三个引用已删除原生实现的 YAML 用例，保留其它导航场景并允许 CLI/普通搜索。
- Python 静态契约检查退出 0；node --check evals/promptfoo/assertions.mjs 退出 0；真实评分函数验证 CLI、普通搜索回退通过及失败工具拒绝，共 3 个场景，退出 0。未运行在线模型评测。
- cargo fmt --all 退出 0；cargo clippy --all-targets --all-features -- -D warnings 退出 101，任务外错误：crates/vw-desktop/src/app/task/models.rs:568 不稳定 atomic_try_update（E0658）；crates/vw-cli/src/lib.rs:39 未知 clippy::assert_is_empty lint。未改动这些文件。
- 有效 Rust 测试数量：未运行（仓库规则禁止 cargo test）；Clippy 不替代行为测试。
- 动态槽位 1，CARGO_TARGET_DIR=/Users/dyhb/code/vibe-window/target/rust-slot-1；CARGO_BUILD_BUILD_DIR 同路径覆盖仓库共享 build-dir。Cargo 全部结束后锁与 owner 已释放，编译缓存保留。
- 最终 diff 与 git diff --check 已审查通过；排除工作区其他 Lark 和桌面设置改动。plan.md 保持只读。
- 下一步：修复上述任务外工具链问题后重跑全量 Clippy，通过后删除本任务文件。
