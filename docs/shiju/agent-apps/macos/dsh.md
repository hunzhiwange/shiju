# DeepSeek Harness macOS 实机验收

2026-11-06，Asia/Shanghai。任务57；前序56生命周期API/SDK证据可用，未将其身份/UI待回归项误判为接口缺失。已读只读plan.md、来源与接口、任务13/19/24/31/32/39及56证据，按RustCodeGraph技能检查索引和供应商实现。本次只记录真实场景，无Rust修改；Windows/WSL/Linux未实机验收。

## 环境与隔离

macOS 27.0 (26A428)、arm64；Node v26.13.0、npm 11.20.0。开始和结束的command -v dsh均无结果；个人~/.local/bin/dsh、/opt/homebrew/bin/dsh、/usr/local/bin/dsh与~/.dsh均不存在。随机0700根内独立HOME/config/prefix/Applications，只链接既有node/npm，不读取个人厂商凭据。随机测试令牌与管理员策略0600，授权p/a/host的隔离前缀，Gateway只监听127.0.0.1:19457。

[官方仓库README](https://github.com/deepseek-ai/deepseek-harness#run-from-npm)给出npm渠道，固定上游配方为`npm i -g @deepseek-ai/dsh@latest --engine-strict`。本次registry latest=0.2.2-rc.1，旧版0.2.1-rc.5；新版要求Node>=24.9.0，bin=lib/bin.js。版本、来源、tarball及integrity见dsh-registry.json。未发明原生下载地址。

## API、SDK与真实安装

脱敏请求/响应见dsh-api.json；生产GatewayClient真实HTTP输出见dsh-sdk.txt；确切临时SDK rustc命令、实际程序/cwd/HOME与npm日志见dsh-runtime.txt。安装ID换为稳定标识，连续相同轮询合并为repeat，不保存令牌。

- 开始catalog 200：Dsh/DeepSeek Harness/native_cli；API/SDK空安装，launch-identities 200空列表。
- 在隔离前缀执行`npm install --global --prefix <isolated-prefix> @deepseek-ai/dsh@0.2.1-rc.5 --registry=https://registry.npmjs.org --fetch-timeout=30000 --fetch-retries=1`，退出0；实际`dsh --version`退出0、0.2.1-rc.5；API/SDK为runnable。npm安装用了约4分钟，提示五类依赖安装脚本未获allowScripts，不隐瞒或放宽脚本权限；版本可运行不证明所有插件/PTY功能正常。
- 正式update预览来源system，操作6aabd1d4 completed；实际版本0.2.2-rc.1，SDK一致。只读发现source=unknown，生命周期复核官方包与管理员prefix后锚定system，保留这一真实差异。
- 同渠道旧版安装到隔离HOME/.local，退出0。API发现两实例；指定首实例卸载fd4d1722 completed；第二实例仍0.2.1-rc.5/runnable，实际--version退出0，SDK一致。
- 收回到单一写前缀后，正式install fb16380b completed，API/SDK0.2.2-rc.1/runnable；正式uninstall edaca147 completed。
- 立即取消25b08096和executing阶段取消05e2be69均cancelled/terminal；独立重读status/events均200。执行中观察PID9361为`node <isolated-prefix>/bin/npm i -g @deepseek-ai/dsh@latest --engine-strict`，取消后kill(pid,0)为ProcessLookupError，安装为空。未外推未观察的孙进程。
- 隔离前缀创建dsh断链，diagnostics返回probe_failed/damaged_installation，卸载预览422。删除断链；第二份官方npm uninstall退出0。最终API/SDK空态。
- 主机策略online=false时读取503，恢复true后200；未把这项主机离线模拟描述为实际网络拔线。

## 启动边界及未验证

已安装真实ID、授权HOME cwd、args=[help]调用launch返回200；args=[--version]返回422。源码apps/launch/request.rs明确禁止客户端透传以`-`开头的参数，此422不是dsh不支持版本命令。CUA读取com.apple.Terminal被工具安全限制拒绝，无法验证终端实际输出、程序环境与cwd，因此不以launch 200冒充终端通过。

launch-identities为空；providers/mod.rs:33明确返回`dsh provider session projection is not implemented`。这是当前实现缺口，不是上游不支持。未提供或复制个人provider/account，指定供应商与适用官方账号启动及材料清理待专用身份和投影补齐后回归。

## Cargo记录

从1开始原子mkdir动态领取slot-2，owner记录task57/session和持有PID80138。CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，覆盖.cargo/config.toml中build.build-dir=target，不通过--target-dir绕过。

| 确切命令 | 退出码 | 有效自动测试 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 101 | 0，既有调用错误 |
| cargo run -p vw-desktop --bin shi-ju | 101 | 0，同上 |
| cargo run -p vw-desktop --bin shi-ju（其他工作修正错误后重试） | -15，人工结束 | 0，已出现Running |

初次错误是工作区既有`external.rs`对结构体变体PathRevealFinished使用元组调用；未越界修复，观察该文件被其他工作修正后重试成功。CLI与重试桌面的链接器__eh_frame大小warning保留在日志，不声明无警告。日志见dsh-cli-build.txt、dsh-desktop-build.txt、dsh-desktop-run.txt、dsh-desktop-retry.txt。

临时SDK首次选到不同Tokio构建产物，运行101/no reactor；改为匹配GatewayClient的Tokio产物后退出0，保留失败输出与最终命令。这是辅助链接选择错误，无产品修复。控制脚本首次机械替换误改sha256为sha257，启动前失败并释放自有锁，纠正后重新动态领取；未修改产品代码。

有效自动测试0，本项是实机/API/UI场景验收，不以编译或零测试冒充行为通过；按任务规定纯证据文档不跑Clippy。未执行全量test、fmt检查、wasm或Makefile。


## Iced与最终清理

专用Task57Iced.app使用同槽位默认features产物及隔离HOME/config/策略；初次直接窗口重试后，在专用bundle中成功取得交互窗口。通过现有右上网关菜单“新增网关服务”，设置测试地址19457与租户p/a；应用列表链条按钮填host与测试应用令牌。初始自动desktop-local主机与策略不符导致未授权toast，改用真实授权host后目录加载成功，未扩大策略权限。

Iced过滤dsh，空态与API/SDK相同；点击macOS安装预检、确认提交，0c057387进入Executing/Running，点击取消后Terminal/Cancelled；API独立重读status/events同为cancelled。截图dsh-iced-empty/running/cancelled.png。

把测试host online设false后刷新，出现“远端离线或服务不可用”及失败toast；恢复true后重试，目录恢复且保留取消终态。截图dsh-iced-offline/recovered.png。另一次正式API安装906dba7f completed后刷新，Iced显示0.2.2-rc.1/可运行，与SDK同一安装一致；截图dsh-iced-installed.png。只读来源unknown时UI保守显示来源待确认并不给卸载按钮，未伪称未知来源可安全卸载；管理员生命周期重新锚定后正式API卸载92403308 completed。最终Iced/API/SDK均空态，截图dsh-iced-cleaned.png。所有截图来自实际渲染，不是Element测试或拼图。

2026-11-06 14:49–15:05执行。安装、升级、卸载、隔离多实例保持、取消、破损入口、SDK/UI一致性与主机离线恢复已获证据；终端实态及指定供应商/官方账号依赖仍缺，完整验收保持已阻塞，编号任务保留。未将缺账号或工具访问限制标记为上游不支持。

自有专用桌面PID10610、重试桌面和Gateway全部退出，控制器确认退出码[0,101,-15,101]并释放slot-2本人锁；保留编译缓存。隔离安装已通过正式卸载清空，随机令牌/策略/config与临时.app随后删除；个人安装基线不变。未修改plan.md，不纳入其他工作区代码或编号任务删除。
