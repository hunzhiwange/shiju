# Grok Build macOS 实机验收（已完成，待回归）

2026-11-06 13:49–13:58，Asia/Shanghai。核对只读plan.md、来源与接口、task-02/15/19/21/31及批次49证据。批次49的UI与身份待回归不妨碍本应用独立生命周期验收。本次没有Rust修改，未验证Windows/WSL/Linux。

## 环境、来源与隔离

macOS 27.0 (26A428)，arm64；Node v26.13.0、npm 11.20.0。基线command -v grok无输出；/opt/homebrew/bin/grok、/usr/local/bin/grok与个人~/.grok/bin/grok均不存在。未读个人凭据、未卸载个人实例。

官方来源https://x.ai/cli/install.sh，实际curl获取退出0，SHA256=bdd23859b10bd0c1492ffc3443e8413b73a9b09966d48d4d3fa32121043956fc。脚本声明指定版本、HOME/.grok与GROK_BIN_DIR；已检查其PATH软链行为。最初误查cli.x.ai域名TLS失败，随后使用源码中正确官方地址成功，没有绕过证书验证。官方npm登记https://registry.npmjs.org/@xai-official%2fgrok返回latest=1.0.77，旧版1.0.76真实存在，tarball=https://registry.npmjs.org/@xai-official/grok/-/grok-1.0.76.tgz，integrity=sha512-oqjFHYVVBf2HAdy5xDgOrP/aQifWa/aKrqIAoIDu5Q9G7f/tXNfRTiAFrcAt3AA/ned1hW8tvLVtj0eOG5U2Gg==。旧版postinstall从官方平台可选依赖安装二进制，已读取脚本核对，不猜下载URL。

0700随机隔离根，独立HOME/config/prefix/Applications与第二份安装HOME。0600策略只授权p/a/host本机与固定动作；随机令牌临时保存在0600文件，策略只保存摘要。Gateway绑定127.0.0.1:19450。写入进程清空环境，PATH限定测试prefix/bin、/usr/bin、/bin，npm隔离配置，不将个人配置/令牌传入。官方脚本不能经PATH改写个人/usr/local/bin入口。

## 实际场景与证据

HTTP完整脱敏请求与响应见grokbuild-api.json（63条），正式SDK见grokbuild-sdk.txt。安装ID为服务端原值，非凭据；测试根已替换占位符，未保存授权头。

- 官方npm安装旧版命令：HOME=<isolated-home> npm install --global --prefix <isolated-prefix> @xai-official/grok@1.0.76 --registry=https://registry.npmjs.org --ignore-scripts=false --include=optional --allow-scripts=@xai-official/grok，退出0。首次手工驱动将user/global配置同时设/dev/null导致npm退出1，改为独立空userconfig后成功，非产品缺陷。实际grok --version退出0，grok 1.0.76 (bc5f1dfb4a01)。目录API grokbuild/native_cli、实例1.0.76/runnable、launch-identities=[]均200。
- update预览200/source=system，提交202，6d93d668最终completed。实际grok --version退出0，grok 1.0.77 (50ca579fa996)，API与SDK一致，证明真实版本变化。通用发现source=unknown；生命周期scoped_discover按测试prefix中官方package.json验证为system，差异如实保留。
- 第二份官方1.0.76以独立second-home安装到隔离home/.local，npm退出0；双搜索路径发现1.0.77和1.0.76。多搜索路径卸载预览422空响应；源码apps/lifecycle/local.rs::validate限定单一prefix。缩回第一prefix后正式指定卸载05f60d7a completed；恢复双搜索路径只剩第二份1.0.76，SDK同样读取到剩余实例。没有声称多prefix写入受支持。
- 第一prefix空列表由SDK读取后，正式install 54ec1166 completed，发现及SDK均1.0.77/runnable；实际产物为官方npm平台二进制，非native下载目录。源码native_grok_mcode.rs先官方脚本后npm回退，本次证明允许的npm落地，未独立证明原生下载成功。
- 正式卸载后，两次安装取消分别覆盖立即取消与executing阶段，最终cancelled/terminal，独立status及events重读200，API无实例。executing采样捕获PID 58964、PPID 56014：node <isolated-root>/prefix/bin/npm i -g @xai-official/grok@latest --ignore-scripts=false --include=optional --allow-scripts=@xai-official/grok；取消后kill -0检查该PID不存在。仅证明所观察npm进程及无安装后验，不扩大到未观测下载孙进程。
- 测试prefix创建断链grok，diagnostics返回probe_failed/damaged_installation，指定卸载预览422空响应；随后删除断链，未破坏个人安装。
- 第二prefix官方npm uninstall退出0；最终API与SDK空列表。剩余测试HOME原生副本随测试根清理，未调用个人卸载。默认桌面和独立bundle、网关均终止，slot锁释放，缓存保留。

## 待回归与明确边界

标准默认features桌面cargo run已进入Running。相同产物复制为独立Task50.app（com.rsung.shiju.task50-acceptance），只为CUA定位并使用隔离HOME/config。CUA看到“识矩 - 项目”，点击右上设置、300ms点击及super+comma只显示“系统配置”tooltip，未进入设置；grokbuild-iced-entry.jpg为本次入口截图。一次bundle进程因非持久控制器结束失去绑定，改用持久控制器重试后仍复现入口限制。没有伪造本应用卡片、网关连接、进度、断线toast或重连截图；SDK/Iced一致性未完成，需可操作桌面回归。

正式launch-identities为空，没有专用获授权测试身份。Grok供应商投影在providers/mod.rs明确返回“grok provider session projection is not implemented”（task-31同记）；此源码限制不等于账号缺失不支持。真实Grok默认Terminal执行程序、cwd、供应商/官方账号与凭据材料清理均未验证，不借个人Terminal HOME、不虚构登录。上述外部身份与桌面条件使本任务不能完整通过，保留编号任务供回归。

## Cargo与驱动记录

按1–10原子mkdir动态领取slot-2，owner记录task50/会话/PID。CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖.cargo/config.toml的build.build-dir=target；未另建构建缓存。命令及退出码：

| 命令 | 退出码 | 有效自动测试数 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 0 | 0，构建 |
| cargo run -p vw-desktop --bin shi-ju | -15 | 0，Running后主动结束 |

本项为人工场景/API验收，有效自动测试0，不以编译或零测试代替行为证明。无Rust修改，按任务无需Clippy。已有ld __eh_frame linker warning，未修改无关链接选项。未运行全量test、fmt检查、wasm或Makefile。rustcodegraph status成功；explore长时间无输出被中止，后续对未取得图结果的领域文件使用rg/读取。

临时SDK驱动使用生产GatewayClient授权方法并对响应unwrap断言；首次错误Tokio rlib链接导致退出101/no reactor。按构建产物排查匹配运行时，五次不匹配候选运行仍101，不算成功。修正编译退出0：

```text
rustc --edition=2024 <isolated-root>/sdk.rs -L dependency=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps --extern vw_gateway_client=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libvw_gateway_client-601c8d932ba563e6.rlib --extern tokio=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libtokio-49c3d960b83bfe81.rlib -o /Users/xiangmin/worker/code/shiju/target/rust-slot-2/task50-sdk
```

修正后UPGRADED、REMAINING、EMPTY、API INSTALL、FINAL EMPTY共五次驱动均退出0并返回实际生产目录/实例；不是单元测试计数。CLI/desktop构建控制器退出记录[0,0,-15,-15]；独立bundle主动结束-15。所有本任务进程结束后释放自有slot-2，保留编译缓存；临时策略、令牌、包和测试根已清理。plan.md保持只读。
