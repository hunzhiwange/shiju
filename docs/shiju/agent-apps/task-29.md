# 任务29：POSIX终端启动

2026-11-06，macOS。状态：已完成。

依赖核对：已读取只读plan.md、来源与接口.md、任务28证据及正式LaunchMaterial/validate_launch_request；RustCodeGraph核对来源launch_provider_terminal、build_provider_command_line、Shell参数和终端适配器。前序编号文件已有删除不属本次提交；不修改plan.md、upstream或来源快照。

## 实现与边界

新增执行主机业务入口launch_posix_terminal，消费服务端LaunchMaterial；不接收客户端命令串、主机ID或远端路径，不建立RPC。调用者必须在授权执行主机上立即重新运行任务28校验，再传入材料；本适配器补查本地文件、目录及可执行权限，不能代替授权或消除文件替换竞态。任务36负责认证入口接线。skip_permissions=true明确拒绝，等待供应商所有者材料化选项，不静默忽略。

macOS支持Terminal、iTerm2、Otty CLI、Alacritty、kitty、Ghostty、WezTerm、Kaku及Warp；AppleScript字符串转义反斜线与双引号，Shell字符串独立引用每个argv。Otty缺失明确诊断。已启动但失败的macOS终端回退Terminal；未知终端直接拒绝。Warp仅创建私有0700临时脚本，URL路径按字节编码，交接前关闭写句柄，启动失败自动删除，成功交接后脚本执行时自删除；GUI接受后未执行的脚本可能遗留于系统临时目录，不包含凭据。

Linux候选包括gnome-terminal、konsole、xfce4-terminal、mate-terminal、lxterminal、alacritty、kitty和ghostty，首选优先且去重；按终端区分--、-e及单字符串参数。spawn成功表示已交给终端进程，后台等待回收子进程，不阻塞到交互窗口关闭；不将此结果声明为应用运行成功。macOS检查启动器退出结果。所有候选缺失/失败给出明确诊断，不回显敏感载荷。

沿用来源Shell标志：zsh=-lic，bash/fish=-ic，sh/dash=-c；fish引用单独处理反斜线和单引号。cwd切换位于Shell启动配置加载之后，失败立即退出，避免登录配置覆盖cwd后运行错误目录。当前应用结束后终端由各终端配置决定，不额外打开第二个交互Shell。

无UI变更、无个人应用启动或安装、无HOME修改。macOS真实bash/zsh子进程覆盖argv与cwd，OS终端边界用命令/进程夹具替代；fish参数和Linux候选为显式环境规则测试，不宣称fish、Linux或所有第三方终端实机通过。逐应用终端实机验收仍归后续批次。

## Cargo及验证

原子mkdir依次领取实际slot-2；锁只由本任务释放，保留编译缓存。

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖.cargo/config.toml的build.build-dir=target；全部Cargo子进程继承两个绝对路径。

- 红灯：`cargo test -p vw-agent-apps --lib cc_apps_posix_terminal_launch -- --nocapture`，退出101；2个有效测试，1通过1失败。合法启动命令测试因最小签名返回未实现失败，不是依赖或工具错误。
- 迭代测试曾因macOS不存在/bin/true的夹具路径失败，改用/usr/bin/true；未据此声称行为通过。
- `cargo fmt --all`，退出0；审查未引入无关Rust格式化差异。
- 绿灯：`cargo test -p vw-agent-apps --lib cc_apps_posix_terminal_launch -- --nocapture`，退出0；7通过、0失败、128过滤。覆盖实际Shell输出的中文/空格/引号/字面量argv、cwd，未知终端、缺失Shell、权限跳过拒绝、fish参数、macOS转义与Warp失败清理、非零退出/缺失启动器及成功回退。
- `cargo clippy --all-targets --all-features -- -D warnings`，退出0，无警告。
- `git diff --check`，退出0；只提交领域模块、独立测试和本证据。

下一步：任务30 Windows终端适配；任务36通过授权执行主机入口接线，逐应用批次完成GUI实机验收。
