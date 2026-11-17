# 任务144：修复desktop-apps-ui的host_module_35_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号293, 294, 295, 296, 297, 298, 299, 300。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据293：`host_module_35_tests::binding_failures_preserve_selection_and_close_detail_cancels_stale_work`

位置：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:396`；结果：FAIL。

```text
crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:414:5:
    assertion failed: matches!(&app.shiju_apps.launch, ShijuAppLaunchState::SelectingApiKey
        { selected_api_key_id: Some(42), error: Some(error), .. } if error ==
        "gateway unavailable")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据294：`host_module_35_tests::binding_save_rejects_missing_or_disabled_selection`

位置：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:297`；结果：FAIL。

```text
crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:345:5:
    assertion failed: app.active_toast.as_ref().is_some_and(|toast|
            toast.message.contains("状态已失效"))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据295：`host_module_35_tests::binding_save_success_continues_launch_and_ignores_stale_results`

位置：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:349`；结果：FAIL。

```text
crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:362:5:
    assertion failed: matches!(app.shiju_apps.launch, ShijuAppLaunchState::SavingApiKeyBinding
        { selected_api_key_id: 42, .. })
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据296：`host_module_35_tests::configured_binding_continues_existing_launch`

位置：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:165`；结果：FAIL。

```text
crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:177:5:
    assertion failed: matches!(app.shiju_apps.launch, ShijuAppLaunchState::Preparing
        { generation: active_generation, ref installation_id, } if
        active_generation == generation && installation_id ==
        "01K1APP0000000000000000001")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据297：`host_module_35_tests::open_shiju_app_checks_binding_before_launch`

位置：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:142`；结果：FAIL。

```text
crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:148:5:
    assertion failed: matches!(app.shiju_apps.launch, ShijuAppLaunchState::CheckingBinding
        { generation: active_generation, ref installation_id, } if
        active_generation == generation && installation_id ==
        "01K1APP0000000000000000001")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据298：`host_module_35_tests::shiju_apps_tabs_ignore_stale_or_wrong_account_results`

位置：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:53`；结果：FAIL。

```text
crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:76:5:
    assertion failed: matches!(app.shiju_apps.mine, ShijuAppsLoadState::Loaded(_))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据299：`host_module_35_tests::unconfigured_binding_rejects_read_only_or_missing_enabled_keys`

位置：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:230`；结果：FAIL。

```text
crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:244:5:
    assertion failed: matches!(&app.shiju_apps.launch, ShijuAppLaunchState::SelectingApiKey
        { can_edit: false, selected_api_key_id: None, error: Some(error), .. } if
        error.contains("不可修改"))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据300：`host_module_35_tests::unconfigured_binding_selects_first_enabled_key`

位置：`crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:188`；结果：FAIL。

```text
crates/libs/desktop-apps-ui/src/host_module_35_tests.rs:198:5:
    assertion failed: matches!(app.shiju_apps.launch, ShijuAppLaunchState::LoadingApiKeys
        { generation: active_generation, .. } if active_generation == generation)
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib 'host_module_35_tests::binding_failures_preserve_selection_and_close_detail_cancels_stale_work' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib 'host_module_35_tests::binding_save_rejects_missing_or_disabled_selection' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib 'host_module_35_tests::binding_save_success_continues_launch_and_ignores_stale_results' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib 'host_module_35_tests::configured_binding_continues_existing_launch' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib 'host_module_35_tests::open_shiju_app_checks_binding_before_launch' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib 'host_module_35_tests::shiju_apps_tabs_ignore_stale_or_wrong_account_results' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib 'host_module_35_tests::unconfigured_binding_rejects_read_only_or_missing_enabled_keys' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib 'host_module_35_tests::unconfigured_binding_selects_first_enabled_key' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
