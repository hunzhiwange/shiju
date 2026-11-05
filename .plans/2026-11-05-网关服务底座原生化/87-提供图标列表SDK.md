# 任务 87: 提供图标列表SDK

批次：【批次 87】 依赖：批次 86（前置累计包含批次 1–86）。本批只有本任务；不安排并行。线性顺序见 prompt.md。

状态：未开始

目的：让桌面以强类型 SDK 读取图标列表。

来源任务：`.plans/2026-11-05-CC能力完整融合/2-网关服务底座与原生化边界.md`；来源快照与能力交叉分工见母目录能力覆盖.md、来源清单.json。

预计会话范围：只完成本文件限定的一组盘点或一个行为；通常一个聚焦会话。不要在此会话执行后续任务。若实际入口拆分，仅核对等价路径并记录本任务；不得借机迁移其他领域。

## 文件

- 新建：`crates/vw-gateway-client/src/client/agent_apps_api.rs`
- 新建：`crates/vw-gateway-client/src/client/agent_apps_api_tests.rs`
- 修改：`crates/vw-gateway-client/src/client/mod.rs`

## 上下文

- 仓库根 AGENTS.md 与 `skills/rustcodegraph/SKILL.md` 优先；读现有调用者、工厂连线和相邻测试后再写。
- 母任务 1 来源快照验收完成，且本计划前置批次均有交付证据；当前包尚不存在的事实不能被当作已交付。
- 仅验收 macOS；不要求 Linux/Windows 实机、交叉编译或 wasm；未验收不能宣称跨平台通过。

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

- crate：`vw-gateway-client`；聚焦测试使用默认 features，无额外 feature；Clippy 使用全部 features。
- 行为：SDK 发送既有认证/租户头并解析图标或结构化业务错误。
- 失败验证测试：上述独立测试文件中新增 `cc_base_icon_sdk_read_` 命名的测试；末尾下划线代表同前缀多个行为用例，至少包括正文指定的正常/失败边界。
- 失败验证命令：`cargo test -p vw-gateway-client --lib cc_base_icon_sdk_read_ -- --nocapture`。
- 预期失败原因：当前没有本任务新增行为或接线，新增断言失败；首次新增公共符号导致编译失败也须记录，实施后必须观察真实行为断言通过。每个测试过滤前缀必须匹配正文要求的独立用例，最终报告实际用例名与数量。禁止将零测试当预期失败。
- 通过验证命令：`cargo test -p vw-gateway-client --lib cc_base_icon_sdk_read_ -- --nocapture`。输出必须包含目标测试且有效数量大于零；实际数量逐项记录。
- 模拟策略：纯业务用真实数据；存储用真实临时 SQLite/隔离 PostgreSQL；API 用真实 Router 和数据库；SDK 只在传输测试替代远端 HTTP 边界；最终无窗口链路禁止 mock 业务与数据库。

## 步骤前置

1. 在上述独立测试文件写出本行为失败场景，显式注册测试模块（vw-agent 的 autotests=false）。
2. 先运行失败验证，保存确切失败断言或缺少符号；确认是本任务预期原因。
3. 再按以下步骤实现最小变更，不复制整个上游模块。

## 步骤

1. 在 GatewayClient 定义 custom_icons_list，沿用当前 base URL 和 request helpers；不增加客户端本机存储。
2. 用现有 test_support 的真实回环 HTTP server 检查路径、header、中文及错误 JSON。该 server 仅替代网络边界，不证明服务业务正确。
3. 断线恢复通过重新列表读取，不增加未经后端支持的图标事件订阅方法。

## 验证

- Rust 修改后先运行 `cargo fmt --all`，再运行 `cargo test -p vw-gateway-client --lib cc_base_icon_sdk_read_ -- --nocapture`。
- 必须运行 `cargo clippy --all-targets --all-features -- -D warnings`。不要运行全量单元测试、fmt check、wasm 或 Makefile。仅本任务格式化差异可保留，其他既有差异不能提交。
- 在 macOS 记录实际槽位、绝对 CARGO_TARGET_DIR、确切命令、退出码、有效测试数；PostgreSQL 有关测试不得跳过后宣称通过。
- 结构编译、mock 请求记录或旧运行结果都不能替代本任务当前行为证据。

## 完成

仅允许状态：`未开始`、`已完成`、`已阻塞`、`已完成，待回归`。只在本文件记录阻塞及检查，不修改只读 plan.md 或其他任务。不得顺手扩大到领域实现；新需求先询问是否记录回来源产物。

拿到本任务当前证据后才可标记完成；有实际代码、测试、配置或正式文档变更，使用 `$git-commit` 检查并仅提交本任务实际变更。只有任务状态更新或删除不单独提交。报告证据后删除本任务文件；无关既有错误阻断回归时可标记 `已完成，待回归` 并保留，不能宣称验证成功。真正缺前置依赖则标 `已阻塞` 并保留。
