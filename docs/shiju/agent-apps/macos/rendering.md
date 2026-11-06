# macOS 明暗主题与中文渲染验收

2026-11-06（Asia/Shanghai），macOS 27.0 / 26A428，arm64。任务60，依赖批次59已交付的生产接口和GUI；59待回归的专用账号/历史版本不影响本项字体与布局验收。只读plan.md，未重构其他设置页。

## 方法与范围

真实Iced原生窗口直接调用生产安装升级/卸载overlay、preferences::overlay、launch::overlay，加载生产字体与PingFang SC。不是仅构造Element的单元测试，也不是HTML模拟。每组明暗两主题，600×650逻辑窗口（启动560×650），截图为Retina物理像素。首轮46场景，补强后46场景；重复执行不计为92种不同场景。

仅修改四个现有窗口验收夹具：设置与启动错误重复40次、启动cwd含中文用户名，滚动底部后实际执行focus_next；生命周期/卸载在同一原生窗口第4帧由Executing/Running更新为Terminal/Completed，再截图。没有修改产品逻辑、权限、安装或账号数据；夹具名称和路径均为合成数据，不是个人信息。

## 场景与审查

| 生产界面 | 每主题场景 | 两主题数量 | 新证据目录 |
| --- | --- | --- | --- |
| 安装升级 | 多实例、确认、动态进度、明确错误、长预检/禁用、取消 | 12 | [lifecycle](rendering/extended/lifecycle/) |
| 卸载 | 多实例、确认及残留说明、动态进度、过期错误、长预检/禁用、取消 | 12 | [uninstall](rendering/extended/uninstall/) |
| 应用设置 | 保存/重启提示、超长错误、处理中禁用、中文WSL顶部、滚动底部/焦点 | 10 | [preferences](rendering/extended/preferences/) |
| 终端启动 | 完成、超长错误、处理中禁用、缺身份、中文路径/参数底部/焦点、本机现有配置 | 12 | [launch](rendering/extended/launch/) |

人工逐图检查16项（两轮取样，不冒充46项全部手工点击）：首轮安装暗色长说明/确认、亮色进度/错误；卸载亮色确认/取消、暗色错误/长说明；设置暗色滚动底部/亮色禁用；启动暗色参数底部/亮色空身份；补强后设置亮色长错误/暗色滚动底部，启动暗色长错误/亮色中文路径焦点。观察中文正常换行，无缺字、截断按钮、滚动条覆盖内容或字形整形panic。原生窗口程序按帧更换数据并自动截图；不是通过CUA逐个点击46个场景。

4px滚动条、两侧7px留白由生产控件参数与截图共同核对；模态内容单独滚动，标题及底部按钮固定，取消/关闭/刷新等辅助按钮复用统一样式。设置与启动控件使用settings_*样式及统一padding/text size。源码对照system_settings_common及custom_model_modal.rs；system_settings_apps内无MONOSPACE，中文与混合路径使用默认字体。

本项没有真实写入设置、安装卸载或启动厂商CLI；身份、Windows/WSL能力均是用于macOS渲染的显式夹具，不代表其他平台实机通过。刷新/取消/toast的消息派发由cc_apps_desktop_聚焦行为测试覆盖；本轮截图主要验收四类生产表单，不声称重新完成外网安装闭环。标准cargo run进入Running后按120秒上限主动SIGTERM，返回-15；不把主动结束当作自然退出0。

CUA确认了原生桌面环境，但bundle ID对应多份既有应用，直接二进制路径也不受getApp支持，未误操个人已安装应用。通过Iced自带window::screenshot采集实际渲染作为等效路径，不使用旧版桌面截图。辅助联系表脚本因缺Pillow失败，未安装依赖；改为直接查看原始PNG，不影响原生验收。

## Cargo与命令

两阶段均从仓库根原子mkdir依次动态领取**slot-1**，owner记录计划、task60、会话rendering与控制进程PID。两阶段均设置：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1

第二项覆盖.cargo/config.toml中的build.build-dir=target；Cargo、脚本、窗口和标准桌面进程继承相同绝对目录。阶段命令和退出码见[首轮](rendering/commands.txt)及[补强后](rendering/extended/commands.txt)，完整渲染日志在各目录的lifecycle.log、uninstall.log、preferences.log、launch.log，标准启动日志[desktop.log](rendering/desktop.log)。

| 确切命令（均默认features） | 退出码 | 有效数量 |
| --- | --- | --- |
| cargo fmt --all | 0 | 格式化，检查本任务差异无额外产品改动 |
| cargo test -p vw-desktop --test apps-lifecycle-window | 0（两轮） | 12原生渲染场景/轮 |
| cargo test -p vw-desktop --test apps-uninstall-window | 0（两轮） | 12原生渲染场景/轮 |
| cargo test -p vw-desktop --test apps-preferences-window | 0（两轮） | 10原生渲染场景/轮 |
| cargo test -p vw-desktop --test apps-launch-window | 0（两轮） | 12原生渲染场景/轮 |
| cargo test -p vw-desktop --bin shi-ju cc_apps_desktop_ -- --nocapture | 0 | 0测试，31过滤；不计行为证据 |
| cargo test -p vw-desktop --lib cc_apps_desktop_ -- --nocapture | 0 | 29通过，8583过滤 |
| cargo clippy --all-targets --all-features -- -D warnings | 0 | 全仓通过 |
| cargo run -p vw-desktop --bin shi-ju | -15 | 启动运行后120秒上限主动SIGTERM；0自动测试 |
| git diff --check（本任务文件） | 0 | 无空白错误 |

状态：已完成。29项自动行为测试通过，46种原生渲染场景两轮通过，16项人工截图审查。首轮保留15张基线截图（含Running进度），补强后保留46张；其余重复基线截图不重复入库。窗口harness=false，不能将其计成46条Rust单元测试。无产品缺陷修复，故未编造TDD失败用例。

完整日志只有既有macOS链接器__eh_frame大小性能提示和系统IMKCFRunLoopWakeUpReliable消息，无panic；Clippy本轮最终退出0。所有本次Cargo/窗口/标准桌面进程均已结束，两阶段slot-1锁均已释放，缓存保留。其他会话的slot-2/slot-8、既有运行桌面和Git改动未触碰。未运行无过滤cargo test、fmt检查、wasm或Makefile。

下一步：继续后续计划；各厂商专用身份/历史版本闭环仍以前序独立待回归记录为准，本项不代替那些实机验收。
