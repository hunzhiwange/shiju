# 任务2：自治权限与审批帮助

2026-11-16，保持当前目录与分支。实现完成；遵照用户覆盖指令不运行任何 Cargo 命令、单元测试、实机或最终回归，不因禁测保留任务卡。编译、运行注册数、passed/failed/ignored、字体实际整形绘制结果均未验证，退出码不适用。

## 静态归属与断言映射

来源两条测试名保持不变。宿主注册链沿用 baseline.md：components → system_settings_autonomy → ui_refresh_22_tests；领域新增 lib.rs → host_split_2_tests。两条领域候选分别为 `host_split_2_tests::settings_help_gap_11_permissions_explanations`、`host_split_2_tests::settings_help_gap_11_channel_formats`。非 wasm cfg，tokio test，每条子进程以 module_path 完整名称加 --exact 启动，保留 1 passed / 0 failed 防零测试断言，但本次未执行。

|旧用例|领域断言|保留宿主断言|
|---|---|---|
|permissions_explanations|三种语言与明暗主题、420/760 宽度；重定向/级别/自然语言审批帮助；静态说明隐藏、悬停/固定改变截图、输入位置不变、复制内容、Escape 恢复；三项提示；三个自治级别、中风险/高风险、移除、请求确认消息|上述级别和四种控制消息的 Local → SettingsMessage 转换，以及帮助 CopyCode 转换|
|channel_formats|相同语言主题与宽度；排除工具、审批人、通道模式帮助及格式示例；三项提示、校验错误文本、Backspace 保留通道消息载荷|通道输入 Local → SettingsMessage 载荷转换|

两条宿主 UI 重用例改为两条普通映射测试，领域新增两条，不声称宿主测试条数下降。security、其它自治测试族与共享宿主渲染夹具未改。

## 夹具和接线

直接构造 AutonomySettingsState 与 autonomy_inputs::View，不创建 App、不初始化网关或配置。使用 desktop_assets::fonts::load_all、原 Renderer/UserInterface、事件、layout、draw 和 screenshot 路径。局部帮助端口复用宿主实际使用的 desktop-chat-view selectable_titled_text，实现测试局部 SelectionMessage；非本组操作显式 panic。新增该已有工作区库的 dev-dependency 并手工同步锁文件依赖边，无新增包；它不属于生产依赖。desktop-chat-tools 已依赖 settings-agents，此边只用于 dev 场景，未通过 Cargo 验证依赖解析。无反向宿主源码依赖、path/include 或 App 复制。

截图保留内存像素比较，未新增 image 依赖与文件导出；原 helper 中仅供 Sandbox backend 的菜单搜索分支不迁移，其宿主实现保留。本组不涉及 sandbox。

宿主生产 view/view_overlays 已经调用同一领域实现并提供真实帮助 renderer，静态确认无需改生产接线。新增模块只服务本组，未建跨库夹具。

## 静态检查

读取任务1 evidence/baseline.md（计划的 task-1.md 实际文件名为 baseline.md）。RustCodeGraph status、符号定位、精确源码与差异核对完成。只对新增测试文件调用 rustfmt，语法格式化成功；git diff --check 成功。两者均不证明类型正确或行为通过。

任务前宿主来源文件1524行，本次结束1396行；新领域模块456行（含局部夹具）。源码候选为领域2条、宿主保留2条；实际 libtest 清单、测试执行及所有计划 Cargo 命令均未运行。后续建议在允许验证时执行本任务原定领域过滤和宿主两项过滤，并检查真实字体绘制。
