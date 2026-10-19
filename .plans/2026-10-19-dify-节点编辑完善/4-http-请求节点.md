# 任务 4: HTTP 请求节点

批次：【批次 4】 依赖批次 3

状态：已完成，待回归

回归记录（2026-10-20）：

- `cargo test -p vw-desktop http_request_visual_edit_round_trips_without_secret_leak --lib`：1 passed，退出码 0。
- `cargo test -p vw-desktop http_request_validation_rejects_invalid_url_timeout_and_empty_auth_without_secret --lib`：1 passed，退出码 0。
- `cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`：退出码 0。
- `cargo clippy --all-targets --all-features -- -D warnings`：被无关 Shiju 改动阻断；`ShijuSkillSearchQuery` 测试仍引用已不存在的 `tag_id`/`company_tag_id` 字段。
- 验证槽位：1；`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`。宽泛 `http_request_` 过滤器中本任务 2 项均通过，另有 2 项无关系统设置 allowlist 测试失败。

目的：实现 HTTP 方法、URL、鉴权、请求头、参数、请求体、超时、TLS、重试与错误策略的安全可视化编辑。

来源任务：无

预计会话范围：只处理 http-request 的数据模型与面板；不发送请求、不探测 URL、不管理真实凭据。

## 文件

- 修改：`crates/vw-desktop/src/apps/workflow/model/defaults.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/{mod.rs,draft_build.rs,visual_sync.rs,node_validation.rs}`
- 修改：`crates/vw-desktop/src/apps/workflow/message/{mod.rs,node.rs}`
- 修改：`crates/vw-desktop/src/apps/workflow/view/node_visual.rs`
- 新建：`crates/vw-desktop/src/apps/workflow/view/http_request.rs`
- 测试：对应模块独立 `*_tests.rs`

## 上下文

- Dify DSL 常见字段含 method、url、authorization、headers、params、body、timeout、ssl_verify、retry_config/error_strategy。
- 密钥字段不得写入日志或 toast；空 Basic/Bearer 凭据必须明确校验，不伪装成可用配置。

## Cargo 共享槽位规则（仅 Rust/Cargo 任务）

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

- 行为：各 body/auth 形态无损往返，非法 URL/超时/空鉴权被定位且错误不泄漏密钥。
- 失败验证测试：`http_request_visual_edit_round_trips_without_secret_leak`。
- 失败验证命令：`cargo test -p vw-desktop http_request_visual_edit_round_trips_without_secret_leak --lib`
- 预期失败原因：HTTP 节点目前仅能编辑原始 DSL。
- 通过验证命令：`cargo test -p vw-desktop http_request_ --lib`
- 模拟策略：不发网络请求，只验证 YAML、状态和渲染树。

## 步骤

1. 为 auth/body/timeout/retry 与脱敏错误写失败测试。
2. 实现强类型草稿和保留未知字段的合并。
3. 实现分段面板，密钥输入不回显到反馈。
4. 加入格式与范围校验，不验证远程可达性。
5. 格式化、聚焦测试和 clippy。

## 验证

- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：HTTP 测试实际执行通过，输出不含 fixture 密钥，clippy 退出码 0。
- 所需证据：红绿测试、脱敏断言、测试数、槽位与绝对目标目录。

## 完成

HTTP 节点可安全编辑 Dify 常用配置，未知字段保留且不执行外部请求；完成后按计划提交规则处理。
