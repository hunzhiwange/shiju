# 任务189：修复desktop-task-board的board_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号365。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/desktop-task-board/src/board_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据365：`board::board_tests::kanban_falls_back_to_default_on_unknown_workflow`

位置：`crates/libs/desktop-task-board/src/board_tests.rs:119`；结果：FAIL。

```text
crates/libs/desktop-task-board/src/board_tests.rs:127:5:
    assertion `left == right` failed
      left: [Pool, Pending, Planning, Running, Failed, Paused, Completed]
     right: [Pool, Pending, Planning, Running, Failed, Paused, CodeComplete, CodeReview, PrSubmitted, Completed]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib 'board::board_tests::kanban_falls_back_to_default_on_unknown_workflow' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行记录

- 根因：`TaskStatus::all()` 是完整开发生命周期状态目录；默认工作流已为 `generic_task`，不含 CodeComplete、CodeReview、PrSubmitted。`kanban_columns` 经 `all_for_workflow_id` 回退默认工作流的实现正确。
- 修改：仅调整本文件断言，预期取 `default_task_workflow().statuses` 并保留有序完整比较、归档过滤和首列标签断言；复用生命周期缓存测试锁，与相邻自定义工作流测试隔离。未修改共享业务文件。
- 修复前命令：`python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib 'board::board_tests::kanban_falls_back_to_default_on_unknown_workflow' -- --exact --nocapture`。
- 修复前结果：退出 101，测试实际执行 0 项，未取得当前断言红灯；依赖 `desktop-settings-apps` 编译被既有错误阻断：`src/system_settings_apps/list.rs:185:63` E0425，形参为 `_local_available`，调用仍引用 `local_available`。该文件已有其他任务修改，不接管。
- 红灯尝试槽位 3；`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。日志 `/tmp/shiju-task189/red.log`，退出码文件 `/tmp/shiju-task189/red.exit`；槽位入口已正常退出并释放自有锁、保留缓存。
- 修复后运行同一精确命令：退出 101，实际执行测试 0 项，仍被同一 `desktop-settings-apps/list.rs:185:63` E0425 阻断；日志 `/tmp/shiju-task189/green.log`，退出码 `/tmp/shiju-task189/green.exit`。复用槽位 3，双缓存路径同上；两次入口均正常退出，自有子进程已收尾、锁已释放。未声称红灯复现或 passed=1。
- 代码状态：保留当前分支和检出；提交仅包含 `board_tests.rs` 及本编号任务记录。其他工作区与暂存区修改均排除。下一步：对应所有者修复依赖编译错误后，重跑上述精确命令，须实际执行 1 项并 passed=1、failed=0、退出 0，再完成并删除本编号任务文件。
- 轻量检查：`git diff --check -- crates/libs/desktop-task-board/src/board_tests.rs` 退出 0；已审查 diff，仅本任务测试预期和隔离锁变化。未执行全量测试、Cargo check 或桌面构建。
