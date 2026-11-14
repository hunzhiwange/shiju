# 编辑器配置原生验收（2026-11-14）

正式依赖为 Git v0.5.4 / e8ec1e34b46ff19fbe5c1ffed4abd4483fc025a9。本地修复为底层 840ffb6b92b981661acb6e0af6998b44fbf18698；以下成功窗口使用 v0.5.4 干净快照叠加该提交的文本绘制修复，通过临时 CLI patch 构建。仓库配置没有本地 patch，不代表正式依赖中文问题已解决。

## 构建与验证

先获取包含中文修复的编辑器源码，使其包版本与应用的0.5.4匹配；本轮使用 v0.5.4 的 git archive，仅叠加该提交四文件差异，排除共享工作区并发 search 改动。以 EDITOR_FIX 指向该临时源码中的 iced-code-editor 包目录：

```sh
# EDITOR_FIX must be exported as an absolute path to the temporary package.
python3 scripts/cargo_slot.py sh -c '
  cargo build --config "patch.\"https://github.com/doyouhaobaby/iced-code-editor.git\".iced-code-editor.path=\"$EDITOR_FIX\"" -p vw-desktop --test editor-config-window &&
  cargo test --config "patch.\"https://github.com/doyouhaobaby/iced-code-editor.git\".iced-code-editor.path=\"$EDITOR_FIX\"" -p vw-desktop --lib editor_config_ -- --nocapture &&
  cargo clippy --config "patch.\"https://github.com/doyouhaobaby/iced-code-editor.git\".iced-code-editor.path=\"$EDITOR_FIX\"" --all-targets --all-features -- -D warnings
'
```

必要时先用同一临时 config 执行 `cargo update -p iced-code-editor`，核对输出确实选用了 path 包。本轮发现旧本地0.5.3包会被忽略，故未将那次运行计为通过。临时覆盖会影响 Cargo.lock；结束后只恢复编辑器的远端 source，不覆盖共享锁文件。

本轮本地覆盖：应用6/0、底层17/0，两仓整仓Clippy均exit0。最终正式v0.5.4默认依赖应用6/0、整仓Clippy exit0；该聚焦测试未检测出实机中文缺字，故保留原生回归要求。底层新增像素用例初始0/1失败；浮层两用例初始0/2失败。完整日志在执行环境的 `/tmp/editor-config-*`，关键结果保存在 validation.txt。底层测试必须在编辑器仓库运行；ShiJu检查必须经cargo_slot入口。

## 实际窗口步骤

1. 构建 `editor-config-window` 后运行本轮产物。使用隔离的应用数据目录；验收宿主在内存提供70行Rust文档（中文函数名、25次中文长行、Tab、嵌套括号、颜色字面量），复用真实预览组件与Editor/Preview消息处理。主程序隔离网关读取失败截图仅记录排查，不算编辑器通过。
2. 点击 Light / Dark 切换，齿轮打开当前文件设置；滚动面板时先确认开关标签位置。逐个点击8项显示开关关闭、开启，检查行号/换行、空白符号/参考线、色块/括号颜色随之改变。关闭换行时列宽输入禁用。列宽输入20、80及清空，观察中文长行实际重排。
3. 滚到编辑区选项，分别切换折叠、自动缩进、自动括号、Vim，再恢复。折叠第一行时正文收起至最后一行。关闭自动缩进与括号后，第二行末Enter再输入左括号，仅得到无缩进的单个左括号；开启时Enter保留Tab、输入左括号得到配对。Vim开关显示NORMAL，按i显示INSERT，Escape返回；关闭后恢复普通编辑。
4. 缩进选择器实际展开菜单并选择2 spaces、4 spaces、Tab；在首行Home后按Tab，观察两点、四点或箭头，分别撤销。复制六个中文字符，在括号内粘贴后实际看到重复文本，再撤销。截图中的零散测试字符来自该过程。
5. 滚动至中部，中文函数标题固定，点击标题返回顶部；中文长行、主题切换、弹层滚动与正文编辑连续运行，未复现v0.5.4的物理字形坐标溢出。恢复默认选项后结束。

## 截图覆盖

| 场景 | 证据 |
| --- | --- |
| 正式0.5.4中文缺字与崩溃 | v054-light-initial.jpg、v054-dark-initial.jpg、v054-runtime-crash.txt |
| 中文恢复、明暗显示开关往返 | accepted-light-display-off.jpg、accepted-light-display-on.jpg、accepted-dark-on.jpg、accepted-dark-display-off.jpg |
| 明暗编辑开关与Vim | accepted-light-edit-off-vim-on.jpg、accepted-light-edit-restored.jpg、accepted-dark-edit-off-vim-on.jpg、accepted-dark-vim-normal.jpg、accepted-dark-vim-insert.jpg |
| 列宽20/80、菜单选择 | final-light-width20.jpg、final-light-width80.jpg、final-light-indent-menu.jpg、final-light-indent2.jpg、final-light-indent-tab.jpg、accepted-dark-width20.jpg |
| 正文真实编辑、中文复制粘贴 | accepted-light-input.jpg、accepted-light-copy-paste.jpg |
| 默认4空格、2空格、Tab实际插入 | accepted-light-tab-insert.jpg、accepted-dark-spaces2.jpg、accepted-dark-tab.jpg |
| 自动缩进、括号开关对照 | accepted-light-autoindent.jpg、accepted-light-auto-brackets.jpg、accepted-dark-auto-off.jpg |
| 折叠与sticky点击 | accepted-light-folded.jpg、accepted-light-sticky.jpg、accepted-dark-sticky.jpg、accepted-dark-sticky-return.jpg |

`fixed-*` 是字体修复后的中间控件观察，仅作补充。`final-light-*` 仅证明控件/列宽；正文操作以补齐Preview转发后的 `accepted-*` 为准。截图由原生窗口直接取得，JPEG原始字节，无拼接或修饰。设置与编辑器颜色分别来自Light/Dark实际主题。

## 交付边界

维护者仍需提供包含底层840ffb6的远端revision/tag。收到后更新正式依赖、重跑默认6项聚焦测试与两仓Clippy，并按以上步骤回归窗口；此前保留任务6，不删除或声称正式中文验收通过。
