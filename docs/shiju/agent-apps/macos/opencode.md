# OpenCode macOS 实机验收（已完成，待回归）

2026-11-06，Asia/Shanghai。已核对只读plan.md、来源与接口、task-02/14/19/21/31/32与批次50证据。前批身份/UI待回归不等于符号缺失，本次独立验证OpenCode生命周期。无Rust修改，未做Windows/WSL/Linux实机验收。

## 环境与隔离

macOS 27.0 (26A428)，arm64；Node v26.13.0、npm 11.20.0。初始command -v opencode无输出；/opt/homebrew/bin/opencode、/usr/local/bin/opencode、个人~/.opencode/bin/opencode与~/.local/bin/opencode结束前均不存在。未读个人配置或凭据，未卸载个人实例。

0700随机隔离根、独立HOME/config/prefix/Applications；prefix/bin仅链接既有node/npm。0600策略限定p/a/host本机固定动作；随机令牌仅临时保留，策略只存摘要。Gateway绑定127.0.0.1:19451。安装进程使用限定PATH与独立npm配置。仅测试prefix与home/.local写入。

官方npm登记https://registry.npmjs.org/opencode-ai实读latest=1.19.5、previous=1.19.4，旧版tarball=https://registry.npmjs.org/opencode-ai/-/opencode-ai-1.19.4.tgz，integrity=sha512-O3rTc9OCd6CeeJohCAjFYD3MX5U0weKzKnySfsJ9sw9uAbNZZK4meIcjd+7k3inmZB5G2qONSen3yA45yECiFA==。官方https://opencode.ai/install获取退出0，SHA256=a2dd59522b9a091b6cdb2e4870c0f2135f7da09807cd5a7e04222d4316925b72；脚本明确支持--version、--no-modify-path，固定HOME/.opencode/bin且拒绝2.x。版本与URL均来自真实登记/脚本，未猜下载地址。

## 实际结果

脱敏API请求/响应见opencode-api.json；生产GatewayClient输出见opencode-sdk.txt。实例ID保留服务端原值，测试根替换为占位符，不保存授权头。

- baseline目录OpenCode/native_cli、实例空列表、launch-identities=[]，均200。
- HOME=<isolated-home> npm install --global --prefix <isolated-prefix> opencode-ai@1.19.4 --registry=https://registry.npmjs.org退出0，真实入口--version退出0输出1.19.4。
- update预览200/source=system、提交202，1b8bb7ca最终completed；SDK实际读取1.19.5/runnable，证明版本变化。通用发现source=unknown；生命周期scoped_discover核对prefix官方package.json后source=system，两者差异保留。
- 第二份官方npm 1.19.4安装到独立home/.local，退出0。双路径发现1.19.5与1.19.4；多search_paths卸载预览422空响应，local.rs::validate明确要求单一prefix。缩回第一prefix，指定卸载899a975c completed；恢复双路径只剩第二份1.19.4，SDK一致。
- 第一prefix空列表由SDK确认，正式install 060213da completed，发现与SDK为1.19.5/runnable。当前生产local.rs::run的Install使用管理员npm路径；不能将此结果称为native_primary官方脚本被API消费的证明。
- 指定卸载c7fec7ff completed。立即取消19b40ab1和executing取消8afc121c均cancelled/terminal，status与events独立重读200、实例空。executing采样捕获PID63600/PPID61059：node <isolated-root>/prefix/bin/npm i -g opencode-ai@latest --ignore-scripts=false --include=optional --allow-scripts=opencode-ai；取消后kill -0该PID失败，证明观察到的进程消失，不外推未观察孙进程。
- 测试prefix创建断链opencode，diagnostics 200/probe_failed/damaged_installation，指定卸载预览422空响应。之后删除断链。第二prefix npm uninstall --global --prefix <isolated-home>/.local opencode-ai退出0，最终API/SDK空列表。
- SDK UPGRADED、REMAINING、EMPTY、API INSTALL、FINAL EMPTY五次均退出0，使用生产授权方法、unwrap断言；有效自动测试计数仍为0，属于真实场景驱动。

## 待回归

默认features标准cargo run进入Running；同产物复制隔离Task51.app（com.rsung.shiju.task51-acceptance）供CUA定位。首次CUA状态超时，重试成功；窗口“识矩 - 项目”，AX只有窗口/菜单。右上设置300ms点击、缩放坐标尝试及super+comma均未进入设置，opencode-iced-entry.jpg为实际入口截图。未取得OpenCode卡片、网关连接、进度、断线toast/重连与SDK/Iced一致性截图；保留桌面回归，未修改其他任务正在变更的UI。

launch-identities为空，缺专用授权身份。providers/mod.rs明确返回“opencode provider session projection is not implemented”；缺身份不等于不支持。真实OpenCode默认Terminal执行程序/cwd/环境、适用官方账号与临时材料清理均未验证，不使用个人Terminal HOME或虚构令牌。

## Cargo与命令

原子mkdir动态领取slot-2，owner记录task51/会话/PID。CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖.cargo/config.toml的build.build-dir=target；保留缓存。

| 命令 | 退出码 | 有效自动测试 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 0 | 0，构建 |
| cargo run -p vw-desktop --bin shi-ju | -15，Running后主动终止 | 0，人工场景验收 |

无Rust变更，按任务无需Clippy；已有ld __eh_frame linker warning未改。未运行全量test、fmt检查、wasm或Makefile。rustcodegraph status成功；query install_command成功定位native_primary等实现；实际调用连线随后按领域文件核对，未重建索引。

SDK编译退出0，确切命令：

```text
rustc --edition=2024 <isolated-root>/sdk.rs -L dependency=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps --extern vw_gateway_client=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libvw_gateway_client-601c8d932ba563e6.rlib --extern tokio=/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/libtokio-49c3d960b83bfe81.rlib -o /Users/xiangmin/worker/code/shiju/target/rust-slot-2/task51-sdk
```

plan.md保持只读。本任务未完整通过，编号文件保留供回归。

## 原生补充与清理

独立native-home中执行bash <official-install.sh> --version 1.19.4 --no-modify-path。首次下载较慢主动终止curl，脚本退出143，不计通过；相同官方命令重试退出0，真实native-home/.opencode/bin/opencode --version退出0输出1.19.4。API installations/diagnostics均200并发现该路径、version=1.19.4/runnable、source=unknown。只读策略暂将home/search_paths指向native-home；update和uninstall预览均422空响应，local.rs::validate要求search_paths严格等于管理员prefix/bin，因此不能声称此原生实例经当前生产API可升级/卸载。恢复原策略最终空列表200。未把官方脚本安装成功扩展为完整原生管理已连通，原生API管理仍待回归。

标准桌面与独立bundle均主动退出-15，控制器记录[0,0,-15,-15]；全部使用槽位产物的本任务进程结束后自有slot-2释放，缓存保留。随机测试根、策略、令牌、npm包、原生副本及本任务下载临时目录已清理；不删除个人配置。有效自动测试0，仅人工/API场景证据。结束时个人四个常见入口仍不存在。
