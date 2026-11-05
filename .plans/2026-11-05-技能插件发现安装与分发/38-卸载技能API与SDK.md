# 任务 38: 卸载技能API与SDK

批次：【批次 38】 依赖：批次 37（累计前置批次 1–37）。本批仅此任务，不安排并行。线性顺序为 1 → 2 → … → 57，可复制入口见 prompt.md。

状态：未开始

目的：卸载响应传递备份和保留外部文件信息。

来源任务：`.plans/2026-11-05-CC能力完整融合/12-技能插件发现安装与分发.md`；具体来源：本计划业务任务 uninstall；上游commands/skill.rs或commands/plugin.rs。上游相对路径基于 `/Users/xiangmin/worker/code/cc-switch/`，固定来源与许可保持不变。

预计会话范围：只完成上述一个行为或一个API/SDK小族；通常一个聚焦会话。不要继续后续任务，不做跨领域重构；若测试揭示新的独立行为，先记录范围问题并确认，不把会话扩成整块技能系统。

## 文件

- 拟新增或修改：`crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/uninstall_api.rs`。
- 拟新增或修改：`crates/vw-api-types/src/agent_apps_skills/uninstall_api.rs`。
- 拟新增或修改：`crates/vw-gateway-client/src/client/agent_apps_skills/uninstall_api.rs`。
- 拟新增或修改：`crates/vw-gateway-client/src/client/agent_apps_skills/uninstall_api_tests.rs`。
- 独立测试（拟新增）：`crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/uninstall_api_tests.rs`。

## 上下文

- 实施前核对首任务执行合同和母批次1–11验收：来源快照、vw-agent-apps正式workspace注册及依赖、作用域存储/授权/目标主机、14项应用注册与MCP边界必须已交付；缺失标记本任务已阻塞，不伪造接口，不在本会话补完全部前置。
- 以上新路径和 `cp12_uninstall_api` 测试前缀是本计划拟定义的接口，不是现存实现声明。执行前用 rustcodegraph 核实相邻真实调用者；已有等价模块则原位扩展并在本任务记录实际路径，禁止重复所有者。
- 新建的三个 agent_apps_skills 目录均含 mod.rs；分别从 handlers/mod.rs、vw-api-types/src/lib.rs、client/mod.rs 注册，client 公共导出沿用现有模式。网关路由接入 gateway/api/mod.rs 的已认证应用管理路由；在同族独立测试中注册SDK线级测试。
- 沿用显式控制流和强类型错误；真实文件系统与设置存储验证，禁止安装时执行脚本、记录密钥或将客户端路径当作目标主机路径。

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

- 行为：卸载响应传递备份和保留外部文件信息。
- 失败验证测试：独立文件 `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/uninstall_api_tests.rs`，拟新增明确测试名 `cp12_uninstall_api_success` 与 `cp12_uninstall_api_rejects_without_side_effects`，分别证明本任务成功行为及拒绝/失败后状态不变；可按表驱动增加本行为边界样本。将测试模块用 cfg(test) 显式注册，防止零测试。
- 失败验证命令（本任务定义后执行，默认features；不添加臆测feature）：
- `cargo test -p vw-agent --lib cp12_uninstall_api -- --nocapture`
- `cargo test -p vw-gateway-client --lib cp12_uninstall_api -- --nocapture`
- 预期失败原因：经真实Axum路由与SDK发送授权/越权/错误请求，当前路由缺失或结果字段丢失。先让测试进入业务断言失败；缺模块的编译失败仅作准备证据，不能代替行为失败。已有行为通过时保留通过证据并针对来源缺口补失败场景，不造假失败。
- 通过验证命令：重复以上聚焦命令；每个过滤至少命中1项，记录实际数量。新行为前后必须对应相同测试。
- 模拟策略：真实临时目录、权限、链接、归档、持久化与生产解析；远程HTTP/外部不可控服务可用固定本地响应，记录边界。API用真实路由与SDK，UI消息测试仅隔离网关外部连接，不能mock待验收业务本身。

## 步骤

1. 读取本任务相关现存模块、工厂接线和相邻测试，确认前置证据及文件占用，检查未提交差异。
2. 在独立测试文件编写上述失败场景，注册后执行失败验证命令，确认非零业务断言失败。
3. 本任务拟新增路由：DELETE /agent-apps/skills/{id}（尚不存在；若母批次2已指定同义路由，以其合同为准并在本任务记录）。仅负责这一API小族及相应DTO/SDK；先授权后访问文件系统；返回部分清理而非只布尔值；重复删除返回明确not_found。复用已验收作用域/执行端合同，未交付即阻塞，不反向实现整个母批次。
4. 完成本任务最小接线；只增加真实调用者所需类型和函数，不顺手铺后续抽象或改变权限。
5. 执行 `cargo fmt --all`，审查并排除无关格式化；运行通过验证命令，必要时仅在保持通过后重构。
6. 执行 `cargo clippy --all-targets --all-features -- -D warnings`，修复本任务引入的问题；既有无关错误只记录，不覆盖其他会话变更。

## 验证

- 聚焦命令、crate与features以上文为准；Clippy为workspace全部features，使用同一阶段领取槽位。
- 不运行无过滤全量 `cargo test`、wasm检查、Makefile、fmt check；仅验收macOS，不加Windows/Linux验收门槛。
- 预期：成功行为与失败后状态断言均通过，错误不泄露敏感信息；聚焦测试命中数大于0，Clippy退出0。
- 所需证据：失败断言/通过输出、确切命令、退出码、有效测试数量、相关符号与路径、实际槽位及绝对 CARGO_TARGET_DIR。文档/截图/请求结果写 `docs/vibewindow/agent-apps/skills/uninstall_api-证据.md`，不记录原始令牌或私有载荷。
- API证据附授权与越权请求/结果以及目标主机；桌面证据附真实页面明暗主题截图与无panic日志；缺真实渲染、账号或环境则明确未验证，不能用构造Element或编译成功替代。

## 完成与阻塞

仅使用 `未开始`、`已完成`、`已阻塞`、`已完成，待回归`。当前证据齐全才完成；只在本编号文件记录阻塞原因和已执行检查，不修改只读plan.md，不把prompt.md当进度来源。

保持本任务边界；缺前置先核对交付合同，只修复与本行为直接相关的最小缺陷，不在本会话实现整个依赖领域。真正缺依赖保持已阻塞；只有无关既有错误阻断回归时可标已完成，待回归并保留本文件，不能声称验证成功。

有实际代码、测试、配置或正式文档变更后，使用 `$git-commit` 检查并仅提交本任务变更；只有本任务状态更新或删除时不提交。报告完成证据后删除本任务文件；待回归或阻塞保留。不得提交工作区其他任务的变更。
