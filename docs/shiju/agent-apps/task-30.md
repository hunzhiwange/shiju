# 任务30：Windows与WSL终端启动

2026-11-06，macOS。状态：已完成。

## 依赖与实现

读取只读plan.md、来源与接口.md、任务29证据、任务28 LaunchMaterial与校验边界；正式crate、WSL parse_target和前序终端模块均已交付。通过RustCodeGraph检查来源launch_windows_terminal、launch_terminal_running及调用关系。未修改upstream、来源或plan.md。

新增launch_windows_terminal执行主机适配器，支持cmd、PowerShell、Windows Terminal，首选无法spawn时回退cmd；未知终端明确拒绝。Windows进程使用CREATE_NEW_CONSOLE，后台等待回收子进程；成功只表示终端交接，不声明CLI运行成功。PowerShell使用-NoProfile和UTF-16LE EncodedCommand，路径与参数逐个单引号引用并转义；cmd只接收固定PowerShell命令和base64，不接收原始命令文本。无临时脚本和编码页依赖。批处理参数中的cmd元字符明确拒绝，避免隐式cmd解析注入。

WSL使用已有解析器再次校验发行版、shell与flag；命令始终含--distribution和--exec，不回退默认发行版。UNC路径必须属于相同发行版，拒绝跨发行版与路径穿越；POSIX路径保留中文与空格。在选定发行版内切换cwd并检查程序文件和执行权限；使用sh内层执行参数，fish外层独立转义。Auto明确使用该发行版内sh而非Windows主机shell。失败不尝试其他发行版。

调用方必须在授权执行主机即时观察与校验材料，包括WSL内部路径及授权根目录；本适配器不创建权限、RPC或客户端可提交命令的入口。任务28的本地验证器仍显式拒绝WSL，任务36负责通过真实主机上下文接线WSL校验，不以本适配器代替授权。skip_permissions材料在供应商选项材料化前拒绝。非Windows平台调用生产入口明确返回不支持。

## 验证

原子mkdir领取slot-2，占用记录含计划/任务/会话与持有进程；阶段结束释放自己的锁，保留缓存。

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

后者覆盖仓库build.build-dir，所有Cargo及其子进程继承以上绝对路径。

- 红灯：`cargo test -p vw-agent-apps --lib cc_apps_windows_terminal_launch -- --nocapture`，退出101；2个有效测试，1通过1失败，合法终端构建因最小签名返回未实现失败。
- 实现迭代：首次绿灯编译发现借用后移动错误，修正clone；首次Clippy发现collapsible_if及chunks_exact_to_as_chunks，均已修复。
- `cargo fmt --all`，退出0；审查仅本任务Rust变更，无无关格式化。
- 绿灯：`cargo test -p vw-agent-apps --lib cc_apps_windows_terminal_launch -- --nocapture`，退出0；5通过、0失败、135过滤。
- `cargo clippy --all-targets --all-features -- -D warnings`，退出0，无警告。
- `git diff --check`及暂存差异检查，退出0。

覆盖终端选择、UTF-16解码后的中文/空格/引号与cwd、bash/fish/sh参数、选定发行版绑定、跨发行版拒绝、未知终端、相对路径、批处理展开注入、权限跳过、非Windows入口及终端spawn失败/成功回退。进程夹具真实启动/bin/sh，输出并核对中文argv副作用。仅替代OS终端边界；不宣称Windows/WSL实机验收，无UI变更。既有编号任务删除均排除本次提交。

下一步：任务31供应商启动材料，任务36接线授权主机启动入口。
