# OpenClaw macOS 实机验收（已完成，待回归）

2026-11-06 14:05–14:13，Asia/Shanghai。已读只读plan.md、任务52、来源与接口、task-13及批次51证据。前序待回归不等于生产接口缺失；本次无Rust变更，未做Windows/WSL/Linux实机验收。

## 环境与来源

macOS 27.0 (26A428)、arm64、Node v26.13.0、npm 11.20.0。初始command -v openclaw无输出，结束时Homebrew、/usr/local、个人~/.local/bin与~/.openclaw/bin均无入口。未读取个人凭据或删除个人实例。

0700随机隔离根内独立HOME/config/prefix/Applications；prefix/bin/node与npm链接既有Homebrew工具。0600策略限定p/a/host本机和固定生命周期动作，随机令牌临时保存，策略仅含摘要。Gateway仅127.0.0.1:19452。临时npm配置不使用个人配置；安装仅隔离prefix与home/.local。

官方安装页 https://docs.openclaw.ai/install 声明npm渠道，当前文档对npm 11.16+推荐--allow-scripts=openclaw；生产npm.rs当前OpenClaw配方仅i -g openclaw@latest。本机npm 11.20只警告仍执行脚本，不外推npm 12兼容性。真实registry https://registry.npmjs.org/openclaw 得latest=2026.11.1、稳定前版2026.10.10；旧包URL与integrity直接取versions字段：

- https://registry.npmjs.org/openclaw/-/openclaw-2026.10.10.tgz
- sha512-9HZhzq/Yck3uH3M/iYP7szqoF5JXpXfz0+orhBOKalSH+dj0IC9uWPECkR1vH4lMCytxpHjJKHh9efXRYMJl6w==

## 真实场景结果

API请求/脱敏响应见openclaw-api.json，生产GatewayClient输出见openclaw-sdk.txt，截图见openclaw-iced-entry.png。实例ID为服务端原值，非凭据；未保存授权头。

- 初始catalog OpenClaw/native_cli、installations=[]、launch-identities=[]，均200。
- HOME=<isolated-home> npm install --global --prefix <isolated-prefix> openclaw@2026.10.10 --registry=https://registry.npmjs.org退出0；真实入口--version退出0，OpenClaw 2026.10.10 (7dc3350)。
- 正式update ec33324d最终completed，真实--version退出0，2026.11.1 (4ffdd20)；API与SDK均2026.11.1/runnable，构成版本变化证据。通用发现source=unknown，生命周期预览scoped_discover校验官方package.json后source=system，保留差异。SDK首次BEFORE UPGRADE FINISHED确实仍为旧版，后续UPGRADE COMPLETED才为新版，不把提前读取当通过。
- 第二份官方2026.10.10安装到隔离home/.local，npm退出0。双search_paths发现两实例；指定卸载预览422空响应，local.rs::validate要求单prefix。缩回第一prefix后指定卸载b37b974d completed；恢复双路径只剩第二份2026.10.10，SDK REMAINING一致。此证明剩余实例保持，不能称多prefix写入已支持。
- 正式install 81734cfb completed，API/SDK为2026.11.1/runnable；指定卸载2961baeb completed。立即取消850c55f2与executing取消7d18f250均cancelled/terminal，status/events独立重读200，实例为空。取消前观察PID73022/PPID66541：node <isolated-root>/prefix/bin/npm i -g openclaw@latest；取消后kill(pid,0)为ProcessLookupError，证明该进程消失，不外推未观察孙进程。
- 测试prefix中创建断链openclaw；diagnostics 200/probe_failed/damaged_installation，卸载预览422。之后删除断链，未损坏个人安装。第二prefix npm uninstall --global --prefix <isolated-home>/.local openclaw退出0；最终API与SDK均空列表。
- SDK共6次执行退出0，含提前读取、升级后、剩余副本、第一prefix空、API安装、最终空；采用unwrap断言。这是实机场景驱动，有效自动测试计0，不把构建或0测试当行为通过。

## 未验证与明确限制

标准默认features cargo run进入Running。隔离Task52.app（com.rsung.shiju.task52-acceptance）由同一产物复制，CUA实际窗口“识矩 - 项目”、AX仅窗口和菜单；设置图标长点击、按截图缩放位置点击及super+comma均未进入设置，已保存本次入口截图。未获得OpenClaw卡片、现有连接入口、进度/取消、断线toast/重连和SDK/Iced一致性证据。未绕过UI注入配置，未修改其他任务正在改动的UI。

providers/mod.rs对OpenClaw明确返回openclaw provider session projection is not implemented，但未取得该项实际API/UI错误，不能将其记为完整限制验收通过。launch-identities为空，缺专用授权供应商/官方身份；不使用虚构凭据或个人Terminal的HOME。真实默认终端程序/cwd/环境、供应商/适用官方账号、材料清理仍未验证。保留任务文件供回归，不能宣称全项通过。

## Cargo与清理

原子mkdir动态领取slot-2，owner记录task52/会话/控制PID66132。CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖.cargo/config.toml的build.build-dir=target。

| 确切命令 | 退出码 | 有效自动测试 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 0 | 0，构建 |
| cargo run -p vw-desktop --bin shi-ju | -15，Running后主动结束 | 0，人工场景 |

临时SDK编译命令退出0：

```text
rustc --edition=2024 <isolated-root>/sdk.rs -L dependency=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps --extern vw_gateway_client=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libvw_gateway_client-601c8d932ba563e6.rlib --extern tokio=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libtokio-49c3d960b83bfe81.rlib -o /Users/xiangmin/worker/code/shiju/target/rust-slot-2/task52-sdk
```

无Rust变更，按任务无需Clippy；已有ld __eh_frame提示未改。未执行全量test、fmt检查、wasm或Makefile。rustcodegraph status成功，explore无输出等待后终止，按图未覆盖路径直接核对领域源码。

测试Gateway与标准桌面退出-15，控制器退出记录[0,0,-15,-15]；独立bundle进程结束。全部本任务槽位产物进程结束后释放自有slot-2锁、保留缓存。测试根、临时npm包/策略/令牌已清理，个人常见入口仍不存在。plan.md保持只读。
