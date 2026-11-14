# Pi 模板 slug 校验契约

任务 15，2026-11-14。前置来源契约为同目录 coverage.md；固定来源提交为 ca58c0313360eae7c851fbd7553c1614c7af03e7。

实现入口：vw_agent_apps::prompts::template_slug::validate_template_slug(&str) -> Result<(), String>。沿用包内显式字符串错误模式，无 Tauri 依赖，不读取文件、不执行命令、不授予主机权限。

来源：快照 upstream/src-tauri/src/services/pi_prompt_files.rs 的 validate_template_slug 与 MAX_TEMPLATE_SLUG_BYTES；前端 src/lib/piPromptSlug.ts 及 tests/lib/piPromptSlug.test.ts 作为契约对照。业务行为以 Rust 来源为准：不 trim、不改名、不做 Unicode 归一化。

- 非空，UTF-8 长度最多 128 字节；ASCII 128 字节允许、129 拒绝；“评”42 个允许、43 个拒绝。
- 允许 review-pr、release.v2、评审、SYSTEM、内部连续点号；拒绝点前缀和点后缀（包含单点、双点）。
- 拒绝 Rust char::is_control / char::is_whitespace 命中的字符及 `< > : " / \ | ? *`。
- 第一个点之前的 basename 忽略 ASCII 大小写，拒绝 con、prn、aux、nul、com1–com9、lpt1–lpt9，包含带扩展名情况；com0、com10 等不在保留集合。
- Rust 与 JavaScript 的 Unicode 空白集合有细微差别，沿用 Rust 来源而不扩大或缩减该集合。

API、SDK、UI 接入由后续任务 25、28、33 承担；本任务仅交付可复用业务校验入口，不新增路由或表单。

按本轮最新要求仅写代码与必需契约资料，未新增或运行测试，未执行任何 Cargo 操作、实机验收或最终回归。未领取 Cargo 槽位、未设置构建缓存路径。以上示例是来源规则说明，不代表验证通过；后续统一验证需覆盖 Unicode、点号、128 字节、空白、控制字符、斜杠及设备名边界。
