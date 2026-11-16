# 任务4：通道权限说明

按用户覆盖要求完成代码与静态审查；未运行任何 Cargo、单元测试、实机测试或最终回归。没有运行退出码、通过数或截图证据，以下注册数量均为源码静态统计，不能代表运行通过。

前置清单：读取 evidence/baseline.md 与 before-host-tests.txt；当前目录、分支保持原样。

## 逐项断言映射

领域注册：desktop-settings-channels/src/lib.rs → host_split_4_tests（同名3项）。宿主注册仍为 system_settings_channels.rs → ui_refresh_17_tests（同名3项）。

|原用例（settings_help_gap_06_ 前缀）|领域断言|保留宿主断言|
|---|---|---|
|feishu_modes_keep_port_and_qr_controls|Webhook/Websocket；隐藏旧说明；端口提示位置；4321退格产生432；扫码说明及QrRequested消息|两种模式下端口帮助hover/pin/菜单/复制/关闭|
|dingtalk_keeps_allowlist_consequences|默认与命名实例；旧说明隐藏；空白名单拒绝与星号允许；编辑器宽度；两种字段键的编辑消息|两种实例的钉钉及允许用户帮助卡片全部交互|
|ordinary_and_wildcard_lists_and_disabled_form|邮件无星号提示/无帮助图标；邮件编辑事件；Lark星号提示；禁用邮件表单隐藏；重新启用消息|Lark格式帮助及禁用Email说明卡片全部交互|

保留原三种尺寸/语言/明暗主题，领域夹具直接加载desktop_assets字体并使用真实UserInterface update/draw/screenshot。领域子进程以完整模块测试名运行并检查1 passed；未执行。局部Fixture只构建四个通道的真实配置和ViewState，无App、网关初始化、跨目录include或宿主反向依赖。不迁二维码执行或通道运行时。

帮助卡片复制发出宿主CopyCode消息，领域Message只有None/Settings，因此本组保留薄宿主用例，不增加测试专用生产接口。宿主现有面板适配与map_message已接入领域实现，无需更改。领域调用同一生产面板；页面整体装配仍由宿主负责。

实际迁移3组领域断言，保留3条宿主交互用例；宿主测试条数净减少0，领域新增3条。复用现有开发依赖，无清单/锁文件修改。未扩展到同来源的其它测试族。静态检查包含注册、消息匹配、配置必填字段及差异审查。

已知静态限制：现有领域代码引用settings_group_heading_with_help/settings_flexible_row_with_help等控件接口，而当前desktop-ui-controls导出与签名并不完全对应；这是迁移前已有源码状态，本任务不跨范围修复，编译与渲染结果均未验证。
