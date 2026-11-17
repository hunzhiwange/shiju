# 任务177：修复desktop-settings-apps的catalog_ui_refresh_06_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号345, 346。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据345：`host_module_25_tests::catalog_ui_refresh_06_tests::ui_refresh_06_layout_and_interaction`

位置：`crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs:299`；结果：FAIL。

```text
crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs:176:62:
    missing layout path [0, 2, 1, 0, 0, 0, 0, 2, 0]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据346：`host_module_25_tests::catalog_ui_refresh_06_tests::ui_refresh_06_preserves_values_and_disabled_states`

位置：`crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs:420`；结果：FAIL。

```text
crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs:176:62:
    missing layout path [0, 2, 1, 0, 0, 0, 0, 3, 1]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-settings-apps --lib 'host_module_25_tests::catalog_ui_refresh_06_tests::ui_refresh_06_layout_and_interaction' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p desktop-settings-apps --lib 'host_module_25_tests::catalog_ui_refresh_06_tests::ui_refresh_06_preserves_values_and_disabled_states' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本轮执行证据（2026-11-17）

- 仅修改归属测试文件；未接管共享业务源码。旧版卡片内操作已迁至独立详情面板，更新摘要/搜索路径、文字定位、选择禁用及真实悬停进入/离开断言，扩大交互测试高度以容纳独立详情；保留启动/升级/安装请求目标和中文输入断言。
- 执行期间其他任务移除了 connection 模块、列表连接入口及 page_fixture 对应 overlay 分支。按最新实现移除旧弹窗断言，改为验证遗留 connection_open 不暴露凭据表单，覆盖明暗主题、窄屏及中文搜索。没有恢复已删除入口。
- 红灯：两条任务原生精确命令各执行 1 项、0 passed / 1 failed，退出 101，均复现 missing layout path；日志 `/tmp/shiju-task177/red-layout.log`、`red-values.log`、`red-slot.log`。
- 红灯槽位 3，双缓存均 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；后续槽位 9，双缓存均 `/Users/xiangmin/worker/code/shiju/target/rust-slot-9`。后续为复用缓存使用 `scripts.cargo_slot.run([...], number=9)`，内部 cargo 参数与任务精确命令相同；同阶段两条命令复用一次入口。
- 中间运行验证了启动/选择/拖动/升级/查询交互；安装按钮超出旧视口，已增高；窄屏搜索同样增高后尚待有效复验，不据此报告通过。
- 最终两条精确命令均编译退出 101、实际测试数 0：共享 `system_settings_apps/list.rs:185` 引用 `local_available`，参数在另一任务中改为 `_local_available`，触发 E0425。日志 `/tmp/shiju-task177/pass-layout.log`、`pass-values.log`、`pass-slot.log`。此业务文件正在共享修改，未覆盖其他任务内容。
- 修改文件 rustfmt 与 git diff --check 通过；未运行全量测试。槽位入口已退出并释放自身锁，未清缓存。
- 下一步：共享业务文件所有者修复编译错误后，重新执行本卡两条精确命令，必须分别 ran=1、passed=1、failed=0、退出0；通过后删除本卡。当前不能标记完成。
