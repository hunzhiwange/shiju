# MiniMax Code macOS 实机验收（已完成，待回归）

2026-11-06 14:28–14:40，Asia/Shanghai。已读只读plan.md、任务55、来源与接口、task-15/31/45与批次54证据，并用RustCodeGraph检查实际调用。批次54生命周期可用，桌面和身份待回归不作为符号缺失。本次无Rust修改，其他平台未实机验收。

## 环境与来源

macOS 27.0 (26A428)、arm64、Node v26.13.0、npm 11.20.0。command -v mcode为空；个人~/.minimax-code、~/.local/bin/mcode、/opt/homebrew/bin/mcode、/usr/local/bin/mcode均不存在，结束后相同。随机0700根内隔离HOME/config/prefix/Applications，链接既有node/npm；随机令牌及管理员策略0600，仅p/a/host本机操作授权，Gateway监听127.0.0.1:19455。未读取或复制个人厂商凭据。

官方registry=https://registry.npmjs.org/@minimax-ai%2fcode，本次latest=0.7.6、前版0.7.4，bin=cli.js，Node范围>=22.19 <23 || >=24 <27，来源dist/integrity见mcode-registry.json。官方安装器https://filecdn.minimax.chat/public/install.sh已下载审阅并记录SHA256，未执行。native_grok_mcode.rs存在原生配方，但当前正式local.rs::run使用授权npm前缀；本次npm渠道结果不证明原生安装器实机通过。

## API、SDK与真实产物

mcode-api.json含脱敏请求/响应，mcode-sdk.txt来自生产GatewayClient的真实HTTP调用，临时SDK rustc命令及实际执行结果见mcode-runtime.txt。移除重复轮询，安装ID替换为稳定installation-1/2；令牌不入证据。

- 隔离HOME下执行npm install --global --prefix <isolated-prefix> @minimax-ai/code@0.7.4 --registry=https://registry.npmjs.org --allow-scripts=@minimax-ai/code,better-sqlite3，退出0；真实mcode --version退出0、0.7.4。API catalog 200/MiniMax Code/native_cli，API/SDK均发现0.7.4/runnable。最初空状态以路径基线证明；安装前未取得空API截图，不补造该证据。
- 正式update预览source=system，操作12e45e4c completed；实际--version退出0、0.7.6，SDK确认相同版本/runnable。只读发现source=unknown，生命周期复核管理员prefix及官方package.json后source=system，按真实差异记录。
- 第二份官方0.7.4安装至隔离home/.local，退出0。API真实发现两实例；指定第一实例卸载2bbc6d58 completed；之后API/SDK仅剩第二份0.7.4/runnable，其真实--version退出0、0.7.4。脚本冗余重试已删除的第一ID返回422，不算第二次卸载成功。
- 第一prefix空态SDK读取成功；正式install 618b9f7f completed，API/SDK确认0.7.6/runnable；正式卸载989a24e7 completed。
- 立即取消174c3525与executing取消680e9313均cancelled/terminal，独立重读status/events均200。执行中实际观察PID62163运行node <isolated-prefix>/bin/npm i -g @minimax-ai/code@latest；取消后kill(pid,0)为ProcessLookupError，实例为空。未外推未观察孙进程。
- 仅在隔离prefix创建指向不存在文件的mcode断链：diagnostics 200/probe_failed/damaged_installation，卸载预览422；移除断链。第二前缀官方npm uninstall --global --prefix <isolated-home>/.local @minimax-ai/code退出0；最终API/SDK空态。
- 真实已安装ID以授权HOME cwd及--version提交launch返回422，cwd=/也422；没有实际终端成功证据。launch-identities=[]，没有专用供应商/官方账号，不将缺账号记为不支持。task-31及providers实现表明mcode单次供应商投影尚未迁移，属于当前实现缺口，不能声称上游不支持；本次未用虚构身份绕过授权。

## UI与待回归

cargo run默认features真实启动。未打包进程不能被CUA按路径直接选取；首次临时.app仅链接二进制，CUA重启时未继承隔离HOME，出现既有用户会话，立即停止交互并结束本次槽位进程，不采纳该画面。随后改为临时启动脚本显式设置隔离HOME/config/策略/槽位，重新打开空会话并进入设置。

mcode-ui-unavailable.png是隔离应用列表实拍，显示“本机应用服务尚未就绪，请重启新版网关后重试”。客户端网关页面可打开，但配置测试网关时工具报告用户更改窗口，随后页面已变化；未取得可确认的连接结果。没有mcode卡片、SDK/Iced一致、UI安装进度/取消、断线恢复和错误toast通过证据。此UI待回归不等同于桌面编译失败，也不将服务错误截图冒充mcode卡片验收。

尚需：受控本机应用服务连接与UI完整场景；默认终端及实际程序/cwd/环境核验；专用供应商/适用官方账号及材料清理；原生脚本渠道如需声明已验收则另补证据。上述缺项保留编号任务，不宣称完整通过。

## Cargo与清理

原子mkdir顺序领取slot-5，owner记录task55/session/PID99048。
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5；CARGO_BUILD_BUILD_DIR同一路径，覆盖.cargo/config.toml的build.build-dir=target；全部Cargo与产物进程继承，无--target-dir绕过。

| 确切命令 | 退出码 | 有效自动测试 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 0 | 0，构建 |
| cargo run -p vw-desktop --bin shi-ju | -15，人工结束 | 0，窗口已启动 |

构建与启动日志见mcode-cli-build.txt、mcode-desktop-build.txt、mcode-desktop-run.txt。CLI/桌面链接器均提示既有__eh_frame大小warning。临时SDK rustc编译及各阶段调用退出0。本项人工场景，有效自动测试0，不以编译或零测试算行为通过；无Rust修改按任务无需Clippy，未执行全量test、fmt检查、wasm或Makefile。

全部自有桌面、SDK、包管理器与Gateway退出后释放本人slot-5锁，保留缓存，其他槽位不动。隔离安装/令牌/策略/临时.app已清理，个人mcode路径基线不变；plan.md未修改。
