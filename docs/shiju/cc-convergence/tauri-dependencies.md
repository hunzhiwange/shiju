# Tauri 运行依赖边界

2026-11-14，任务18。依赖任务17的静态IPC收口证据见同目录 `ipc.md`。本轮用户要求只实现与静态审查，不运行任何 Cargo、构建、单元测试或实机测试。本文不是构建图或产物验收通过证据。

## 当前清单与保留集合

- 根 `Cargo.toml` 注册正式 `crates/vw-agent-apps`、Iced桌面及WebView包，并在 workspace.exclude 中排除 `crates/vw-agent-apps/upstream/src-tauri`。该排除声明不是依赖边，来源归档和整体移除由任务21/22处理，不能在此提前移除隔离。
- 正式 `crates/vw-agent-apps/Cargo.toml` 没有 tauri、tauri-plugin-* 或 tauri-build；其原生模块使用 reqwest、数据库、系统安全存储及受控进程等既有依赖。正式模块树不注册来源Tauri命令，详见 `ipc.md`。
- 全仓正式 Cargo.toml（排除upstream与target）查询 tauri，唯一命中为根清单的上述exclude。正式build.rs查询 tauri/WebView无匹配；scripts和GitHub工作流查询tauri无匹配。来源清单与初始化代码保持原样，待统一归档。
- 当前根 `Cargo.lock` 中唯一名称含tauri的包为 `tauri-winrt-notification`，其反向依赖来自 `notify-rust 4.18.2`；正式桌面 `Cargo.toml` 在Windows目标声明notify-rust。它是系统通知适配依赖，不是来源 Tauri 应用运行栈。`app/desktop_notification.rs` 的Windows `show` 与应用注册仍有真实调用，不按名称误删。
- `crates/vw-webview/Cargo.toml` 的wry、WebKit、WebView2与Objective-C系统库支持正式WebView；桌面的AppKit库支持Iced系统动作，均保留。没有以清理Tauri为由移除仍使用的系统库。

因此本任务的最小运行依赖删除集合为空，不新增配置、不修改任何清单或锁文件，不手工删锁条目。检查开始时根锁文件及memory-lucid清单已有无关未提交修改，保持原状并排除本提交。

## 静态证据与后续验证

- RustCodeGraph status退出0；explore与node确认桌面通知入口。索引行号与当前源码可能有偏移；清单、锁文件以当前文件查询为准，不把图推断当解析后的Cargo依赖图。
- `rg -n -i tauri Cargo.toml Cargo.lock crates/vw-agent-apps/Cargo.toml` 退出0，命中exclude、notify-rust依赖与通知包条目；正式清单查询仅命中exclude；正式build.rs、scripts与工作流查询无匹配（退出1）。一次脚本通配路径未匹配未形成证据；一次Python TOML扫描因当前Python无tomllib失败，未据此声称清单解析通过，未安装依赖。
- 未运行metadata、tree、锁文件解析更新、macOS链接或任何行为测试。正式产物依赖图仍须后续统一验证；本轮仅证实没有可在静态层安全删除的Tauri运行清单边。
- 基线 `fbd619001`；本任务只新增本文。恢复撤回本文即可，来源快照和现有锁文件未改变。未来统一验证重点是正式产物依赖图、通知和WebView链接；不能将Windows通知适配包的名称命中判为Tauri运行栈残留。
