# 任务 6: 平台基础与Cookie适配

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 6】 依赖：批次 5（任务 5）。线性顺序为任务 6；本批次仅此任务，不并行。

状态：未开始

目的：平台基础与Cookie适配，保持现有行为并降低职责耦合。

来源任务：用户本次 vw-webview 底层拆分要求。

预计会话范围：仅处理本卡列出的职责及直接接线；前置任务已移走的符号从新库引用，不重新实现。

## 文件

- 新建：`crates/libs/webview-platform/Cargo.toml`、`crates/libs/webview-platform/src/lib.rs` 及下述职责模块（已由前置任务建立的文件只修改）。
- 源文件（均相对 crates/vw-webview/src）：main.rs；执行前阅读原实现与调用者。
- 修改：根 `Cargo.toml`、直接消费者清单和对应源引用；目标 `crates/libs/webview-platform` 内相应职责模块。
- 测试源码：随所属职责迁移现有独立 `*_tests.rs`；混合测试文件按测试函数拆出，保留断言及 fixture，禁止运行。

## 上下文

实际相关符号：webview_data_dir、fallback_webview_data_dir、fallback_webview_data_dir_from、append_log、show_error_dialog、is_trusted_shiju_app_target、read_shiju_app_set_cookie_for_target、runtime_cookie、runtime_cookie_headers。以当前源码为准核对可见性、cfg 与调用方，不依赖 re-export 掩盖循环依赖。

## 步骤

建立 storage.rs、logging.rs、dialog.rs、runtime_cookie.rs；移动目录和错误提示以及受信任 Cookie 输入/解析，保留读取上限及属性校验。只添加当前模块使用的目标依赖；把相关已有测试移到独立文件。纯核心不获得 Cookie 或 GUI 依赖。

## 静态审查计划

本任务不使用失败测试/通过测试流程：仅做行为不变的迁移，且用户明确禁止运行测试及 Cargo；不得声称测试通过。按公共约定仅阅读和迁移测试源码。

- 在仓库根执行 `rg -n 'crate::|super::|include_str!|#\[path' crates/vw-webview/src crates/libs`，重点核对本卡移动模块的路径与测试挂载，不要求所有 crate:: 引用消失。
- 执行 `git diff --check`、`git diff --stat`，阅读本任务完整差异，逐个比对移动前后函数体和调用接线。
- 本卡所需证据：平台库依赖 core 和对应目标依赖；原始 Cookie 不进入日志，读 stdin 的先后顺序不变；记录实际新增/删除路径、符号归属、清单依赖及差异审查结果。
- 编译、单元测试、实机验证均记为未验证，留给用户未来统一验收；代码及上述静态审查完成后按公共收尾规则提交并删除本卡。
