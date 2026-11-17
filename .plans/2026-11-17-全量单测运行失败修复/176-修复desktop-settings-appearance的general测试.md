# 任务176：修复desktop-settings-appearance的general测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号343, 344。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据343：`host_module_26_tests::system_settings_general_tests::tab_appearance::help_rollout_03_themes_render_contextual_help`

位置：`crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs:707`；结果：FAIL。

```text
crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs:220:9:

    running 1 test
    test host_module_26_tests::system_settings_general_tests::tab_appearance::help_rollout_03_themes_render_contextual_help ... FAILED

    failures:

crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs:603:9:
    theme-basics-zh-dark: hovering beside "名称" must draw the help card
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


    failures:
        host_module_26_tests::system_settings_general_tests::tab_appearance::help_rollout_03_themes_render_contextual_help
```

### 证据344：`host_module_26_tests::system_settings_general_tests::tab_appearance::settings_tabs_general_appearance`

位置：`crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs:799`；结果：FAIL。

```text
crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs:220:9:

    running 1 test
    test host_module_26_tests::system_settings_general_tests::tab_appearance::settings_tabs_general_appearance ... FAILED

    failures:

crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs:878:21:
    "我的智能伙伴" missing from Companion
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


    failures:
        host_module_26_tests::system_settings_general_tests::tab_appearance::settings_tabs_general_appearance
```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-settings-appearance --lib 'host_module_26_tests::system_settings_general_tests::tab_appearance::help_rollout_03_themes_render_contextual_help' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-settings-appearance --lib 'host_module_26_tests::system_settings_general_tests::tab_appearance::settings_tabs_general_appearance' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
