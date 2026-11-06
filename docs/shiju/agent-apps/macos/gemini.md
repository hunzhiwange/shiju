# Gemini macOS 实机验收（已完成，待回归）

2026-11-06，Asia/Shanghai。读取只读plan.md、来源与接口、task-13/34/37/44及批次48实机与恢复证据。批次48待回归的真实身份不等于生产接口缺失；本次仅Gemini，无Rust修改，无其他平台实机声明。

## 环境和来源

macOS 27.0 (26A428)，arm64；Node v26.13.0、npm 11.20.0。原有入口/opt/homebrew/bin/gemini，真实文件/opt/homebrew/lib/node_modules/@google/gemini-cli/bundle/gemini.js；隔离HOME下仅执行--version得到0.66.0。个人入口前后SHA256均f48ad00f929e699361d442440e89a70ab08342f9042ea22f99f0e9ff4bf60ba6；没有读取个人配置、密钥或钥匙串。

官方渠道：https://geminicli.com/docs/get-started/installation/ 声明npm install -g @google/gemini-cli。实际GET https://registry.npmjs.org/@google%2fgemini-cli 得latest=0.67.0，稳定前版0.66.0来自versions字段。旧版tarball https://registry.npmjs.org/@google/gemini-cli/-/gemini-cli-0.66.0.tgz，integrity=sha512-EAtsJ9/JxiphHlduzvhoCkb7I0LdOxyugB5NpUtVqKVGZIWjV3JmurvK89ZrvMbV3kwk5SdWINz1M5ZN/2K/Lw==。未猜下载URL。

0700随机隔离根内独立home/config/prefix/Applications；prefix/bin/node、npm链接既有Homebrew工具。0600策略限定本机host、租户p/a、固定应用动作，随机令牌仅临时保存，授权文件只含令牌摘要。Gateway仅127.0.0.1:19449。安装仅测试prefix及home/.local，不覆盖个人实例。

## 实际结果

脱敏HTTP请求/响应见gemini-api.json，正式SDK输出见gemini-sdk.txt。API安装ID采用服务端原值，不把编码ID误当凭据；不保存授权头。

- 初始catalog Gemini/native_cli、installations=[]、launch-identities=[]，均200。
- HOME=<isolated-home> npm install --global --prefix <isolated-prefix> @google/gemini-cli@0.66.0 --registry=https://registry.npmjs.org 退出0；隔离入口--version退出0，0.66.0。
- 正式update预览200/source=system、提交202，5382e50b最终completed。实际--version与发现均0.67.0，证明可观察升级。通用发现source仍unknown，与管理员prefix中package.json复核的生命周期system来源不同，保留差异。
- 正式卸载4acb70e1 completed，再正式install 95004311 completed，实际发现0.67.0/runnable。status及events分别重读200，没有将queued当成功。
- 第二份官方0.66.0安装到隔离home/.local，npm退出0。双路径发现两实例；多search_paths卸载预览422，源码apps/lifecycle/local.rs::validate要求单prefix搜索路径，与task-34契约一致。临时缩回第一prefix后正式指定卸载056f2c38 completed，再恢复双路径发现只剩第二份0.66.0。SDK驱动首次未成功读取，见下节。
- 正式install fa898b98立即DELETE，cancelled/terminal，events重读一致，发现为空。另1a83977a观察running/executing后DELETE也cancelled，events一致、发现为空；操作系统采样未捕获本安装npm子进程，因此仍不能声称证明运行中下载进程树取消。
- 仅在本测试prefix创建断链gemini；diagnostics 200，probe_failed/damaged_installation，卸载预览422。随后清除断链，不破坏个人安装。
- 第二prefix npm uninstall --global --prefix <isolated-home>/.local @google/gemini-cli退出0，最终API为空列表；首次SDK驱动失败。
- SDK首次临时驱动错误链接了另一份Tokio依赖，三次均退出101/no reactor；不是产品API失败，不计成功。后续复验结果见追加记录。

## 待回归及边界

标准cargo run窗口已启动；同次产物复制为独立Task49.app以便CUA定位，bundle=com.rsung.shiju.task49-acceptance，独立HOME/config。CUA可读“识矩 - 项目”窗口和截图，但设置按钮反复点击仅显示“系统配置”tooltip，延长点击与super+comma均未打开设置；AX仅窗口和系统菜单。未绕过UI注入网关配置，没有取得Gemini卡片截图、离线toast/重连或SDK/Iced对应记录。此前批次截图不作为本应用证据。此为本次桌面交互环境/入口未解决的验收限制，未修改无关UI模块。

独立网关正式launch-identities为空，没有获授权的专用供应商/官方测试身份。生产供应商/账号材料化与托管PTY已经交付；不将缺身份称为不支持，不用虚构令牌证明真实登录，不沿用个人Terminal的HOME。默认终端实际Gemini程序/cwd/环境、供应商/官方账号启动、真实材料清理未验证。运行中下载取消、断线重读及toast仍待回归；本任务没有完整通过，保留编号任务文件。

## Cargo与补充回归

原子mkdir按1–10动态领取slot-2，owner记录任务49、会话及控制PID。CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖.cargo/config.toml中build.build-dir=target。未另建Cargo缓存目录。实际命令：

| 命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 非行为测试 |
| cargo build -p vw-desktop --bin shi-ju | 0 | 非行为测试 |
| cargo run -p vw-desktop --bin shi-ju | -15 | 默认features，进入Running后主动结束 |
| cargo test -p vw-agent --lib cc_apps_launch_identity_list -- --nocapture | 0 | 3通过，11467过滤 |
| cargo test -p vw-agent --lib cc_apps_launch_native_terminal -- --nocapture | 0 | 1通过，11469过滤 |

共4项有效自动回归，真实Terminal用受控CLI/身份夹具验证SDK/HTTP/PTY与材料生命周期，不冒充官方Gemini真实登录。无Rust变更，按任务无需Clippy。已有ld __eh_frame提示仍存在，未改链接选项。没有全量test、fmt检查、wasm或Makefile。

SDK临时驱动确切编译命令（<isolated-root>为本次随机隔离根）：

```text
rustc --edition=2024 <isolated-root>/sdk.rs -L dependency=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps --extern vw_gateway_client=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libvw_gateway_client-1a94bf756d53cb1d.rlib --extern tokio=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libtokio-0baca5301493f837.rlib -o /Users/xiangmin/worker/code/shiju/target/rust-slot-2/task49-sdk
```

rustc退出0；首次三次驱动均退出101，未计成功。标准桌面、测试bundle与gateway均已终止，控制器记录构建0/0、gateway -15、desktop -15；清理测试根及临时令牌/策略/包，个人入口哈希一致，自有slot-2锁释放、缓存保留。其他进程/锁未删除。plan.md保持只读。

## SDK修正复验

第二阶段重新原子领取slot-2，同一绝对CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR。首次SDK链接错误已修正为与SDK一致的Tokio rlib，不修改生产源码。首轮修正驱动在网关尚未监听时返回Transport，退出0仅是驱动未断言，并非读取成功，保留原输出。随后加入TCP就绪探测，真实127.0.0.1:19459网关SDK三次分别读取空列表、0.67.0/runnable实际安装、卸载后空列表，全部返回CAT Ok与INSTALLATIONS Ok，退出0。安装7f754f28、卸载a819c2cc均正式API completed，独立API与SDK路径/版本一致。此复验不替代缺失Iced证据。

第二阶段cargo build -p vw-cli --bin shiju与cargo build -p vw-desktop --bin shi-ju均0，默认cargo run进入Running后主动结束-15。新增19459网关主动结束-15，不计自动测试。SDK修正编译命令退出0：

```text
rustc --edition=2024 <isolated-root>/sdk.rs -L dependency=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps --extern vw_gateway_client=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libvw_gateway_client-1a94bf756d53cb1d.rlib --extern tokio=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libtokio-f993a923b732abac.rlib -o /Users/xiangmin/worker/code/shiju/target/rust-slot-2/task49-sdk-fixed
```

复验后官方包通过API卸载，临时环境、网关和桌面已清理，自有slot-2锁释放并保留缓存。最终仍为已完成，待回归，任务文件保留。
