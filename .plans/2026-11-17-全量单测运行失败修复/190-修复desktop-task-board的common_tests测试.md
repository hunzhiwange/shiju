# 任务190：修复desktop-task-board的common_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号366, 367, 368。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/desktop-task-board/src/common_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据366：`common::common_tests::danger_button_style_distinguishes_pressed_state_and_uses_board_radius`

位置：`crates/libs/desktop-task-board/src/common_tests.rs:106`；结果：FAIL。

```text
crates/libs/desktop-task-board/src/common_tests.rs:14:5:
    expected 0.76862746 to be close to 1
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据367：`common::common_tests::primary_button_style_preserves_status_background_and_adds_board_elevation`

位置：`crates/libs/desktop-task-board/src/common_tests.rs:59`；结果：FAIL。

```text
crates/libs/desktop-task-board/src/common_tests.rs:14:5:
    expected 0.9798983 to be close to 1
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据368：`common::common_tests::secondary_button_style_keeps_shared_status_colors_and_uses_board_radius`

位置：`crates/libs/desktop-task-board/src/common_tests.rs:82`；结果：FAIL。

```text
crates/libs/desktop-task-board/src/common_tests.rs:14:5:
    expected 0.8698165 to be close to 1
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib 'common::common_tests::danger_button_style_distinguishes_pressed_state_and_uses_board_radius' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib 'common::common_tests::primary_button_style_preserves_status_background_and_adds_board_elevation' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib 'common::common_tests::secondary_button_style_keeps_shared_status_colors_and_uses_board_radius' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行记录

- 根因：看板直接复用公共按钮样式；公共样式已采用可读性配色及独立禁用态，失败测试仍断言旧透明度/白字/红色。仅修改本任务 common_tests.rs，不回退公共样式。
- 主按钮对照公共样式保留背景与文字继承断言，保留看板圆角和阴影；主按钮、危险按钮覆盖明暗主题，危险按钮逐状态精确配色，次按钮更新浅色状态期望。
- 修改前按原生精确过滤顺序执行三项，均因非本任务工作区变更 desktop-settings-apps/src/system_settings_apps/list.rs:185 的 E0425（参数改为 _local_available，但调用仍用 local_available）未能进入测试。日志 /tmp/shiju-190-red.log，退出101。
- 修改后同样逐项精确过滤验证，日志 /tmp/shiju-190-green.log；仍受同一依赖编译错误阻塞，未取得测试通过证据。保留任务待回归，需依赖所有者修复后重跑三项命令。
- 修改前槽位4，修改后槽位6；双缓存分别均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-4 和 /Users/xiangmin/worker/code/shiju/target/rust-slot-6（CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 同值）。未清理他人缓存与进程。
- rustfmt 仅作用于 common_tests.rs；git diff --check 通过；审查仅包含按钮测试预期与主题覆盖，未修改业务实现或公共计划。
