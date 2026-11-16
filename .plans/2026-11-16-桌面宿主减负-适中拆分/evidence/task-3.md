# 任务3：自治列表与工具交互

实现完成。遵循用户覆盖指令，保持当前目录、分支，不执行任何 Cargo 命令、单元测试、实机或最终回归。编译、libtest 实际注册数、passed/failed/ignored、字体整形绘制均未验证，退出码不适用；按用户要求删除任务卡，不保留待回归状态。

## 断言归属

|原测试名|迁入领域断言|宿主保留断言|
|---|---|---|
|settings_help_gap_11_list_editor_explanations|三语言/明暗主题、420/760宽度；五类帮助隐藏静态说明、hover/pin截图变化、输入布局不变、复制正文、Escape恢复；四项提示；AllowedCommands添加和确认删除索引0消息|Local添加及确认删除消息转换为SettingsMessage，保持列表类型和索引|
|ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids|下拉与添加垂直对齐、菜单hover截图区别、滚动变化、外点关闭不误触添加、End/Enter保留fixture_tool_27、已存ID显示、添加列表类型|Local草稿ID及添加消息转换为SettingsMessage|

宿主两条重UI测试替换为同名两条普通映射测试；领域新增两条，不声称宿主测试数量下降。领域静态注册：lib.rs → system_settings_autonomy → host_split_3_tests（非wasm）。新模块为生产领域模块的子模块，直接调用私有list_editor，无新增生产接口。宿主view和view_overlays原已调用同一领域实现并map，无需改变生产接线。来源原有list_editor未在宿主父模块导入，本次迁移消除该来源测试对未接入私有函数的直接引用。

## 夹具与范围

任务2夹具抽到本库autonomy_test_support，任务2/3两组真实使用；SelectionMessage实现仍仅一份。保留字体load_all、Renderer/UserInterface、事件、布局、draw、screenshot与像素比较路径，不创建App、不初始化网关。补充focusable几何采集供菜单定位；子进程隔离接收调用模块路径，使用完整测试名和--exact，保留1 passed/0 failed防空跑断言，未执行。列表状态仅复制本组直接依赖的命令、中文禁止路径和28项工具夹具。

新增共享夹具是本任务文件清单外唯一必要新增文件；修改任务2测试仅为了改用共享夹具，无其它测试族/生产行为变化。未改清单、锁文件，无宿主反向依赖、include/path挂载宿主源码或跨库框架。

## 静态检查与限制

读取任务2证据及直接源码；RustCodeGraph status检查后，针对已由前置任务改动的源码读取当前文件。对三个新建/调整测试文件运行独立rustfmt成功；git diff --check成功。核对模块注册与消息映射。以上不证明类型正确或行为通过。

领域新增候选2条，宿主保留候选2条；实际清单未运行。后续允许验证时建议执行计划原定领域host_split_3_tests过滤、两条宿主过滤与宿主lib清单，补查真实字体绘制。任务2夹具抽取后的两条用例亦需确认。所有Cargo检查按本次用户要求延期。
