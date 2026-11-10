# 任务 57: 转换Responses到Anthropic请求

批次：【批次 56】 依赖：批次 55 的任务 56。外部前置：核实母任务 1 的来源快照与母任务 2、3 的对应运行时、执行主机和应用能力。原生配置写入、账号上游分别等母任务 4、6 的对应契约。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：指令、输入、图文/文件及工具回合转Anthropic，处理leading user和不完整工具回合。

来源任务：母任务8；本目录来源映射对应条目。

预计会话范围：一个聚焦会话只处理下述行为与其测试，通常按30–60分钟工作块组织，不承诺构建耗时。跨协议请求/JSON/SSE分项；不顺带重构相邻模块。

## 文件

- 新建：`crates/vw-agent/src/model_relay/responses_anthropic_request.rs`（只放来源差异行为）；优先扩展同职责现有函数，禁止重复整套转换器。
- 读取并按需要最小修改：`crates/vw-agent/src/model_relay/transport.rs`、`mod.rs`，以及本项对应的chat.rs/messages.rs/messages_stream.rs/responses.rs/responses_stream.rs/gemini.rs/gemini_stream.rs。
- 修改：`crates/vw-agent/src/app_routing/dispatch.rs`（本行为执行分发）。
- 测试：新建独立文件`crates/vw-agent/src/model_relay/responses_anthropic_request_tests.rs`，至少一个测试命名为`cc_route_057_responses_anthropic_request_expected_behavior`；用父模块`#[cfg(test)]`显式注册。
- 修改：上述新模块直接父级mod.rs；首次建目录时仅补必要模块接线，不重排其他任务模块。
- 证据：`docs/shiju/agent-apps/routing-macos-acceptance.md`，仅本项条目。

## 上下文

- 来源固定提交及哈希见来源映射；源根`/Users/xiangmin/worker/code/cc-switch/`，首项引入完成后优先读取仓库upstream快照。
- 先读真实调用者、相邻独立测试和底座合同；新增路径为拟定落点，同职责已存在则沿用并在本任务证据记录实际路径。
- 先读来源：`/Users/xiangmin/worker/code/cc-switch/src-tauri/src/proxy/providers/transform_codex_anthropic.rs`及直接调用者/相邻测试。

## 实现边界

- 指令、输入、图文/文件及工具回合转Anthropic，处理leading user和不完整工具回合；不同时实现流转换。
- 以任务2公共传输和任务3可信作用域为入口；常驻来源扩展策略与旧会话严格策略显式区分，不能全局取消旧relay字段白名单或身份验证。纯格式转换不得读取凭据、执行工具、进入session/prompt；工具只返回给客户端。
- 只验收macOS；不删除现有其他平台实现，不执行Windows/Linux/WSL构建或模拟通过；WSL连接地址保留来源映射，标记范围外/未验证。
- 设计取舍记入任务证据，实质范围变化才集中确认。缺少底座时只登记具体接口和恢复条件。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-20`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–20。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 20 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 测试计划

- 行为：指令、输入、图文/文件及工具回合转Anthropic，处理leading user和不完整工具回合；不同时实现流转换。
- 新增失败测试：`crates/vw-agent/src/model_relay/responses_anthropic_request_tests.rs`内`cc_route_057_responses_anthropic_request_expected_behavior`；使用`cc_route_057_`前缀组织本项边界用例，不声称这些测试已存在。
- 失败验证命令：`cargo test -p vw-agent --lib cc_route_057_ -- --nocapture`。
- 预期失败原因：Responses工具回合或媒体输入产生无效Messages。若代码原已满足，补上当前缺失场景证明后再做最小接线，不制造假失败。新增符号尚不存在时先让测试编译，红灯须来自行为断言，不能只提供找不到符号证据。
- 通过验证命令：与失败验证命令完全相同；有效测试至少1个、失败0、退出码0，所有本项边界用例均通过。
- crate/features：`vw-agent`使用当前默认features；不添加推测feature；API任务还运行vw-gateway-client默认features。vw-agent-apps当前未注册，必须等母任务1/2交付可测试lib后才可运行该包命令。
- 模拟策略：真实有限回环HTTP/TCP上游记录线协议并控制慢响应、错误、压缩与SSE断线；只替代不可控外部模型，不mock内部授权/路由/转换/SDK。fixture不得包含真实密钥。

## 步骤

1. 确认底座合同、父模块注册和本行为现有调用链；仅沿用一个真实所有者。
2. 新增上述独立失败测试；领取Cargo槽位，运行红灯命令，确认失败来自目标行为。
3. 实现本项最小变更；接线不扩展到其他协议或控制面行为。
4. Rust修改后运行`cargo fmt --all`，审查并排除无关格式化差异，再运行同一聚焦命令。
5. 执行`cargo clippy --all-targets --all-features -- -D warnings`；只修本次引入问题。格式化/全仓Clippy阶段与其他代码任务串行，不因槽位允许并发而并行写格式化。
6. 将本项实现符号/API或数据面端点/SDK/UI与证据写正式验收文档，资源退出后释放自己的槽位。

## 验证

- 运行：`cargo test -p vw-agent --lib cc_route_057_ -- --nocapture`；随后上述格式化与全features Clippy。
- 所需证据：当前红灯断言、绿灯退出码与有效测试数、边界用例结果、差异审查、实际槽位及绝对CARGO_TARGET_DIR；只编译成功或零测试不算通过。

## 完成与阻塞

- 仅本项证据齐全时标`已完成`；有实际代码/测试/配置/文档变更，按`$git-commit`核对并只提交本任务变更，报告证据后删除本任务文件。只有本任务状态/删除则不单独提交。
- 缺实际依赖为`已阻塞`，仅在本文件记录明确原因、复现和已运行检查；先在本任务边界内尽力解决。无关既有错误导致回归无法执行时可为`已完成，待回归`，保留任务文件，不假报验证。
- 不修改plan.md、母计划状态、其他任务或无关工作区变更；prompt.md不是进度来源。不得为解除阻塞实现另一独立母能力。
