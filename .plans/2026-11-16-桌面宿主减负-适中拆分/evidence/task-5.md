# 任务5：通道表单显示与编辑

遵照用户本次覆盖要求，仅写代码、静态核对；没有执行任何 Cargo、单元测试、实机测试或最终回归。无运行退出码、有效通过数及截图证据；以下数量均为源码统计，不代表编译注册或测试通过。实现完成后删除本任务文件，不保留待回归。

读取任务4短证据与任务1静态基线。保持当前目录和分支。

## 断言映射

|原用例（ui_refresh_17_前缀）|领域 host_split_5_tests 同名用例|保留宿主断言|
|---|---|---|
|preserves_values_and_disabled_states|Webhook密钥回显；900宽标签/控件水平对齐；640宽竖排与宽度；Email页签消息；420宽密码输入及编辑内容；切换表单不丢Webhook密钥；同长度密码截图一致、不同长度截图不同；禁用表单无输入/焦点且忽略陈旧事件；重新启用消息|真实宿主页签点击映射TabSelected(Email)，宿主settings::update调用后active_tab回显|
|edits_long_inputs_and_number_controls|繁体中文420宽长URL追加编辑；Telegram多行列表宽度、焦点非编辑消息、中文输入编辑消息及draw/screenshot；数值输入与徽标对齐、退格8787→878消息，Escape/Enter无消息|整条迁出，无保留|

领域使用现有Fixture局部状态，以update::updates::update生产入口启用四个所需通道、填值和编辑；Services仅提供显式失败网关客户端与无本地文件的ConfigAccess，返回持久化/网络Task丢弃，不执行。未复制宿主settings::update、App/AppState或网关初始化。使用领域overview::view及真实panel，宿主原panel代理和map_message已连线，无生产修改。

领域独立注册链：lib.rs cfg(test) → host_split_5_tests，两个同名tokio测试；宿主仍经system_settings_channels.rs → ui_refresh_17_tests注册第一条薄用例，第二条源码移除。源码候选数量：领域+2，宿主-1；实际libtest清单未运行。

新增channel_render_fixture.rs，由host_split_4_tests与host_split_5_tests两个真实测试组共享；从任务4原样提取字体加载、真实UserInterface事件、geometry、draw/screenshot，不增加跨库框架。此相邻文件调整理由是复用计划要求的通道渲染夹具，任务4断言不变。保留子进程完整模块名过滤和1 passed检查，尚未执行。

本任务修改：领域lib.rs、host_split_4_tests.rs、新增channel_render_fixture.rs与host_split_5_tests.rs、来源宿主测试文件、本证据及自身任务文件删除。没有改Cargo清单/锁文件、生产控件或其它测试族。

静态审查：核对消息类型、update签名、依赖已存在、生产宿主代理及模块注册；仅本次领域测试文件用rustfmt整理；git diff --check用于文本检查。任务4已记录的desktop-ui-controls接口不一致属于既有状态，本次不跨范围修复。所有编译及真实渲染结果仍未知，未宣称暗黑主题/字形整形已通过。
