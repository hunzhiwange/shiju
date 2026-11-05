# 任务 27: Claude原生登录切换

批次：【批次 27】 依赖批次 26；此前各批已验收；共同前置为母任务 1–3

状态：未开始

目的：切换本机已存在官方登录失败时不改变供应商配置；后续文件写失败恢复原登录。

来源任务：母任务 4；需求映射见本目录来源映射.md；源码入口如下。

预计会话范围：只证明“Claude原生登录切换”这一个行为，新增接口仅服务本行为，不同时迁移其他应用或重构共享设施。依赖完成后无需阅读其他全部任务。

## 文件

- 新建或沿用任务 1 确认的既有文件：`crates/vw-agent-apps/src/providers/claude_login.rs`。
- 新建或沿用任务 1 确认的既有文件：`crates/vw-api-types/src/app_providers/claude_login.rs`。
- 新建或沿用任务 1 确认的既有文件：`crates/vw-agent/src/gateway/api/handlers/app_providers/claude_login.rs`。
- 新建或沿用任务 1 确认的既有文件：`crates/vw-gateway-client/src/client/app_providers/claude_login.rs`。
- 修改：以上新模块的直接父模块注册文件；首次创建目录时补最少 `mod.rs` 并接到现有根。只登记本任务模块，不重排其他模块。
- 注册入口：`crates/vw-api-types/src/lib.rs`、`crates/vw-gateway-client/src/client/mod.rs`、`crates/vw-agent/src/gateway/api/handlers/mod.rs`、`crates/vw-agent/src/gateway/router.rs`；沿用母任务 2 的分域注册，不另开通用命令入口。

- 证据：`docs/shiju/agent-apps/provider-macos-acceptance.md`，只维护本任务条目。

## 上下文

上游来源根为 `/Users/xiangmin/worker/code/cc-switch/`，快照提交见来源映射。新增路径是拟定落点，任务 1 核对后沿用同职责已有模块。仅验收 macOS；业务归执行端，API/SDK 走母任务 2 权限和错误契约。

- 先读来源：`/Users/xiangmin/worker/code/cc-switch/src-tauri/src/services/provider/claude_official.rs` 及其直接调用者和相邻测试。
- 先读来源：`/Users/xiangmin/worker/code/cc-switch/src-tauri/src/services/provider/claude_direct.rs` 及其直接调用者和相邻测试。

## 实现边界

- 切换本机已存在官方登录失败时不改变供应商配置；后续文件写失败恢复原登录。
- 不实现 OAuth 授权，母任务 6 拥有托管账号；macOS Keychain 外部接口可以替身，但真实 Keychain 无证据必须记未验证，不能使用真实用户材料测试。
- 先搜索真实调用者和相邻测试；行为相同的现有实现直接接通，不能复制永久业务所有者。缺少依赖只在本任务登记，不能伪造前置完成。

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

- 行为：切换本机已存在官方登录失败时不改变供应商配置；后续文件写失败恢复原登录。
- 独立测试文件：`crates/vw-agent-apps/src/providers/claude_login_tests.rs`、`crates/vw-api-types/src/app_providers/claude_login_tests.rs`、`crates/vw-agent/src/gateway/api/handlers/app_providers/claude_login_tests.rs`、`crates/vw-gateway-client/src/client/app_providers/claude_login_tests.rs`；按各 crate 现有方式注册独立测试模块。
- 新增失败测试名：cc04_claude_login_failure_leaves_pointer_and_settings；cc04_claude_login_write_failure_restores_original。其余 DTO/SDK/Gateway 测试使用相同 `cc04_claude_login_` 前缀，各测试名追加实际证明行为，不验证 mock 次数。
- 失败验证命令（默认 features，不加额外开关）：

```bash
cargo test -p vw-agent-apps --lib cc04_claude_login_ -- --nocapture
cargo test -p vw-api-types --lib cc04_claude_login_ -- --nocapture
cargo test -p vw-agent --lib cc04_claude_login_ -- --nocapture
cargo test -p vw-gateway-client --lib cc04_claude_login_ -- --nocapture
```

- 预期失败原因：本标题要求的行为尚未实现或尚未接到真实调用链，至少一个测试在业务断言处失败。先让测试接线编译；仅“找不到模块/类型”或运行 0 个测试不算失败验证完成。
- 通过验证命令：上述相同过滤命令；修改后先 `cargo fmt --all`，再聚焦测试，最后 `cargo clippy --all-targets --all-features -- -D warnings`。
- features：vw-agent-apps 采用母任务 2 已注册的默认 features；其余相关 crate 均使用当前 Cargo.toml 默认 features。任务 1 必须确认包可用后才能执行，不为验证另造特性。
- 有效数量：业务包至少 2 个有效行为测试，其余每个列出 crate 至少 1 个有效测试。 不以编译成功或零测试代替验收。
- 模拟策略：临时目录/临时数据库/真实本地 HTTP 和 SDK 优先；只替代网络上游、钥匙串或不可控进程边界。使用假凭据，记录替代边界与返回数据形状；不修改真实用户配置。

## 步骤

1. 核对依赖证据与当前差异，只阅读本行为的来源、现有模块、父注册与调用者。
2. 在独立文件新增上述失败测试；领取槽位后运行失败命令并确认预期业务失败。
3. 实现本行为的最小变更。对外服务补强类型 DTO、权限 handler 和 SDK；内部能力接现有调用者，不添加任意文件/命令调用 API。
4. 接通本行为涉及的所有入口并运行聚焦验证；当前供应商编辑/首次激活等依赖事务的路径由所属应用适配负责，未接通不能声明 live 已成功。
5. 运行格式化、聚焦验证与强制 Clippy，审查并排除无关差异；只在通过后做必要局部整理。

## 验证

- 运行：上述确切聚焦命令与 `cargo clippy --all-targets --all-features -- -D warnings`；所有命令继承同一阶段领取的绝对 CARGO_TARGET_DIR。
- 预期：cc04_claude_login_failure_leaves_pointer_and_settings；cc04_claude_login_write_failure_restores_original 全部通过，目录/文件/状态与行为断言一致，权限拒绝没有副作用。
- 所需证据：红/绿断言结果、命令/退出码/有效数量、实际槽位与绝对 CARGO_TARGET_DIR、差异审查；对外行为附 SDK 请求→API 响应→执行端文件结果；macOS 记录系统/架构/应用版本。平台、账号、安装或运行时证据缺失需明确记未验证，不能将 mock 当真实平台通过。

## 完成

- 将本任务完成证据追加到 `docs/shiju/agent-apps/provider-macos-acceptance.md` 的本任务条目，包含来源函数、实现路径、API/SDK/UI 和 macOS 验证结果；该正式资料保留。不要改其他任务条目。

- 只有本任务要求的新证据齐备才标记 `已完成`；因无关既有错误不能回归时使用 `已完成，待回归` 并保留任务文件；真正缺依赖时 `已阻塞` 并仅在本任务记录原因、已尝试检查与恢复条件。
- 不修改任何 plan.md，不向 prompt.md 写进度。不得扩大到其他任务或全仓清理；有新需求或新设计先询问是否回写来源，不自行改母计划。
- 有代码/测试/配置/文档实际变更时使用 `$git-commit` 审查并只提交本任务变更；只有编号任务状态更新/删除不提交。最终回复报告证据后删除已完成任务文件；待回归或阻塞文件保留。正式证据文档不能随任务删除而丢失。
