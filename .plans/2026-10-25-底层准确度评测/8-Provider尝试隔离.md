# 任务 8: Provider尝试隔离

批次：【批次 8】

依赖批次：5、7

状态：未开始

目的：只把新题的每次调用接入独立目录、会话和实验metadata。

来源任务：旧任务3（本次细化替代旧编号）

预计会话范围：单会话边界：最多一个主要行为；仅列出的生产/评测文件和相邻独立测试。遇到另一独立缺陷或新增基础设施需求只记录依赖缺口，不顺手实现。

## 文件

- `修改 evals/promptfoo/vibewindow-provider.mjs`
- `修改 evals/promptfoo/vibewindow-provider.test.mjs`

## 上下文

callApi内let sessionId已使不同调用初始会话分开；同一次turns必须继续复用，不要误判已有行为缺陷。

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

- 行为：只把新题的每次调用接入独立目录、会话和实验metadata。
- 失败验证测试：`provider_isolates_attempt_directories_and_keeps_turn_session`，位于上列独立测试文件。
- 失败验证命令：`node --test evals/promptfoo/vibewindow-provider.test.mjs evals/promptfoo/attempt-workspace.test.mjs`。
- 预期失败：新增行为尚未实现，断言失败或新模块缺失；不能把无关环境错误当作所需失败。
- 通过验证命令：`node --test evals/promptfoo/vibewindow-provider.test.mjs evals/promptfoo/attempt-workspace.test.mjs`。
- 模拟策略：真实fixture/文件系统/解析器；只替身HTTP、模型或Harbor环境边界，记录输入输出形状。不得用mock断言代替最终产物验收。

## 步骤

1. 复用已有sseResponse与fetch替身，新增同一Provider两次调用、同题多轮、第二轮HTTP错误三个场景。
2. 仅显式runtime-accuracy任务使用任务7目录；旧profile保持directory配置；将workRoot放入X-VibeWindow-Directory，每次调用首轮不带上次session_id。
3. 附加attemptId、taskId、split、manifestDigest、期望重复次数与fixtureDigest到成功和失败结果metadata；异常路径保留已收到轨迹和分类，不丢失诊断证据。
4. 工作目录必须保留到任务10验收完成；本任务只传递生命周期，不在返回前删目录。finally处理HTTP计时器，受控清理入口以任务7契约为准。

## 验证与完成证据

- 既有四项Provider测试保持通过；首轮无串会话、后续轮复用session；失败保留身份与目录供验收。
- 补充运行：`evals/promptfoo/score.sh test`（仅Promptfoo相关任务）；它只验证语法与配置，不能代替上述Node/Python测试。
- 代码变更按仓库规则运行：`cargo clippy --all-targets --all-features -- -D warnings`；既有无关错误只记录。只在改Rust时执行 `cargo fmt --all` 并排除无关格式化。
- Cargo crate：运行时 `vw-agent`、入口 `vw-cli`；无Rust行为修改时不新增Rust测试。Gateway/build使用vw-cli默认features，clippy使用workspace全部targets/features；不运行全量cargo test、Makefile、wasm检查或fmt --check。
- 记录失败到通过的日志（如适用）、确切命令、退出码、有效测试/尝试数量、输入/输出摘要。运行结果含源码或工具载荷时只存results忽略目录。

## 补充契约

runtime-accuracy接入前先确认metadata来源是可信任务清单，不接受Agent输出改变taskId、split或manifestDigest。workRoot生命周期由Provider拥有；中间失败保留轨迹和身份的改动仅服务这一尝试契约，不重构全局SSE解析。

## 范围外

- 不修改被测Agent代码、其他类别题目或旧profile评分，不执行后续任务。
- 不为当前模型提分放宽oracle、阈值、权限或预算；新需求不自动写回其他来源产物。

## 完成

满足当前任务全部证据后才能标记 `已完成`。无关编译改动阻断最终检查时标记 `已完成，待回归` 并保留文件；依赖/环境缺失标记 `已阻塞`，原因只写本文件。有实际代码、测试、配置或文档变更时使用 `$git-commit`，只提交本任务；仅任务文件状态或删除不提交。最终报告证据后删除已完成任务文件；plan.md只读。
