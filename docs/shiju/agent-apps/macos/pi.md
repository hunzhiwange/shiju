# Pi macOS 实机验收（已完成，待回归）

2026-11-06 14:20–14:27，Asia/Shanghai。读取只读plan.md、任务54、来源与接口、task-13/45及批次53证据。真实接口已存在，前序待回归未当作符号缺失；本次无Rust修改。未验收Windows/WSL/Linux。

## 环境与隔离

macOS 27.0 (26A428)、arm64、Node v26.13.0、npm 11.20.0。初始command -v pi为空。随机0700测试根内独立HOME/config、prefix、Applications，node/npm链接既有Homebrew工具。随机测试令牌与策略0600，Gateway只监听127.0.0.1:19454，p/a/host仅授权本机生命周期与启动。未读取或搬运个人凭据，未覆盖个人实例。

官方生产配方npm.rs::install_args(Pi)为npm i -g @earendil-works/pi-coding-agent@latest。registry返回官方repository=https://github.com/earendil-works/pi.git、bin=dist/bundle/cli.js、Node>=22.19.0；最新1.5.0、前版1.4.0。下载URL与integrity直接来自registry，见pi-registry.json。

## 场景与证据

请求/脱敏响应见pi-api.json，生产GatewayClient见pi-sdk.txt，临时SDK编译命令见pi-sdk-command.txt，执行日志见pi-runtime.txt和pi-cleanup.txt。

- 初始catalog 200、Pi/native_cli；installations=[]和launch-identities=[]均200，SDK基线退出0且一致。
- HOME=<isolated-home> npm install --global --prefix <isolated-prefix> @earendil-works/pi-coding-agent@1.4.0 --registry=https://registry.npmjs.org：首次依赖解析停滞，主动TERM（不算通过），继承临时缓存，以fetch-timeout=30000、fetch-retries=1重试退出0。实际pi --version退出0、1.4.0；API/SDK均1.4.0/runnable。
- 正式update操作7d299cac completed，真实--version输出1.5.0，SDK确认1.5.0/runnable，构成可观察升级变化。通用发现source=unknown，生命周期scoped_discover校验官方package.json后预览source=system；不混淆两种读取来源。
- 第二份官方1.4.0安装至隔离home/.local，退出0。双search_paths发现两实例；双路径写入预览422，缩回第一prefix后指定卸载完成，恢复双路径仅剩第二份1.4.0，SDK REMAINING一致。随后尝试直接读取第二份版本时清理已完成，FileNotFoundError，不算新增运行证据；此前API版本探测与SDK已证明其runnable。
- 正式install操作318fe191 completed，API/SDK确认1.5.0/runnable；指定卸载eb32dc1a completed。
- 立即取消a462b297及executing取消cdafef34均cancelled/terminal；重新请求status/events均200。执行期观察PID93910/PPID89601运行node <isolated-prefix>/bin/npm i -g @earendil-works/pi-coding-agent@latest；取消后kill(pid,0)返回ProcessLookupError，实例为空。不外推未观察孙进程。
- 仅在隔离prefix创建指向不存在路径的pi断链，diagnostics 200/probe_failed/damaged_installation，卸载预览422；删除断链。第二前缀使用官方npm uninstall --global --prefix <isolated-home>/.local @earendil-works/pi-coding-agent退出0。最终API/SDK均空。

## 待回归范围

默认features桌面构建及指定cargo run均实际执行，但现有其他任务改动system_settings_apps/mod.rs引用缺失list.rs、drag.rs，E0583并伴随rustc增量ICE，未获得可运行当前桌面。未用旧桌面截图冒充当前实现，故没有pi卡片、测试Gateway连接、进度取消、断线重读及错误toast截图；SDK/API一致不能替代SDK/Iced一致。

launch-identities=[]，无专用授权供应商或官方账号。默认终端、授权cwd、真实执行程序/环境、供应商与适用官方账号及启动临时材料清理未验证。未将缺账号标作不支持。双prefix写入422为空响应，不能称其有清晰UI限制。以上保留待回归，不宣称完整通过，不删除编号任务。

## Cargo与清理

两阶段均原子mkdir动态领取slot-2，owner记录task54/session与控制PID；阶段间释放并重新领取。CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖仓库build.build-dir=target。

| 确切命令 | 退出码 | 有效自动测试 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 101 | 0，E0583/ICE |
| cargo run -p vw-desktop --bin shi-ju | 101 | 0，同上 |

临时SDK rustc编译退出0，多阶段调用均退出0。本项人工场景，有效自动测试0；构建和零测试不算行为通过。无Rust修改，按任务无需Clippy。未运行全量test、fmt检查、wasm或Makefile。RustCodeGraph status/explore核对代码地图，未索引变更直接读取。

第一阶段构建退出[0,101]；最终Gateway/桌面退出[-15,101]，全部自有槽位进程结束后已释放锁、缓存保留。清理本次隔离安装/策略/令牌/临时目录，未修改plan.md。下一步修复桌面无关构建并准备专用启动身份后回归UI与终端场景。
