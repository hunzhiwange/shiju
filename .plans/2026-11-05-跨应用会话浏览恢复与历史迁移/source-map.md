# 来源与需求映射

此文件是规划索引，当前不宣称能力已交付。源提交 ca58c0313360eae7c851fbd7553c1614c7af03e7；来源路径相对于 `/Users/xiangmin/worker/code/cc-switch/`。任务1将函数级核对与实际实现/证据写入正式 coverage.md，不改只读 plan.md 或母来源快照。

## 母需求覆盖

|母需求|子任务|
|---|---|
|发现、搜索分页、详情与内容|6, 9, 30, 61, 62, 63, 64, 65|
|13适配器、Pi上下文、全部来源格式|6–23；WSL仅任务1保留与能力边界登记|
|受控恢复与安全删除|32–46；各终端任务及对应API/UI|
|Codex统一、幂等、回滚、切换可见性与压缩|54, 55, 56, 57, 58, 59, 60, 84|
|识矩/ACP身份隔离、非跨引擎续聊|2, 31, 46|
|API权限、SDK、Iced、取消与真实macOS证据|全部API任务、桌面任务及最后三个验收任务|

## 主责14的全部来源文件

|来源文件|子任务编号|
|---|---|
|`docs/user-manual/en/3-extensions/3.4-sessions.md`|1, 83, 84, 85|
|`docs/user-manual/ja/3-extensions/3.4-sessions.md`|1, 83, 84, 85|
|`docs/user-manual/zh/3-extensions/3.4-sessions.md`|1, 83, 84, 85|
|`src-tauri/src/codex_history_migration.rs`|54, 55, 56, 57, 58, 59|
|`src-tauri/src/codex_rollout_file.rs`|4, 54|
|`src-tauri/src/codex_state_db.rs`|5|
|`src-tauri/src/commands/session_manager.rs`|61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 72|
|`src-tauri/src/services/codex_session_compression.rs`|60, 70, 82|
|`src-tauri/src/session_manager/cache.rs`|28, 29|
|`src-tauri/src/session_manager/content.rs`|24, 25, 26, 27|
|`src-tauri/src/session_manager/hermes_wsl.rs`|1（来源保留；Windows/WSL不验收）|
|`src-tauri/src/session_manager/hermes_wsl_reader.py`|1（来源保留；Windows/WSL不验收）|
|`src-tauri/src/session_manager/mod.rs`|30, 31, 32–45|
|`src-tauri/src/session_manager/model.rs`|2|
|`src-tauri/src/session_manager/providers/agy.rs`|23|
|`src-tauri/src/session_manager/providers/blocks.rs`|3|
|`src-tauri/src/session_manager/providers/claude.rs`|6, 7, 32|
|`src-tauri/src/session_manager/providers/codex.rs`|8, 9, 33|
|`src-tauri/src/session_manager/providers/codex_items.rs`|8, 9, 33|
|`src-tauri/src/session_manager/providers/dsh.rs`|22|
|`src-tauri/src/session_manager/providers/gemini.rs`|16, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41|
|`src-tauri/src/session_manager/providers/grokbuild.rs`|18, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41|
|`src-tauri/src/session_manager/providers/hermes.rs`|12, 13, 41, 43|
|`src-tauri/src/session_manager/providers/kimi.rs`|19, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41|
|`src-tauri/src/session_manager/providers/mcode.rs`|21|
|`src-tauri/src/session_manager/providers/mod.rs`|1, 30|
|`src-tauri/src/session_manager/providers/omp.rs`|20, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41|
|`src-tauri/src/session_manager/providers/openclaw.rs`|17, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41|
|`src-tauri/src/session_manager/providers/opencode.rs`|10, 11, 40, 42|
|`src-tauri/src/session_manager/providers/opencode_blocks.rs`|10, 11, 40, 42|
|`src-tauri/src/session_manager/providers/pi.rs`|14, 15, 37|
|`src-tauri/src/session_manager/providers/pi_blocks.rs`|14, 15, 37|
|`src-tauri/src/session_manager/providers/utils.rs`|3|
|`src-tauri/src/session_manager/terminal/mod.rs`|47, 48, 49, 50, 51, 52, 53|
|`src/components/sessions/SessionDialogs.tsx`|77|
|`src/components/sessions/SessionItem.tsx`|73, 77, 78, 79, 80|
|`src/components/sessions/SessionManagerPage.tsx`|73, 77, 78, 79, 80|
|`src/components/sessions/reader/PathChip.tsx`|78, 79, 80|
|`src/components/sessions/reader/SessionAgentAvatar.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionCodeBlock.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionDiff.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionEventRow.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionFinalReply.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionImage.tsx`|27, 76|
|`src/components/sessions/reader/SessionImageLightbox.tsx`|27, 76|
|`src/components/sessions/reader/SessionMarkdown.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionNoteRow.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionOutline.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionQuestion.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionReader.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionReaderHeader.tsx`|78, 79, 80|
|`src/components/sessions/reader/SessionStep.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionStepDetails.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionThinkingRow.tsx`|29, 74, 75|
|`src/components/sessions/reader/SessionTimeline.tsx`|29, 74, 75|
|`src/components/sessions/reader/agentStyles.ts`|29, 74, 75|
|`src/components/sessions/reader/context.ts`|29, 74, 75|
|`src/components/sessions/reader/exportMarkdown.ts`|71, 72, 78, 79, 80|
|`src/components/sessions/reader/i18n.ts`|29, 74, 75|
|`src/components/sessions/reader/rowStyles.tsx`|29, 74, 75|
|`src/components/sessions/reader/toolSummary.ts`|29, 75, 76|
|`src/components/sessions/reader/turns.ts`|29, 75, 76|
|`src/components/sessions/utils.ts`|30, 73|
|`src/hooks/useSessionSearch.ts`|30, 73|
|`src/lib/api/sessions.ts`|61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 72|
|`src/lib/query/sessions.ts`|61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 72|
|`tests/components/sessions/reader/agentStyles.test.ts`|29, 74, 75|
|`tests/components/sessions/reader/helpers.ts`|29, 74, 75|
|`tests/components/sessions/reader/stepCost.test.tsx`|29, 75, 76|
|`tests/components/sessions/reader/toolSummary.test.ts`|29, 75, 76|
|`tests/components/sessions/reader/turns.test.ts`|29, 75, 76|
|`tests/fixtures/sessions/agy.jsonl`|1, 29, 83|
|`tests/fixtures/sessions/claude.messages.json`|1, 29, 83|
|`tests/fixtures/sessions/codex.messages.json`|1, 29, 83|
|`tests/fixtures/sessions/gemini.messages.json`|1, 29, 83|
|`tests/fixtures/sessions/generic.messages.json`|1, 29, 83|
|`tests/fixtures/sessions/opencode.messages.json`|1, 29, 83|
|`tests/fixtures/sessions/pi.messages.json`|1, 29, 83|
|`tests/lib/sessionsApi.test.ts`|61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 72|
|`tests/lib/sessionsQuery.test.tsx`|61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 72|

## 交叉来源与词法候选

- `commands/settings.rs`：历史统一/迁移现有历史选项、备份探测、恢复、压缩开关、磁盘统计由维护领域任务拥有；母任务4仅提供配置事务，20仅复用系统设置导航。
- `commands/pi.rs::get_pi_session_discovery`：Pi上下文任务拥有会话发现，母任务15拥有其他Pi工作区设置。
- `commands/session_manager.rs` 的十个真实命令：对应API任务全部覆盖；`export_session_markdown` 的生成由Markdown任务负责，文件选择/保存由桌面actions负责。不能直接开放任意Tauri命令分发。
- `services/provider` 中供应商切换/AGY凭据路径：模板迁移/维护入口及受控恢复对接母任务4/6，凭据不进入SDK。
- `lib.rs` 命令注册与 `src/types.ts` 类型：任务1交叉核对注册/serde；不因未分到主责14而漏掉类型合同。
- `session_manager/providers/claude.rs:2762` 的 `Exit code 101` 是测试文本词法候选，任务1核对实际代码，不能把它当运行事件API。
- 九种 macOS 内置终端与 custom 都有任务；非macOS实现原样保留来源，不要求Windows/Linux/WSL运行证据。
- 源用户手册可能滞后（例如只列9应用、注释写10个而数组有13个）；以实际适配器/注册/测试核对，不用手册计数削减范围。
