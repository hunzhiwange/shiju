# 任务186：修复desktop-settings-shell的help_ui_refresh_03_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号360。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/desktop-settings-shell/src/host_module_30_tests/help_ui_refresh_03_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据360：`host_module_30_tests::help_ui_refresh_03_tests::ui_refresh_03_renders_model_draft_and_error_with_original_actions`

位置：`crates/libs/desktop-settings-shell/src/host_module_30_tests/help_ui_refresh_03_tests.rs:571`；结果：FAIL。

```text
crates/libs/desktop-settings-shell/src/host_module_30_tests/help_ui_refresh_03_tests.rs:596:57:
    called `Option::unwrap()` on a `None` value
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-settings-shell --lib 'host_module_30_tests::help_ui_refresh_03_tests::ui_refresh_03_renders_model_draft_and_error_with_original_actions' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据

- 根因：`modal_card` 已返回卡片，旧测试额外向下一层导致把标题中的错误提示当成 footer；当前 footer 是取消、保存两个按钮，无旧占位节点。
- 最小修改：移除多余层级，显式断言两个操作，按顺序定位取消和保存；保留 tiny-skia 真实绘制、繁体中文、暗色、遮罩与原消息点击断言。业务实现无需修改。
- 红灯：原精确命令退出101，执行1项，0 passed / 1 failed，596行 unwrap；日志 `/tmp/shiju-task186-red.log`。槽位6，双缓存路径均 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。
- 复验：同一精确命令退出101，尚未执行测试；日志 `/tmp/shiju-task186-green.log`。槽位4，双缓存路径均 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。
- 外部阻断：`crates/libs/desktop-settings-apps/src/system_settings_apps/list.rs:185` E0425：形参为 `_local_available`，调用使用 `local_available`；该文件存在其他任务未提交改动，等待后重读仍未修复，本任务不覆盖其所有权。
- 两次入口均已退出并释放自己的槽位；保留缓存。仅目标测试文件变更4增4删，`git diff --check` 通过；未做全量测试、桌面构建。
- 下一步：共享依赖修复后运行上述原生精确命令，确认1 passed，再删除本编号文件。
