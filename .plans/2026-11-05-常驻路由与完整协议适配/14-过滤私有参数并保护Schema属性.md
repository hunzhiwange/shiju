# 任务 14: 过滤私有参数并保护Schema属性

批次：【批次 14】 依赖：批次 13 的任务 13。外部前置：核实母任务 1 的来源快照与母任务 2、3 的对应运行时、执行主机和应用能力。原生配置写入、账号上游分别等母任务 4、6 的对应契约。

状态：已阻塞

### 集成工作区复核（2026-11-10，macOS 27.0 arm64）

- 在 `/Users/xiangmin/worker/code/shiju` 的 `shiju` 分支（`b6fb3a2fc`）确认任务 13 已由 `5255827c6` 交付。workspace 已注册 `vw-agent-apps`，母任务 2、3 的网关、执行主机授权及应用目录可复用。母任务 4 的请求选项与模式事务、母任务 6 的凭据租约契约已存在。任务 109 仍阻塞在验证阶段，其范围仅为可信绑定准备与网关组装，生产凭据发布归任务 86。
- 对上游快照及 CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `body_filter.rs` 运行 `shasum -a 256`，与来源映射均为 `64b6e6c0f69ac17e6531584512c99f08bf2a6d5398615a2f3e330ebb478db824`。核对 `forwarder.rs::prepare_upstream_request_body` 的调用与相邻测试。来源在发送前使用空白名单递归过滤，保留 Schema 名称映射中的属性名，并继续过滤属性定义里的私有参数。
- 生产调用链为网关 `bootstrap` → `gateway_owner::run_with_routing` → `RoutingListener::start_http` → `route_auth::serve`。鉴权通过后，`listener.rs::serve_http` 的闭包忽略正文与绑定，直接返回 503。
  - `rg --files crates/vw-agent/src/app_routing` 确认无 `dispatch.rs`。`rg -n 'SharedTransport|filter_private_params|prepare_upstream_request_body' crates --glob '*.rs' --glob '!**/upstream/**'` 仅找到共享传输定义、导出和测试调用。
  - 生产 `upstream::execute` 的另一调用者使用短期 `RelayBinding`，共用层只接收成形 HTTP 请求。任务 13 的保存入口只校验供应商档案。这些入口均无已授权的常驻 JSON 正文可供本项过滤。
- **调度存在隐式依赖环。** 用 Python `graphlib.TopologicalSorter` 检查《执行索引》。显式依赖可排序，但补上实际缺失的分发依赖后成环。
  - 任务 33 最早把修改 `dispatch.rs` 列为交付项，其行为是 SSE 分帧。首个有正文的原生 HTTP 转发任务 34 经 33、32、31、29、28–21、20、18–15 传递依赖任务 14。把 33、34 或生产凭据发布任务 86 加为 14 的前置均成环。任务 109 可以独立推进，但只完成它仍缺 JSON 分发。
- **恢复条件。** 将首个已授权的生产 JSON 分发接线从下游协议任务中拆出，解除其对任务 14 的传递依赖。接线须由现有监听使用，在可信目标与代次复核后形成 JSON 正文，经 `SharedTransport::forward` 发送。
  - 分发接线与过滤合入时须保持未就绪路径拒绝转发。然后在发送前唯一的正文处理点接入过滤，执行本卡红绿验证，并以生产入口和受控回环上游复核 Schema 保留与私有字段移除。
- 本轮执行了 RustCodeGraph 调用链导航、源码检索、来源哈希和调度图检查。缺生产入口，未运行聚焦红绿测试、格式化或 Clippy，未领取槽位，未设置 `CARGO_TARGET_DIR`。

### 授权补齐与验证进度

- 用户已授权在集成目录补齐最小 JSON 生产分发接线。接线使用同一常驻监听与授权绑定，仅为已绑定目标的 Codex 非流式 Chat JSON 提供发送点。生产凭据及目标发布仍由既有事务所有者负责。当前接线和夹具在集成目录，验证进行中。
- 首轮 `cargo test -p vw-agent-apps --lib cc_route_014_ -- --nocapture` 在槽位 3（`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`）退出 101，执行 0 项。独立 OMP 改动 `providers/omp.rs:339` 对 SHA256 输出使用 `LowerHex` 导致 E0277。已释放槽位 3，保留缓存。
- 复核以 `22617ffe6` 叠加本项 Rust 文件的源码副本执行，未创建 Git 工作树。槽位 15（`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-15`）已领取，格式化、红绿测试和全仓 Clippy 正在排队或执行。集成目录的其他任务改动不进入本项源码副本。

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：递归过滤私有参数但保留工具JSON Schema中以下划线开头的真实属性。

来源任务：母任务8；本目录来源映射对应条目。

预计会话范围：一个聚焦会话只处理下述行为与其测试，通常按30–60分钟工作块组织，不承诺构建耗时。跨协议请求/JSON/SSE分项；不顺带重构相邻模块。

## 文件

- 新建或复用：`crates/vw-agent-apps/src/routing/private_params.rs`。
- 测试：新建独立文件`crates/vw-agent-apps/src/routing/private_params_tests.rs`，至少一个测试命名为`cc_route_014_private_params_expected_behavior`；用父模块`#[cfg(test)]`显式注册。
- 修改：上述新模块直接父级mod.rs；首次建目录时仅补必要模块接线，不重排其他任务模块。
- 证据：`docs/shiju/agent-apps/routing-macos-acceptance.md`，仅本项条目。

## 上下文

- 来源固定提交及哈希见来源映射；源根`/Users/xiangmin/worker/code/cc-switch/`，首项引入完成后优先读取仓库upstream快照。
- 先读真实调用者、相邻独立测试和底座合同；新增路径为拟定落点，同职责已存在则沿用并在本任务证据记录实际路径。
- 先读来源：`/Users/xiangmin/worker/code/cc-switch/src-tauri/src/proxy/body_filter.rs`及直接调用者/相邻测试。

## 实现边界

- 纯策略必须由真实执行调用者使用：`crates/vw-agent/src/app_routing/dispatch.rs`或任务1确认的既有调用点负责接线；状态机由listener消费，请求覆盖由母任务4既有API消费。若本项开始时无真实调用者，不添加孤立工具/trait，先记录依赖；不能只用纯函数绿灯声称线上行为已交付。
- 纯策略必须由真实执行调用者使用：`crates/vw-agent/src/app_routing/dispatch.rs`或任务1确认的既有调用点负责接线；状态机由listener消费，请求覆盖由母任务4既有API消费。若本项开始时无真实调用者，不添加孤立工具/trait，先记录依赖；不能只用纯函数绿灯声称线上行为已交付。
- 递归过滤私有参数但保留工具JSON Schema中以下划线开头的真实属性；白名单只作用于来源允许位置，不扩展认证和控制字段。
- 纯策略不持有数据库、桌面或agent依赖；执行端提供经授权的输入。若现有同职责函数已满足行为，直接复用并补覆盖，不建立平行规则源。
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

- 行为：递归过滤私有参数但保留工具JSON Schema中以下划线开头的真实属性；白名单只作用于来源允许位置，不扩展认证和控制字段。
- 新增失败测试：`crates/vw-agent-apps/src/routing/private_params_tests.rs`内`cc_route_014_private_params_expected_behavior`；使用`cc_route_014_`前缀组织本项边界用例，不声称这些测试已存在。
- 失败验证命令：`cargo test -p vw-agent-apps --lib cc_route_014_ -- --nocapture`。
- 预期失败原因：工具Schema合法属性被误删或私有控制参数泄漏。若代码原已满足，补上当前缺失场景证明后再做最小接线，不制造假失败。新增符号尚不存在时先让测试编译，红灯须来自行为断言，不能只提供找不到符号证据。
- 通过验证命令：与失败验证命令完全相同；有效测试至少1个、失败0、退出码0，所有本项边界用例均通过。
- crate/features：`vw-agent-apps`使用当前默认features；不添加推测feature；API任务还运行vw-gateway-client默认features。vw-agent-apps当前未注册，必须等母任务1/2交付可测试lib后才可运行该包命令。
- 模拟策略：直接使用真实JSON/字节/策略输入输出做表驱动断言，不mock被测策略；仅时间或账号查询等非确定边界可注入，数据形状对应来源真实请求。

## 步骤

1. 确认底座合同、父模块注册和本行为现有调用链；仅沿用一个真实所有者。
2. 新增上述独立失败测试；领取Cargo槽位，运行红灯命令，确认失败来自目标行为。
3. 实现本项最小变更；接线不扩展到其他协议或控制面行为。
4. Rust修改后运行`cargo fmt --all`，审查并排除无关格式化差异，再运行同一聚焦命令。
5. 执行`cargo clippy --all-targets --all-features -- -D warnings`；只修本次引入问题。格式化/全仓Clippy阶段与其他代码任务串行，不因槽位允许并发而并行写格式化。
6. 将本项实现符号/API或数据面端点/SDK/UI与证据写正式验收文档，资源退出后释放自己的槽位。

## 验证

- 除策略测试外审查真实调用点与执行链证据，证明不是未使用函数；涉及网络行为仍须通过后续对应HTTP任务验证。
- 除策略测试外审查真实调用点与执行链证据，证明不是未使用函数；涉及网络行为仍须通过后续对应HTTP任务验证。
- 运行：`cargo test -p vw-agent-apps --lib cc_route_014_ -- --nocapture`；随后上述格式化与全features Clippy。
- 所需证据：当前红灯断言、绿灯退出码与有效测试数、边界用例结果、差异审查、实际槽位及绝对CARGO_TARGET_DIR；只编译成功或零测试不算通过。

## 完成与阻塞

- 仅本项证据齐全时标`已完成`；有实际代码/测试/配置/文档变更，按`$git-commit`核对并只提交本任务变更，报告证据后删除本任务文件。只有本任务状态/删除则不单独提交。
- 缺实际依赖为`已阻塞`，仅在本文件记录明确原因、复现和已运行检查；先在本任务边界内尽力解决。无关既有错误导致回归无法执行时可为`已完成，待回归`，保留任务文件，不假报验证。
- 不修改plan.md、母计划状态、其他任务或无关工作区变更；prompt.md不是进度来源。不得为解除阻塞实现另一独立母能力。
