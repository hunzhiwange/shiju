# 任务 19: 实现OpenClaw环境设置

批次：【批次 3】 依赖：批次 2 的任务 3。同批实施与资源规则见 prompt.md。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：完成“实现OpenClaw环境设置”的单一交付与本文件规定的当前验证证据。

来源任务：母任务 15；逐命令/文件归属见来源映射.md。

预计会话范围：只实现本文件描述的单一行为；不同时搬迁其他应用、不重构共享基础设施。建议一个聚焦会话完成，先用过滤测试取得反馈。

## 文件

- 新建或补充：`crates/vw-agent-apps/src/openclaw_env.rs`。
- 独立测试：`crates/vw-agent-apps/src/openclaw_env_tests.rs`。
- 注册入口：`crates/vw-agent-apps/src/lib.rs`；仅登记本模块及测试，不搬入业务逻辑。
- 正式证据：`docs/shiju/agent-apps/workspace-macos-acceptance.md`，仅追加本任务条目。

## 上下文

- 先读：`openclaw_config.rs::get_env_config/set_env_config；src/components/openclaw/utils.ts`（源根 `/Users/xiangmin/worker/code/cc-switch/`，`src/` 前缀文件保持原路径；Rust 简写位于 `src-tauri/src/`）。
- 阅读接口边界.md、来源映射.md 与任务 1 正式合同。上述新建路径是本计划定义的落点；前置已有同职责模块则直接扩展并记录真实路径，禁止复制永久业务所有者。
- 只验收 macOS 原生目标，不添加 features；源码索引可用时先读仓库 rustcodegraph 技能。缺依赖不伪造通过，不修改其他计划或会话的改动。

## 实现边界

env 完整 JSON 对象读写，支持 vars/shellEnv 嵌套且不字符串化；空白/坏 JSON/数组/null 拒绝，{} 明确清空；敏感值仅经独立授权读取，列表、日志和错误不带原文，读写原生配置不破坏其他分节。

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

## 测试计划

- 行为：本任务目的对应的真实输入/结果，不验证模拟调用次数。
- 失败验证测试：`crates/vw-agent-apps/src/openclaw_env_tests.rs` 中新增 `openclaw_env_preserves_nested_values`（必要场景以该名字为前缀拆成独立测试）。先注册测试模块，再运行，不能以零测试或拼写错误作为失败证据。
- 失败验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent-apps --lib openclaw_env_preserves_nested_values -- --nocapture`。
- 预期失败原因：当前尚无本任务完整行为，新增场景在对应输出、拒绝或文件副作用断言处失败；缺待实现符号导致编译失败时记录原因，实现后还须证明行为断言有效。已有等价能力则先构造来源中的缺失边界场景，不制造人为失败。
- 通过验证命令：同上；使用 crate 默认 features，macOS 本机目标；不使用 --all-features 跑测试。
- 模拟策略：真实临时目录、原生 JSON5/YAML 与 IO；不改进程 HOME，不触碰用户数据；HTTP 探测用本地临时端口，外部浏览器/终端进程只在单元测试替换边界，macOS 验收再实测。

## 步骤

1. 先读本任务来源函数、调用者、相邻测试和前置真实合同，确认修改范围。
2. 编写并运行上述失败测试，记录确切失败和有效测试数。
3. 只实现本任务最小行为；接口/模块定义以本计划合同为准，不另造通用命令后门。
4. Rust 修改后先 `cargo fmt --all`，审查并撤除仅由本次格式化造成的无关差异（不得覆盖他人改动），再运行聚焦通过验证。
5. 执行仓库 Clippy；记录正式证据、槽位和失败/通过输出，只有必要时局部重构。

## 验证

- 聚焦：`python3 scripts/cargo_slot.py cargo test -p vw-agent-apps --lib openclaw_env_preserves_nested_values -- --nocapture`，退出码 0 且有效测试至少 1，所有前缀场景均实际执行。
- 必须：`python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`。本任务引入的问题必须修复；无关既有错误只记录，不顺手修复。
- 所需证据：失败与通过命令、退出码、有效测试数量、相关文件/符号、真实行为结果、实际槽位及绝对 CARGO_TARGET_DIR；有 API 的提供请求/响应与权限拒绝。

## 完成

- 本任务新证据齐备才标为 `已完成`；把结果追加到正式证据文档的本任务条目，不以修改 prompt.md 或 plan.md 记录进度。
- 真正缺依赖时尽可能在本任务边界内排障，无法解决则只在本任务标为 `已阻塞`，写明原因、已运行检查与恢复条件。无关既有错误导致无法回归时可用 `已完成，待回归` 并保留本文件，不伪称验收通过。
- 不扩大到其他子任务；发现新需求/设计或范围变化先询问是否记录回来源，不改只读 plan.md 和母任务状态。
- 有实际代码/测试/配置/文档变更时使用 `$git-commit` 审查并仅提交本任务变更；只有编号任务状态更新/删除时不提交。报告完成证据后删除已完成任务文件；阻塞/待回归文件保留。正式交付证据文档保留。
