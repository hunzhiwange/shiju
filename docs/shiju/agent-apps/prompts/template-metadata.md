# Pi 模板 frontmatter 契约

来源：固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `src/lib/piPromptTemplate.ts` 与 `tests/lib/piPromptTemplate.test.ts`，读取仓库 `upstream` 快照。消费任务 1 的来源覆盖契约及现有 `prompts` 模块，不涉及文件路径、主机授权或 MCode 事务。

`prompts::template_metadata` 导出 get/strip/set description 和 summary 四个纯函数。只识别开头及闭合的 `---`，处理 LF、CRLF、裸 CR，保留其他字段、正文及 `$1/$2/$@/$ARGUMENTS`；不执行模板替换。字段解析沿用来源的局部规则，不将其扩展为完整 YAML 解析。无闭合 frontmatter 时 get/summary 无元数据，strip 原样返回，set 在原文前添加新元数据块。空 description 删除该字段，仅剩空白时移除整个块。

summary 的 description 压缩空白，以 UTF-16 代码单元计数，超过 140 时取前 139 单元并加省略号；argument-hint 不截断。Rust `String` 不能表示 JS 截断产生的孤立代理项，因此该位置用 U+FFFD 表示；其余中文和完整 emoji 按原 UTF-16 边界计数。该差异显式限定于摘要显示，不改原始 description 或正文。

本任务仅交付业务纯函数；API、SDK、UI 由后续任务接入，无新增端点或权限。按本轮最新要求未执行 Cargo、单元测试、实机测试或其他测试，未领取构建槽位，未启动最终回归；实现未经运行验证。后续统一验证应覆盖引号、多行、空值、未闭合块、CRLF、未知字段、变量及 UTF-16 中文/emoji 截断边界。
