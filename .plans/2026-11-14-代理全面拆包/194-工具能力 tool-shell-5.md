# 任务 194: 工具能力 tool-shell-5

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 194】 依赖：本计划批次193；线性顺序为前置计划1—22、本计划23至最后一项，无并行。

状态：未开始

目的：迁移，将本任务职责归入 `tool-shell`。

来源任务：用户要求尽可能全部拆分vw-agent；覆盖映射见coverage.md。

预计会话范围：一个领域的一组源码迁移；大领域已拆成连续任务，同包分批接线。测试源码和资源仅随实现迁移，不执行。

## 文件

- 目标：`crates/libs/tool-shell/Cargo.toml`、`crates/libs/tool-shell/src/lib.rs` 及保持原职责布局的子模块；包已存在则修改。
- 公共接线：根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`、`crates/vw-agent/src/lib.rs`、相关原mod.rs与搜索确认的直接调用者清单。
- 来源及相邻测试/资源：
- `crates/vw-agent/src/tools/shell/security/injection.rs`
- `crates/vw-agent/src/tools/shell/security/injection_tests.rs`
- `crates/vw-agent/src/tools/shell/security/mod.rs`
- `crates/vw-agent/src/tools/shell/security/mod_tests.rs`
- `crates/vw-agent/src/tools/shell/security/obfuscation.rs`
- `crates/vw-agent/src/tools/shell/security/obfuscation_tests.rs`
- `crates/vw-agent/src/tools/shell/security/tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/backslash.rs`
- `crates/vw-agent/src/tools/shell/security/validators/backslash_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/brace_expansion.rs`
- `crates/vw-agent/src/tools/shell/security/validators/brace_expansion_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/comment_quote_desync.rs`
- `crates/vw-agent/src/tools/shell/security/validators/comment_quote_desync_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/control_chars.rs`
- `crates/vw-agent/src/tools/shell/security/validators/control_chars_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/dangerous_vars.rs`
- `crates/vw-agent/src/tools/shell/security/validators/dangerous_vars_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/empty_command.rs`
- `crates/vw-agent/src/tools/shell/security/validators/empty_command_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/git_commit.rs`
- `crates/vw-agent/src/tools/shell/security/validators/git_commit_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/hash_comment.rs`
- `crates/vw-agent/src/tools/shell/security/validators/hash_comment_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/heredoc.rs`
- `crates/vw-agent/src/tools/shell/security/validators/heredoc_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/ifs_injection.rs`
- `crates/vw-agent/src/tools/shell/security/validators/ifs_injection_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/jq.rs`
- `crates/vw-agent/src/tools/shell/security/validators/jq_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/malformed.rs`
- `crates/vw-agent/src/tools/shell/security/validators/malformed_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/metacharacters.rs`
- `crates/vw-agent/src/tools/shell/security/validators/metacharacters_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/mod.rs`
- `crates/vw-agent/src/tools/shell/security/validators/newline_injection.rs`
- `crates/vw-agent/src/tools/shell/security/validators/newline_injection_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/obfuscated_flags.rs`
- `crates/vw-agent/src/tools/shell/security/validators/obfuscated_flags_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/proc_environ.rs`
- `crates/vw-agent/src/tools/shell/security/validators/proc_environ_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/quoted_newline.rs`
- `crates/vw-agent/src/tools/shell/security/validators/quoted_newline_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/redirection.rs`
- `crates/vw-agent/src/tools/shell/security/validators/redirection_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/substitution.rs`
- `crates/vw-agent/src/tools/shell/security/validators/substitution_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/unicode_whitespace.rs`
- `crates/vw-agent/src/tools/shell/security/validators/unicode_whitespace_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/zsh_dangerous.rs`
- `crates/vw-agent/src/tools/shell/security/validators/zsh_dangerous_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators_tests.rs`
- `crates/vw-agent/src/tools/shell/sed/mod.rs`
- `crates/vw-agent/src/tools/shell/sed/parser.rs`
- `crates/vw-agent/src/tools/shell/sed/parser_tests.rs`
- `crates/vw-agent/src/tools/shell/sed/validation.rs`
- `crates/vw-agent/src/tools/shell/sed/validation_tests.rs`
- `crates/vw-agent/src/tools/shell/session_tools.rs`
- `crates/vw-agent/src/tools/shell/session_tools_tests.rs`
- `crates/vw-agent/src/tools/shell/shell-cn.txt`
- `crates/vw-agent/src/tools/shell/shell.txt`
- `crates/vw-agent/src/tools/shell/termination.rs`
- `crates/vw-agent/src/tools/shell/termination_tests.rs`
- `crates/vw-agent/src/tools/shell/termination_windows.rs`
- `crates/vw-agent/src/tools/shell/termination_windows_tests.rs`
- `crates/vw-agent/src/tools/shell/tests.rs`
- `crates/vw-agent/src/tools/shell/timeout-audit.md`

## 上下文

- 重点符号：`injection_findings`、`has_blocking_injection`、`Severity`、`SecurityCategory`、`SecurityFinding`、`SecurityReport`、`SecurityValidator`、`SecurityPipeline`、`obfuscation_findings`、`has_blocking_obfuscation`、`BackslashValidator`、`BraceExpansionValidator`、`CommentQuoteDesyncValidator`、`ControlCharsValidator`、`DangerousVarsValidator`、`EmptyCommandValidator`、`GitCommitValidator`、`HashCommentValidator`。
- 当前文本引用候选：`tools::shell`、`security`、`tools::context`、`tools::traits`、`session::ui_store`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
- 领域边界：具体工具按能力分包，工厂由 tool-assembly 组合；涉及会话、插件与调度器的工具只依赖前置端口，不反向依赖会话或主循环。

## 步骤

1. 读取上述源码、父模块声明、相邻测试和调用者；用 `rg -n 'crate::|super::|pub\(crate\)|include!' crates/vw-agent/src` 限定到本任务查看实际依赖，记录旧路径到目标模块映射。测试文件单独排程时按其生产实现实际所有者迁入，不为测试再建包。
2. 按领域边界迁移本任务源码；剥离应用宿主动作到已存在的窄接口，更新资源路径与必要可见性。禁止新增未调用抽象，不合并行为不同的旧实现。
3. 建立/扩展目标最小清单，复用已拆出的真实所有者；完整转发原feature/cfg。调用者正向接线，旧位置最多薄重导出。后续任务尚未迁的调用者通过真实宿主适配接线，不让新包依赖它。
4. 逐项审阅参数/错误/异步取消、租户、权限、全局状态、序列化与平台语义等价；仅调整测试源码导入，不运行或追加测试。将尚需闭环迁移的宿主适配具体路径写入本任务提交正文。
5. 按公共规则记录当前静态证据、提交代码并删除本编号文件；闭环收口任务须同时消除前期业务残留。

## 静态验收

- 运行：`git diff --check`；逐文件阅读本任务完整差异和迁移后的清单。
- 运行：`rg -n 'vw-agent|vw_agent|crate::app' crates/libs/tool-shell`；审阅是否存在库对主包的反向依赖，文档与测试引用单独检查。
- 检查：本任务全部来源已迁/明确为组合转发，生产实现不重复；类型/状态只有一个所有者；工厂注册和资源完整；后续文件已分配所有者。
- 所需证据：真实迁移映射、最小依赖、窄端口调用者、cfg/feature、直接调用接线、测试/资源迁移清单及差异检查结果。仅静态证据，不能声称编译或行为通过。
- 待统一验收：本任务来源的既有测试、直接调用集成、原支持平台与特性、涉及UI的渲染和外部能力；将具体路径及组合写提交正文，本轮全部禁止运行。
