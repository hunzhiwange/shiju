# Kimi Code macOS 实机验收（部分通过，实机收口阻塞）

2026-11-06 14:41–14:47，Asia/Shanghai。读取只读plan.md、任务56、来源与接口、task-17/31及批次55证据。55的真实API/SDK生命周期可用；UI及身份缺项不等同接口不存在。RustCodeGraph status成功，explore长时间无结果后中断，按技能回退读取当前local.rs、npm.rs、providers/mod.rs及API类型。无Rust修改，其他平台未实机验收。

## 环境及渠道

macOS 27.0 (26A428)、arm64；Node v26.13.0、npm 11.20.0。原有command -v kimi为空，个人~/.local/bin/kimi、~/.kimi-code、/opt/homebrew/bin/kimi、/usr/local/bin/kimi不存在，结束后相同。随机0700根内隔离HOME/config/prefix/Applications；只链接既有node/npm。随机令牌及管理员策略0600，仅授权p/a/host的隔离前缀操作，Gateway仅127.0.0.1:19456。未读取个人厂商凭据。

[官方README](https://github.com/MoonshotAI/kimi-code/blob/main/apps/kimi-code/README.md)明确允许npm安装，要求Node>=22.19.0。本次registry latest=2.5.1，前版2.5.0；bin=dist/main.mjs，包来源与integrity见kimi-registry.json。官方安装器https://code.kimi.com/kimi-code/install.sh下载成功，SHA256=270a86f2d2304529b6d8a3783fca9534874ebaeecb6cfcc1aebcdb6ce20ae1d7，未执行；无.sh地址404后依据官方README纠正。当前正式local执行器走管理员npm前缀，本证据不证明原生安装器通过。

## 真实行为证据

kimi-api.json保存脱敏请求与响应；kimi-sdk.txt来自生产GatewayClient真实HTTP调用；kimi-runtime.txt记录临时SDK rustc命令和执行结果。安装ID替换稳定标识，令牌不保存；重复连续轮询已合并。

- API catalog 200/Kimi Code/native_cli；安装前API空列表、launch-identities=[]。
- 隔离HOME执行npm install --global --prefix <isolated-prefix> @moonshot-ai/kimi-code@2.5.0 --registry=https://registry.npmjs.org --fetch-timeout=30000 --fetch-retries=1，退出0。npm提示postinstall未获allowScripts，实际kimi --version仍退出0、2.5.0；未将警告隐藏。API/SDK发现2.5.0/runnable。
- 正式update预览source=system，26d202c7 completed；实际--version退出0、2.5.1，SDK一致。只读source=unknown，生命周期复核官方package.json和管理员prefix后source=system，记录真实差异。
- 第二份官方2.5.0通过npm安装至隔离home/.local，退出0；API发现两实例。指定首实例卸载3385f380 completed，剩余第二份API/SDK为2.5.0/runnable，实际--version退出0。脚本重复请求已删除ID返回422，不算再次卸载成功。
- 收回策略至单一写前缀，SDK空态；正式install 89778c1f completed，API/SDK发现2.5.1/runnable；正式uninstall 57cae5c9 completed。
- 立即取消78a70783、executing阶段取消e9350687均cancelled/terminal；状态及events独立重读200。观察PID71532执行node <isolated-prefix>/bin/npm i -g @moonshot-ai/kimi-code@latest，取消后kill(pid,0)为ProcessLookupError；实例为空。未外推未观察的孙进程。
- 在隔离prefix创建指向不存在目标的kimi断链，diagnostics 200/probe_failed/damaged_installation，卸载预览422。首次辅助脚本误查pi导致IndexError，纠正为kimi重新验证成功，不修改产品代码。删除断链；第二份npm uninstall --global --prefix <isolated-home>/.local @moonshot-ai/kimi-code退出0。最终API/SDK空列表。
- 已安装真实ID使用授权HOME cwd、args=[--version]请求launch返回422空响应；未取得终端成功、实际cwd/环境证据。身份列表为空，未虚构provider/account；providers/mod.rs对Kimi供应商投影明确返回not implemented，这是当前实现缺口，不是上游不支持。

## UI阻塞与恢复条件

默认features桌面cargo run已出现Running；临时Task56Iced.app启动脚本显式指定隔离HOME/config/策略/槽位。CUA按专用.app路径及唯一bundle ID com.rsung.shiju.task56-acceptance两次读取均报-10005 timeoutReached，虽进程确实启动，未取得可交互窗口或截图。未操作个人ShiJu窗口，未伪造截图。

未验证：通过现有入口连接测试Gateway、Kimi卡片与SDK一致、UI进度/取消、断线恢复及错误toast；默认终端实际程序/cwd/环境；专用供应商与适用官方账号及材料清理。需要恢复可控制的专用Iced窗口，并提供专用身份或完成Kimi身份投影后回归。此任务是验收，缺少实际窗口/身份证据并非仅无关编译失败，因此保持已阻塞，保留编号任务文件。

## Cargo及清理

原子mkdir顺序动态领取slot-5，owner记录task56/session及控制PID66107。CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-5，覆盖.cargo/config.toml的build.build-dir=target；无--target-dir绕过。

| 确切命令 | 退出码 | 有效自动测试 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 0 | 0，构建 |
| cargo run -p vw-desktop --bin shi-ju | -15，人工结束 | 0，进程已启动 |

日志见kimi-cli-build.txt、kimi-desktop-build.txt、kimi-desktop-run.txt。CLI/桌面链接器既有__eh_frame大小warning见日志；本次未改Rust，无需Clippy。临时SDK rustc及各阶段真实调用退出0；辅助finish脚本首次退出1已在前文说明。有效自动测试0，本项以人工/API场景验收，不用编译或零测试冒充通过。未运行全量test、fmt检查、wasm或Makefile。

自有Gateway、桌面及临时.app子进程全部退出后释放slot-5本人锁，保留缓存；其他锁不动。隔离安装、令牌、策略和临时.app清理；个人安装基线不变。plan.md未修改。
