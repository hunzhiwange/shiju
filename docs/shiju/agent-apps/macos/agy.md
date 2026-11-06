# Antigravity macOS 实机验收

首轮记录保留供追溯；原生写路径缺口已在文末继续执行记录中修复，最终状态以该节为准。

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

## 继续执行：原生写路径修复与正式API验收

2026-11-06，用户授权修复可恢复缺口后继续执行。范围仅agy，不扩大其他原生应用或个人安装权限。

生产连线：local生命周期对Agy调用官方安装器已支持的--dir，管理员prefix/bin通过独立argv传入，不拼入shell文本。安装/更新HOME和TMPDIR均为prefix/.shiju-agy（0700），沿原有env_clear、授权写锁、重建计划、Operation进度及取消执行。保留现有管理员npm/node工具校验契约，但不调用npm或Node元数据门控安装原生agy。

只有本执行器执行官方安装/更新成功、原生文件头及真实--version探测通过后，才原子写入0600 SHA-256收据；读取、升级选择与现有原生卸载共同核验收据及精确prefix/bin/agy。拒绝父目录/bin重定向、非私有收据目录、收据/二进制符号链接、脚本及被替换的内容。手工安装仍unknown；后台自更新或外部修改若使摘要失配，同样保守降为unknown，不靠路径伪造来源。卸载保留私有配置/缓存目录，不递归删除用户资料。新增sha2 0.11复用工作区已有依赖，不新增运行时配置或RPC。

正式Gateway（127.0.0.1:19458）使用新建0700隔离根、0600令牌/管理员策略，个人HOME/钥匙串不动。新增证据见agy-resume-api.json、agy-resume-runtime.txt及agy-resume-sdk.txt：

- 初始catalog 200、installations空、launch-identities空；正式安装预检200/system、提交202，7f727d7e经过Executing/Postflight后completed。实际1.5.2/runnable，读取source=system。
- 正式update预检200、037d8748 completed；实际版本保持1.5.2。此前官方update输出“已最新”，此处同样不声称发生可观察版本升级；旧版到新版变化仍待回归。
- 正式uninstall预检200、3ec4cb0f completed，精确安装入口消失，API/SDK为空。
- 再次正式install在Executing取消，da367659为cancelled/terminal；独立GET status/events同终态。观察到本次安装器shell PID83496/83497，取消完成后均已回收，API空态。没有外推未观察的孙进程。
- 再正式install dce17f7c completed；第二份通过同一官方安装器--dir安装到另一隔离HOME，退出0、SHA-512验证通过。API/SDK读到两份1.5.2，其中受管实例system，手工官方实例unknown。正式指定受管实例卸载ba225651 completed后，第二份仍1.5.2/runnable、真实--version退出0。随后只手工移除本任务第二份二进制，API/SDK最终空态。
- SDK最初手工链接到不匹配Tokio而运行101；根据GatewayClient fingerprint依赖找到匹配的tokio-49c3d960b83bfe81，重新rustc链接并真实HTTP读取退出0，保留确切命令。不是产品运行时故障，也不将辅助失败当通过。

本阶段从1–10原子mkdir动态领取slot-4，owner为task58/resume、持有PID60261；CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-4。所有Cargo、辅助SDK和产物子进程均在本槽位生命周期内；最终退出后才释放本人锁并保留缓存。

| 确切命令 | 退出码 | 有效测试与说明 |
| --- | --- | --- |
| cargo test -p vw-agent-apps --lib cc_apps_agy_managed -- --nocapture（修改前） | 101 | 1失败/1通过；空前缀Agy预检被原有npm路径拒绝 |
| cargo fmt --all（初次） | 1 | 其他Git模块新建repository_tests.rs尚缺；改用rustfmt格式化本任务文件 |
| cargo fmt --all（最终） | 0 | 其他工作补齐文件后通过 |
| cargo test -p vw-agent-apps --lib cc_apps_agy_managed -- --nocapture（最终） | 0 | 6通过；含空前缀、手工来源拒绝、摘要替换/符号链接拒绝、bin重定向拒绝、取消回收、真实C原生fixture安装/更新/指定卸载 |
| cargo test -p vw-agent-apps --lib cc_apps_native_uninstall_scope -- --nocapture | 0 | 5通过；相邻原生删除边界 |
| cargo test -p vw-agent-apps --lib cc_apps_hermes_agy_install -- --nocapture | 0 | 2通过；既有安装/更新配方 |
| cargo clippy -p vw-agent-apps --all-targets --all-features -- -D warnings | 0 | 仅本crate检查，无行为测试计数 |
| cargo clippy --all-targets --all-features -- -D warnings（初次） | 101 | 无关Git面板可见性/字段/导入错误；未越界修复 |
| cargo clippy --all-targets --all-features -- -D warnings（最终） | 0 | 其他工作修复后全仓通过 |
| cargo build -p vw-cli --bin shiju | 0 | 新生产Gateway实机产物；保留既有__eh_frame链接器提示 |
| cargo run -p vw-desktop --bin shi-ju（首次） | 101 | 无关Git面板可见性与Icon::Git错误，见desktop-resume日志 |

共13项不同聚焦/回归用例通过，另有真实官方网络安装/API/SDK场景；fixture仅替换官方curl下载边界，没有将fixture当官方二进制实机证据。最初实现编译曾修正不存在的Result::is_err_or与sha2 0.11摘要格式化，不把编译失败冒充业务红灯。无全量test、fmt检查、wasm或Makefile。

### Iced继续验证与最终状态

其他工作修正Git面板后，相同`cargo run -p vw-desktop --bin shi-ju`默认features重试编译成功并进入Running，主动结束退出-15。专用Task58Resume.app使用复制的同槽位新产物及隔离HOME/config；CUA显式Raise后窗口归属正确，顶栏按钮在控件下半部点击可正常响应，未修改GUI产品代码。

通过现有“新增网关服务”配置19458、p/a，再用链条入口输入授权host与随机测试令牌。默认desktop-local与本策略不符时确有未授权toast，改为真实授权身份后加载14项目录，过滤agy后空态与API/SDK一致。Iced点击macOS安装预检、确认提交，4605353f显示Running（预检/执行期间），点击取消后Terminal/Cancelled；独立API status/events均200/cancelled。截图agy-iced-empty.jpg、agy-iced-running.jpg、agy-iced-cancelled.jpg。

随后正式API安装d509a31c completed。首次Iced读取遇2秒版本探测超时，保守显示“版本待确认/需要检查”，保留agy-iced-probe-timeout.jpg，不将超时说成不支持或伪改可运行。刷新重读后Iced显示“系统安装 · 1.5.2 · 可运行”，与同阶段API/SDK一致；截图agy-iced-installed.jpg。未通过放宽探测检查掩盖暂态失败。

管理员测试host online=false时API503，Iced刷新显示远端离线/服务不可用及错误toast，截图agy-iced-offline.jpg；恢复true后重试，1.5.2/可运行恢复，截图agy-iced-recovered.jpg。此为主机离线模拟，不是实际拔网线。正式API卸载a1b68623 completed后，API空态；Iced最后刷新确认0安装、agy待安装。截图均为原生渲染JPEG，直接保存工具原始字节，没有拼图或合成。

SDK两实例、早一轮最终空态以及本轮安装后1.5.2/system/runnable均有成功结果。最后一次UI清理后补跑SDK时，可执行文件意外消失而FileNotFoundError，不能声称该次SDK运行成功。检查发现slot-4的owner已被替换成另一任务PID19949，原持有PID60261仍存活，且槽位原有依赖/临时SDK产物消失；原因不在本任务范围内，未删除该任务锁或缓存。此前完成的13项测试、最终全仓Clippy和真实API/SDK证据保留。为避免原持有器finally误删新owner，只结束本任务已核实的持有进程，当前slot-4锁保持原样。没有将外部槽位变更视为本任务可绕过规则的许可。

自有专用bundle、默认桌面重试、Gateway、辅助SDK及安装器均结束；运行控制器退出记录[-15,101]（网关/首次桌面），重试桌面另为-15。两份本任务测试安装已清空，确认随机根无相关进程后删除其中令牌、策略、配置、缓存、临时.app，清空CUA临时变量；个人安装/钥匙串未动。保留共享构建缓存，未修改plan.md。

最终：实现与可执行验收已完成，任务58改为**已完成，待回归**并保留编号文件。剩余项为真实旧版到新版升级变化、默认终端内程序/cwd/环境实态、专用供应商及适用官方账号启动/材料清理。没有安全已验证旧版来源，也没有隔离官方钥匙串执行主机及专用身份；不伪造账号或将缺证据标为上游不支持。下一步在这些外部条件具备时回归上述缺项，以及最后一次SDK清理后空态。
