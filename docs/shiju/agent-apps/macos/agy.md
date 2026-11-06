# Antigravity macOS 实机验收

2026-11-06 15:07–15:15，Asia/Shanghai；任务58。已读只读plan.md、来源与接口、任务11/13/16/19/21/31/32/34/44及批次57证据。57状态为已完成，待回归；真实API/SDK和目录依赖存在，未把身份/UI缺项判为符号缺失。本次仅证据文档，无产品代码修改。Windows/WSL/Linux未实机验收。

## 环境与来源

macOS 27.0 (26A428)、arm64；本机Node v26.13.0/npm 11.20.0仅作为管理员生命周期环境工具，本应用不走npm安装。安装前后command -v agy均无结果，/Applications和个人Applications均无Antigravity.app；未卸载或覆盖个人实例。

随机0700根内使用独立home/config/second/Applications及安装前缀，随机0600令牌/策略，Gateway只监听127.0.0.1:19458，租户p/a、授权host。无复制个人厂商凭据或读取/修改全局钥匙串。

[官方安装器](https://antigravity.google/cli/install.sh)与生产native_hermes_agy.rs配方一致，默认写入$HOME/.local/bin，支持--dir；按自身manifest下载darwin_arm64包并验证SHA-512。实际manifest版本1.5.2、官方storage.googleapis.com下载地址和摘要见agy-manifest.json；安装器SHA-256及原始脱敏输出见agy-runtime.txt。没有编造旧版下载URL。

## 实际场景

请求/脱敏响应见agy-api.json；生产GatewayClient真实HTTP结果见agy-sdk.txt。安装ID换为稳定标识，不保留可反解随机本机路径的base64 ID。

- 官方bash安装器退出0，实际agy --version退出0、1.5.2；两次安装均显示下载校验通过。API catalog 200为Antigravity/native_cli，发现1.5.2/runnable；SDK结果一致。安装发生在Gateway构建期间，安装前空态是文件系统基线，没有宣称存在安装前API截图。
- 首份位于隔离home/.local/bin，第二份由同一官方安装器--dir安装到second/.local/bin，均退出0。API/SDK发现两实例；仅手工unlink第一份本任务测试二进制后，第二份仍1.5.2/runnable，真实--version退出0。此项证明发现/实例保持，不冒充正式卸载API通过。
- 隔离HOME实际执行agy update，退出0，输出current version 1.5.2及already on the latest version。只证明当前无需更新，没有可观察升级变化；安装器仅查询latest，未找到已验证的安全旧版渠道，升级待回归。
- 正式actions/preview对install、选定实例update/uninstall均422（空响应体）。收敛单一写前缀、在该前缀提供npm/node后仍422；最终空安装状态再次install预检仍422，排除单纯已有安装的影响。
- 来源只读报告为unknown。local.rs::scoped_discover只为经实际package.json核验的npm包锚定system，selected拒绝unverified installation source；install分支调用npm::install_args，而该函数拒绝Agy。任务16底层原生配方已实现，但当前受限HTTP写适配器未接入它；这是现有实现限制，不是上游不支持自动安装/升级/卸载。未伪造来源或绕过策略。无可提交操作，因此进度、取消及操作断线重读未验证。
- 首份路径改为本任务创建的断链后diagnostics 200，状态probe_failed/damaged_installation；完整第二份仍runnable。移除断链和第二份测试二进制后API/SDK空态。
- 主机策略online=false时读取503，恢复true后读取200；这是主机离线模拟，不是实际网络断线，也没有UI toast证据。

## 启动与UI缺项

真实已安装ID、授权home cwd、args=[help]调用launch返回200；独立在隔离HOME/cwd运行真实agy help退出0，输出为空。前者只证明请求被接受，后者只证明直接CLI命令退出，均不能证明默认终端的真实程序/cwd/环境。

launch-identities 200为空。Agy的Gemini供应商投影存在（providers/mod.rs、任务31），但没有专用供应商身份及可用模型服务，不将空列表说成不支持。任务32已定位官方个人账号使用固定OS钥匙串项，需要隔离OS钥匙串执行主机，当前实现明确拒绝；未触碰个人登录，真实官方账号启动仍待专用主机/身份回归。

默认features桌面已通过指定cargo run进入Running。另启动专用Task58Iced.app，先后尝试符号链接和独立复制的同槽位程序；CUA accessibility标识为专用“识矩 - 项目”，但截图/点击后显示另一既有窗口内容，无法可靠证明窗口归属。第二次复核仍相同，停止UI操作；没有保存其他窗口截图冒充本任务证据。所需API/SDK/Iced对应截图、错误toast、UI离线恢复和终端实态仍未验证。

## Cargo与清理

原子mkdir按1–10动态领取slot-2，owner记录task58/会话与控制PID32143。CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，CARGO_BUILD_BUILD_DIR同绝对路径，覆盖.cargo/config.toml的build.build-dir=target，无--target-dir绕过。

| 确切命令 | 退出码 | 有效自动测试 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 0 | 0，构建 |
| cargo run -p vw-desktop --bin shi-ju | -15，主动关闭 | 0，日志已进入Running |

日志见agy-cli-build.txt、agy-desktop-build.txt、agy-desktop-run.txt，保留既有链接器__eh_frame大小warning。辅助SDK的确切rustc命令见agy-runtime.txt；最初两次链接到另一Tokio构建产物而no reactor/101，选择与GatewayClient匹配的libtokio-52b6635379736477.rlib后退出0，SDK保留失败及最终输出。这是辅助链接选择问题，无产品修复。RustCodeGraph status正常，但explore超过两分钟无输出，结束本次查询后读取实际源码核对，未修改索引。

本项是人工实机/API/SDK验收，有效自动测试0，不以编译或零测试当行为通过；按任务纯文档规则不运行Clippy。未运行全量test、fmt检查、wasm或Makefile。

Gateway、cargo run桌面、专用bundle及使用槽位产物的进程全部结束，控制器退出记录[0,0,-15,-15]，确认无随机根相关进程后释放本人slot-2锁，保留缓存。两份测试安装已清空，随后删除随机根内令牌/策略/配置/缓存及临时.app；个人基线不变。未改plan.md及其他工作区改动。

完整验收未通过：原生HTTP写路径、可观察升级、Iced/终端实态及专用身份缺项仍在；保留任务58为已阻塞，不能把部分发现/直接安装成功报为整项通过。
