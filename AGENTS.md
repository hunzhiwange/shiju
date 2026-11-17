# AGENTS.md

ShiJu 是 Rust 优先的自主代理运行时。本文件适用于整个仓库。

## 开发约定

- 先读现有实现、调用者与相邻测试，沿用已有模式，只改当前任务相关内容；修复应用层问题前先排查底层原因。
- 代码导航优先使用 `./skills/rustcodegraph/SKILL.md`；未索引、文档/配置或图未覆盖时使用 `rg`。
- 保持控制流与错误明确，不做伪支持；不为未来需求添加抽象、配置、开关或重型依赖。
- 不削弱安全策略、扩大权限或记录密钥、令牌和敏感载荷；未经要求不做重品牌或身份替换。
- 工厂注册 key 保持稳定、小写；单元测试放独立文件，不与业务逻辑混放。
- 未明确要求，不切换分支或创建 Git worktree；不做无关重构和大规模格式化。

## 验证与任务计划

- 未明确要求，不执行测试、不编译桌面客户端。纯文档修改只做轻量检查，不调用 Cargo。
- 要求验证时，只检查受影响包和目标，测试使用有效过滤条件并确认实际测试数；同一目标已通过编译时不重复 check。
- Rust 重型编译串行执行，复用 `scripts/cargo_slot.py` 槽位缓存；不默认运行全量测试、workspace 构建、全特性 Clippy、全仓格式检查、wasm 检查或 Makefile 命令。格式化仅限修改文件。
- `.plans` Rust 任务使用槽位入口；旧裸 Cargo 命令的等价调整只记录到编号任务文件，不改 `plan.md` 和 `prompt.md`。运行产物、系统授权、物理音频与发布验证沿用原生路径。
- 修复本次引入的错误；既有无关失败只记录。如实说明失败或未运行的检查，不伪报通过。

## Iced UI

- 适配明暗主题，容器按布局要求居中。滚动条宽 4px，内容预留宽度与间隔，参考系统设置两侧 7px 留白。
- 修改表单前参考 `crates/vw-desktop/src/app/components/system_settings_common/` 和 `system_settings_providers/custom_model_modal.rs`，复用统一控件与样式。
- 输入框使用 `settings_text_input_style`、`SETTINGS_CONTROL_PADDING`、`SETTINGS_CONTROL_TEXT_SIZE`；其他表单控件使用对应 `settings_*_style`。
- 配置表单复用 `settings_modal_card`、`settings_modal_overlay`、`settings_close_button`，长表单只滚动内容区，固定标题和操作区。
- 主次及辅助按钮使用 `primary_action_btn_style` / `rounded_action_btn_style`，显式设置内边距与圆角；反馈使用 `toast.rs`。
- 非 ASCII 文本禁止使用 `Font::MONOSPACE`，可能导致字形缓存溢出与主线程 panic。等宽路径/代码与中文标签拆成独立 `text` 控件。
- 字体变更须检查实际渲染路径，不能仅凭构建 `Element` 判断安全；运行验证仍遵守上述授权范围。

## 交接

完成后用 50 字以内说明变更和下一步推荐动作。
