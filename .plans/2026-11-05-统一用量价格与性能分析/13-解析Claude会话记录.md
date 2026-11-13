# 任务 13: 解析Claude会话记录

批次：【批次 3】 依赖：批次 2 的任务 2。外部前置见接口契约.md。同批实施与资源规则见 prompt.md。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：真实Claude assistant JSONL解析为带稳定消息身份和缓存分项的用量，非assistant和损坏行不产生计费。

来源任务：`.plans/2026-11-05-CC能力完整融合/17-统一用量价格与性能分析.md`，需求映射见来源覆盖.md。

预计会话范围：只验证“解析Claude会话记录”这一故事；不扩展到其他适配器或完整用量中心。通常一个聚焦会话，参考30—60分钟；编译缓存未命中耗时单列，不能因此合并更多行为。

## 文件

- 拟新增/按前置交付修改：`crates/vw-agent-apps/src/usage/claude.rs`。
- 拟新增/按前置交付修改：`crates/vw-agent-apps/src/usage/mod.rs`。
- 独立测试：`crates/vw-agent-apps/tests/usage_claude.rs`。
- 必要连线：对应父模块测试注册；新业务包集成测试按前置Cargo.toml的autotests配置显式注册。不为该任务改feature或另建crate。

## 上下文

上游入口`/Users/xiangmin/worker/code/cc-switch/src-tauri/src/services/session_usage.rs`；先读其相邻测试和真实调用者。全部目标文件的“拟新增”定义见接口契约.md；不得把尚未交付的包当现有模块。

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

## 测试计划

- 行为：真实Claude assistant JSONL解析为带稳定消息身份和缓存分项的用量，非assistant和损坏行不产生计费。
- 失败验证测试：在`crates/vw-agent-apps/tests/usage_claude.rs`新增`usage17_claude_`前缀的行为测试组，主测试命名为`usage17_claude_assistant_message_preserves_identity_and_cache`；每个边界独立断言，有效测试数至少为1。
- 失败验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent-apps --test usage_claude usage17_claude_ -- --nocapture`。
- 预期失败原因：当前缺少本任务的解析Claude会话记录行为，断言不满足；若新API尚未定义，先记录编译失败，再让测试编译后取得行为断言失败。依赖缺失或零测试不算预期失败。
- 通过验证命令：先`cargo fmt --all`，再`python3 scripts/cargo_slot.py cargo test -p vw-agent-apps --test usage_claude usage17_claude_ -- --nocapture`，最后`python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`。
- crate：`vw-agent-apps`；聚焦测试用默认features，Clippy使用全部features；仅macOS当前主机，无其他平台target。
- 模拟策略：文件/SQLite/服务/Router/SDK用真实临时资源；仅外网拉取、不可控原生外部进程及时间边界可替代，须记录真实格式与替代边界。不得mock本任务核心逻辑。

## 步骤

1. 先检查git工作区、领域前置及已有调用者，按.plans/2026-11-05-CC能力完整融合/17-统一用量价格与性能分析.md与接口契约.md核对本任务范围；不提交无关变更。
2. 在独立测试文件建立上述失败场景，注册测试；领取Cargo共享槽位后执行失败命令并保存实际失败证据。
3. 沿用Claude项目路径解析和message.id；支持项目/会话关联、真实供应商未知保留未知；只实现单文件解析，不安排调度。
4. 只实现让上述行为通过的最小代码；单文件约500行时按本领域子职责拆开，不能在业务文件内塞单测。
5. 执行格式化、相同聚焦测试和强制Clippy；必要重构只在测试保持通过时进行，结束前释放槽位。

## 验证

- 运行：`python3 scripts/cargo_slot.py cargo test -p vw-agent-apps --test usage_claude usage17_claude_ -- --nocapture`；Rust修改后另执行`cargo fmt --all`和`python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`。
- 预期：行为测试有效数大于0且全部通过，未知/失败/权限与正常分支可辨；仅编译通过或过去测试结果不算验收。
- 所需证据：新增失败测试路径与名称、预期失败输出、通过输出/退出码/有效数、实际macOS版本与架构、真实文件或API结果、差异审查，以及槽位/绝对CARGO_TARGET_DIR。涉及外网替代须说明来源和数据形状。

## 完成与阻塞

- 取得本项当前证据才标记`已完成`；有实际代码/测试/配置/文档变更时使用`$git-commit`仅提交本项，再在最终回复报告证据并删除本编号文件。只有编号文件状态/删除时不单独提交。
- 无关既有编译错误不代修；记录确切命令与错误，可标记`已完成，待回归`并保留任务文件。真正缺领域前置且不能在本任务内解决时标记`已阻塞`，原因和检查只记录本编号文件，不能假造接口或自行补做整个母能力。
- 禁止修改本目录或母目录的plan.md，不把prompt.md当进度来源；母17本轮维持未开始。若确需扩大设计/新增需求先询问是否记录回来源，不能静默扩范围。
- 不执行无过滤cargo test、fmt --check、wasm检查或任何Makefile命令；不修复或提交其他会话改动。所有验收仅macOS。
