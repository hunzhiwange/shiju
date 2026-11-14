# 任务 10: 运行时Cookie

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 10】 依赖：批次 9（任务 9）。线性顺序 10；本批仅此任务，不并行。

状态：未开始

目的：将运行时Cookie完整划为单一职责边界，保持既有行为。

来源任务：用户要求从低依赖开始，尽可能全部拆分。

预计会话范围：只迁移本卡列出的职责与直接接线；不重复搬迁前置任务已移走的实现。

## 文件与上下文

- 新建：`crates/libs/webview-runtime-cookie/Cargo.toml`、`crates/libs/webview-runtime-cookie/src/lib.rs` 及职责模块。
- 源：`crates/vw-webview/src/` 下的 main.rs、main_tests.rs；非 src 文件按仓库根解析。
- 修改：根 `Cargo.toml`、实际消费者清单及调用源文件，目标库职责模块。
- 相关符号：is_trusted_shiju_app_target、read_shiju_app_set_cookie_for_target、runtime_cookie、runtime_cookie_headers。
- 依赖边界：url、cookie、http 的现有匹配版本；不得反向依赖运行时。新库只保留实际使用的直接依赖；目标依赖沿用原 cfg/features。
- 测试源码随职责迁移至独立 `*_tests.rs`，混合文件按函数分拆，保留现有断言和 fixture，不运行。

## 实施步骤

独立受信任 Cookie 输入和解析库；保留输入上限、域和 HTTPS 限制、Cookie 属性及错误。按锁文件选择与 wry 重导出相同 cookie/http 版本，避免为类型重导出依赖整个 wry。迁移相关测试源码。

先搜索真实调用者及私有字段访问，调整最小可见性；移除原实现并同步调用接线，不用复制或反向依赖留尾巴。

## 静态审查与完成证据

按用户要求不执行失败/通过测试流程，不运行 Cargo、单元测试或实机验证；这是结构迁移，不新增业务行为。

- 仓库根执行 `rg -n 'crate::|super::|include_str!|#\[path' crates/vw-webview/src crates/libs`，人工核对本卡移动路径、模块挂载及资源路径，不要求所有搜索结果消失。
- 执行 `git diff --check`、`git diff --stat` 并逐项阅读实际差异；记录函数体保留、测试归属、跨库调用和直接依赖清单。
- 成功证据：本卡职责已从原包移走、所有真实调用已接线、无双向包引用，既有 cfg、安全拒绝、错误和生命周期有逐项对照记录。
- 编译、测试和实机均标未验证。按公共收尾规则提交代码后删除本卡，不能宣称动态验证通过。
