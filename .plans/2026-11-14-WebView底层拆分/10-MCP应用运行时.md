# 任务 10: MCP应用运行时

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 10】 依赖：批次 9（任务 9）。线性顺序为任务 10；本批次仅此任务，不并行。

状态：未开始

目的：MCP应用运行时，保持现有行为并降低职责耦合。

来源任务：用户本次 vw-webview 底层拆分要求。

预计会话范围：仅处理本卡列出的职责及直接接线；前置任务已移走的符号从新库引用，不重新实现。

## 文件

- 新建：`crates/libs/webview-mcp-runtime/Cargo.toml`、`crates/libs/webview-mcp-runtime/src/lib.rs` 及下述职责模块（已由前置任务建立的文件只修改）。
- 源文件（均相对 crates/vw-webview/src）：mcp_app.rs、mcp_app_form.rs、mcp_app_form/、mcp_app_tests.rs；执行前阅读原实现与调用者。
- 修改：根 `Cargo.toml`、直接消费者清单和对应源引用；目标 `crates/libs/webview-mcp-runtime` 内相应职责模块。
- 测试源码：随所属职责迁移现有独立 `*_tests.rs`；混合测试文件按测试函数拆出，保留断言及 fixture，禁止运行。

## 上下文

实际相关符号：run、AppEvent、revoke_document、FormPolicy、FormNavigation、form::install。以当前源码为准核对可见性、cfg 与调用方，不依赖 re-export 掩盖循环依赖。

## 步骤

移动事件循环及表单原生适配到独立库，引用策略库和平台库；FormPolicy 与 revoke_document 留在同库以消除循环依赖。修正 macos 表单实现对 APP_URL/WRAPPER_URL 的访问，迁移动态表单测试。保持 active/submitted/pending 状态、请求原文及提交确认顺序。

## 静态审查计划

本任务不使用失败测试/通过测试流程：仅做行为不变的迁移，且用户明确禁止运行测试及 Cargo；不得声称测试通过。按公共约定仅阅读和迁移测试源码。

- 在仓库根执行 `rg -n 'crate::|super::|include_str!|#\[path' crates/vw-webview/src crates/libs`，重点核对本卡移动模块的路径与测试挂载，不要求所有 crate:: 引用消失。
- 执行 `git diff --check`、`git diff --stat`，阅读本任务完整差异，逐个比对移动前后函数体和调用接线。
- 本卡所需证据：依赖 policy/platform 及实际 wry/tao 目标依赖；断开对原二进制 crate::mcp_app 的引用；记录实际新增/删除路径、符号归属、清单依赖及差异审查结果。
- 编译、单元测试、实机验证均记为未验证，留给用户未来统一验收；代码及上述静态审查完成后按公共收尾规则提交并删除本卡。
