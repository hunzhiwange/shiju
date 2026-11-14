# 提示词、规则与模板来源覆盖契约

记录日期：2026-11-14（Asia/Shanghai）。本文件为子任务 1 的必需契约资料；本轮仅阅读源码、来源索引及任务卡，不运行 Cargo、单元测试、实机测试或其他测试，不启动最终回归。以下测试路径均为后续验证位置，未执行，不代表通过；macOS 应用版本、原生调用和渲染结果均未验证。

## 来源基线与使用方式

固定来源为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，源仓 `/Users/xiangmin/worker/code/cc-switch` 的 `git rev-parse HEAD` 输出一致。母索引为 `.plans/2026-11-05-CC能力完整融合/来源清单.json`；来源快照由 `crates/vw-agent-apps/来源文件.json` 管理。本文所有来源相对路径默认相对于该源仓，已收录文件优先读 `crates/vw-agent-apps/upstream/`，不调用其中的 Tauri 服务。

本次读取索引主责 13 的 34 个文件和 13 个命令：34 个源文件的 SHA256 均与母索引一致；其中 31 个已收录快照的文件字节一致；3 份 `docs/user-manual/*/3-extensions/3.2-prompts.md` 不在快照中，直接读取固定提交源仓。未发现本项来源漂移。该事实仅覆盖本文列出的来源，不声明整棵来源树通过验收。

子任务编号指 `.plans/2026-11-05-提示词规则与模板管理/`。旧 `来源映射.md` 引用的 39、40 不存在于当前 38 项调度：自动场景由下列各实现/API/SDK/UI所有者承担，macOS 原生实测由母计划 `人工验收.md` 交给用户。本文不修改只读 plan.md、prompt.md 或原始来源索引，不创建替代任务卡。

## 分类与唯一边界

| 分类 | 所有者与限制 |
| --- | --- |
| 识矩智能体系统提示词 | 既有智能体配置所有者；本领域只分类/导航，不把外部文件拼入识矩系统级指令 |
| 项目规则 | 既有项目/工作区配置所有者；本文全局应用目标不推导任意项目路径，不新增任意文件接口 |
| 外部应用全局指令/人设 | 子任务 3–13、19–20；按应用、执行主机、租户和配置目录隔离，Hermes 的 SOUL.md 是人设 |
| Pi 原生系统文件 | 子任务 14；仅 SYSTEM.md、APPEND_SYSTEM.md，文件存在即配置，不使用提示库 enabled 位 |
| Pi 原生模板 | 子任务 15–18；prompts/<slug>.md，保留原生变量与文件内容，Rust 不执行模板或命令 |
| OpenClaw 工作区文件 | 母任务 15 是永久编辑唯一所有者；本领域保留来源投影/同步边界，不新增第二编辑源 |
| Skills、MCP、账号与主机权限 | 分别由母任务 12、11、账号/执行底座所有；模板不能绕过原生 Skill 挂载或扩大权限 |

## 14 应用支持矩阵

下表为来源能力和默认路径证据，非本轮运行结论。覆盖目录、环境变量、profile 必须由授权执行主机的路径解析器处理；UI 不以自身 HOME 拼路径。普通提示库支持条目 CRUD、受控导入和激活；系统文件/模板能力仅明确授予 Pi。

| AppId | 来源目标或限制 | 分类/特殊约束 | 本领域实现所有者 |
| --- | --- | --- | --- |
| claude | ~/.claude/CLAUDE.md | 全局指令；设置目录覆盖 | 3、7–11、20 |
| claude-desktop | prompt_file_path 明确拒绝 | 来源 UI 转 Claude Code；必须说明并重新选择目标，不隐式别名 | 3、21、29 |
| codex | ~/.codex/AGENTS.md | 全局规则；配置目录解析 | 3、7–11、20 |
| gemini | ~/.gemini/GEMINI.md | 全局规则 | 3、7–11、20 |
| grokbuild | ~/.grok/AGENTS.md | 全局规则 | 3、7–11、20 |
| opencode | ~/.config/opencode/AGENTS.md | 全局规则 | 3、7–11、20 |
| openclaw | get_openclaw_dir()/AGENTS.md | 来源服务支持投影；编辑归母任务 15 | 3、20；编辑/API/UI 不在本领域重复提供 |
| hermes | ~/.hermes/SOUL.md | 人设；跨应用复制默认不选；记忆文件归母任务 15 | 3、5、7–11、20、34 |
| pi | ~/.pi/agent/AGENTS.md | 当前文件为激活真值；另有 SYSTEM.md、APPEND_SYSTEM.md、prompts/*.md | 3、13–18、24–25、28、32–33 |
| mcode | ~/.minimax/AGENTS.md | MINIMAX_DATA_DIR 等执行端路径；UTF-8 最大 32768 字节 | 3、12 |
| dsh | ~/.dsh/AGENTS.md | DSH_HOME 等执行端路径 | 3、7–11、20 |
| kimi | ~/.kimi-code/AGENTS.md | KIMI_CODE_HOME 等执行端路径 | 3、7–11、20 |
| agy | ~/.gemini/config/AGENTS.md | customizations_dir；不能误写 Gemini 的 GEMINI.md 或私有 Key profile | 3、7–11、20 |
| omp | ~/.omp/agent/AGENTS.md；命名 profile 为 ~/.omp/profiles/<name>/agent/AGENTS.md | 源码只证明 agent 目录 AGENTS 路径；不承诺 Pi 系统文件或模板服务 | 3、19、20 |

矩阵依据 `src-tauri/src/prompt_files.rs::prompt_file_path` 及各 `*_config` 路径解析；Pi 另见 `pi_config/mod.rs`，OMP 另见 `omp_config.rs::get_omp_agent_dir/get_omp_native_agent_dir/normalize_profile`。OMP_PROFILE 优先于 PI_PROFILE，目录覆盖与继承 profile 要按来源及授权路径合同处理，不跨写 Pi 目录。来源注释中的“只读”描述原生发现规则，不证明运行时加载过文件。

## 命令到业务、API、SDK、Iced

下表每行仅一个命令业务所有者；特殊应用由对应适配器细化，不重复建立存储。API 是拟新增的显式资源接口，不是 Tauri 命令透传。确切 HTTP 路径由接口任务结合共享协议确定，本任务不臆造已注册端点。

| 来源命令（commands/prompt.rs） | 业务所有者 | API任务 | SDK任务 | Iced任务 |
| --- | --- | --- | --- | --- |
| get_prompts | 6（存储4、首次导入9、Pi13） | 21 | 27 | 29 |
| upsert_prompt | 4（已激活10、停用11、MCode12、Pi13） | 22 | 27 | 30、31 |
| delete_prompt | 11 | 22 | 27 | 31 |
| enable_prompt | 10（MCode12、Pi13） | 23 | 27 | 31 |
| import_prompt_from_file | 8 | 22 | 27 | 34 |
| get_current_prompt_file_content | 7 | 21 | 27 | 29 |
| get_prompt_file_location | 3（安全读取7） | 21 | 27 | 29、34 |
| get_pi_prompt_file | 14 | 24 | 28 | 32 |
| replace_pi_prompt_file | 14 | 24 | 28 | 32 |
| delete_pi_prompt_file | 14 | 24 | 28 | 32 |
| list_pi_prompt_templates | 17 | 25 | 28 | 33 |
| upsert_pi_prompt_template | 17（slug15、元数据16、改名18） | 25 | 28 | 33 |
| delete_pi_prompt_template | 17 | 25 | 28 | 33 |

非 command 入口：`import_from_file_on_first_launch` → 9；`sync_to_live/sync_all_to_live/project_prompt_set_to_path` → 20 → API26 → SDK28 → UI35；跨应用复制 → 5 → API22 → SDK27 → UI34；分享领域导入 → 36 → API/SDK37 → UI38。文件定位/复制正文是客户端动作，不能让远程 Gateway 打开本机文件或向远端剪贴板写入。

## 逐来源文件归属

每行列主要符号或场景；关联任务细分见后文实现与验证位置。所有测试来源仅阅读，未运行。

| 来源相对路径 | 符号/行为 | 唯一主责及消费任务 |
| --- | --- | --- |
| `docs/user-manual/en/3-extensions/3.2-prompts.md` | 英文用户说明、跨设备配置同步 | 1；具体行为4–20、29–35；配置归母18/19 |
| `docs/user-manual/ja/3-extensions/3.2-prompts.md` | 日文用户说明 | 1；同上 |
| `docs/user-manual/zh/3-extensions/3.2-prompts.md` | 中文说明、Pi三类资源、文件覆盖与备份 | 1；同上 |
| `src-tauri/src/commands/prompt.rs` | 13 个显式命令、PromptFileLocation | 2；21–28（逐命令见上表） |
| `src-tauri/src/database/dao/prompts.rs` | get_prompts/save_prompt/save_mcode_prompts/delete_prompt | 4；6、10–13消费 |
| `src-tauri/src/deeplink/prompt.rs` | import_prompt_from_deeplink | 36；37–38 |
| `src-tauri/src/prompt.rs` | Prompt 字段及 enabled/时间 | 2；4消费 |
| `src-tauri/src/prompt_files.rs` | validate_prompt_content/prompt_file_path/display_path | 3；7、12、19消费 |
| `src-tauri/src/services/pi_prompt_files.rs` | PiAgentsFileGuard、PiPromptFileService、PiPromptTemplateService、revision/slug/限长/改名回退 | 13、14、15、17、18各自独占对应行为；24–25消费 |
| `src-tauri/src/services/prompt.rs` | get/upsert/delete/enable/import、首次导入、同步、MCode/Pi 专属 helper | 4、7–14、20按前述行为唯一归属 |
| `src-tauri/tests/prompt_live_sync.rs` | 外部回填、停用副本不写文件、OMP目标目录 | 10–11、19–20；对应独立测试 |
| `src/components/deeplink/PromptConfirmation.tsx` | PromptConfirmation、确认前预览 | 38；36–37供给 |
| `src/components/prompts/PiNativePromptResources.tsx` | PiSystemPromptFiles/PiInstructionFileEditor、PiPromptTemplates/PiPromptTemplateEditor、草稿revision、改名与撤销 | 32（系统）、33（模板）；14–18供给 |
| `src/components/prompts/PiPromptPanel.tsx` | global/system/templates导航、handleToggle/Save/Import/CopyToApps | 29、31–34按页面行为归属 |
| `src/components/prompts/PromptCopyDialog.tsx` | 默认排除同名和Hermes、停用独立副本 | 34；5供给 |
| `src/components/prompts/PromptFormPanel.tsx` | handleSave/handleClose、字段校验、未保存草稿 | 30 |
| `src/components/prompts/PromptLibrary.tsx` | filterPromptEntries、空态、列表渲染 | 6（检索）；29（呈现） |
| `src/components/prompts/PromptListItem.tsx` | 编辑、切换、删除、复制菜单 | 29、31、34按动作归属 |
| `src/components/prompts/PromptPageFrame.tsx` | 页头、搜索、目标说明 | 29 |
| `src/components/prompts/PromptPanel.tsx` | handleAdd/Edit/Save/Delete/Toggle/Import/CopyContent/CopyToApps、外部刷新与撤销 | 29–31、34按动作归属 |
| `src/components/prompts/promptUtils.tsx` | utf8Bytes/formatSize/charCount/firstLine/formatUpdated/renderSegs/copyText/showPromptToast | 12（限长）；29–35（UI）；中文不得照搬等宽样式 |
| `src/components/prompts/usePromptPageCommon.ts` | usePromptPageCommon、toggleEffectText、路径和当前内容 | 29、31、34 |
| `src/hooks/usePromptActions.ts` | reload/savePrompt/deletePrompt/enablePrompt/toggleEnabled/importFromFile/getLatestPrompts；应用/请求代次隔离 | 29–31；7–11提供领域行为 |
| `src/lib/api/prompts.ts` | Prompt/Pi资源类型及promptsApi | 2；21–28各接口所有者 |
| `src/lib/piPromptSlug.ts` | isValidPiPromptTemplateSlug | 15 |
| `src/lib/piPromptTemplate.ts` | get/strip/setPiPromptTemplateDescription、getPiPromptTemplateSummary及frontmatter辅助 | 16 |
| `src/lib/query/prompts.ts` | promptKeys、列表查询缓存与变更失效 | 27–28（请求）；29–35（UI刷新） |
| `tests/components/PiNativePromptResources.test.tsx` | revision草稿、SYSTEM确认、空内容提示、模板改名/描述/slug/撤销 | 32–33 |
| `tests/components/PiPromptPanel.test.tsx` | 标签切换、保存导航锁、外部AGENTS导入与激活反馈 | 29、31–33 |
| `tests/components/PromptFormPanel.test.tsx` | 表单字段和保存行为 | 30 |
| `tests/components/PromptPanel.test.tsx` | 提示库操作、筛选、导入与反馈 | 29–31、34 |
| `tests/hooks/usePromptActions.test.tsx` | 乱序响应、切应用/卸载、写成功后刷新失败不回滚 | 29–31 |
| `tests/lib/piPromptSlug.test.ts` | 便携文件名边界 | 15 |
| `tests/lib/piPromptTemplate.test.ts` | 原生description摘要、分离恢复不丢frontmatter | 16 |

## 特殊规则与交叉项合同

- 排序是 DAO `ORDER BY created_at ASC, id ASC` 的稳定列表顺序；搜索覆盖记录 key、id、名称、说明和正文，trim 后不区分大小写。Pi 模板另按 slug 排序。没有拖拽排序合同，不增加排序字段。
- 普通条目显式导入保存停用副本；首次导入仅在该应用库为空且当前内容非空时触发。来源普通首次导入 enabled=true，Pi 持久化 enabled=false，激活状态由文件推导；任务9不得把二者混同。新建/导入/重存停用项不能清空手写文件。
- 激活先回填已有选择的外部修改或创建去重原始备份，再写新内容并停用其他项。来源普通列表读取会回填非空外部内容；任务21名为只读接口，是否触发该受控回填由6/10的业务合同表达，不能在handler暗加写文件。同步恢复无启用项时保留当前文件；多启用项仅按稳定顺序投影首项并报告异常，不能静默随机选择。
- 停用/删除只影响管理归属内容；外部修改冲突应返回明确错误，不能直接覆盖。来源 UI 阻止删除已启用项；后端任务11也必须保护，不能只靠按钮。Pi 用文件内容匹配选择，原生AGENTS写入使用同一协调锁与revision；持久化失败和文件失败须有恢复路径。
- MCode 32768 是 UTF-8 字节上限；不是字符上限。来源 `upsert_mcode_prompt/enable_mcode_prompt` 与 `save_mcode_prompts` 将回填、启停和原生写入协调；任务12消费真实配置事务，不能以多次独立保存伪装原子提交。
- Pi 系统/模板文件最大 1 MiB；缺失 revision 为 `missing`，已有内容 revision 基于字节。旧revision拒绝写/删；改名检查源revision和目标不存在，写失败回退，回退失败同时报告。系统文件仅两个枚举值，模板仅已验证slug，不能传任意文件路径。
- slug 最多128 UTF-8字节，非空；禁止点路径、前导/尾随点、空白、控制字符、路径分隔符及便携文件名禁用字符/Windows设备保留名。保留名是数据规则，不代表新增Windows验收。
- TS frontmatter转换只处理description和摘要/argument-hint，保留其他frontmatter、正文和换行；不引入完整模板执行引擎。`$1/$2/$@/$ARGUMENTS` 由 Pi 原生展开；保存模板不等于已验证原生调用。已有 Pi 需要按来源提示 `/reload`，本轮未运行。
- 分享 `import_prompt_from_deeplink` 检查resource/app/name/Base64/UTF-8，先创建停用条目，确认允许后才激活。来源enabled字段本身不是授权；任务36–38实现预览/确认，OS协议注册与分发归母任务20。源时间字段有秒/毫秒混用风险，任务2/4/36按各自实际合同显式处理，不凭本盘点假定统一单位。
- 来源 UI 有撤销、保存中导航锁、草稿revision、应用切换/乱序响应隔离。Iced29–35/38保留对应用户结果；写成功后刷新失败不能当作写失败回滚本地成功状态。辅助按钮复用系统设置样式，反馈用toast.rs；暗黑主题、4px滚动条、中文字体的实际渲染留待统一验证。
- 跨设备同步/配置导出导入归母18/19；本领域20只投影提示词并汇总逐目标失败/取消，不重造设备同步。旧服务没有取消command，不据此省略本计划20/26/28/35已要求的取消链路。

## 实现与验证位置（均为后续任务合同）

共享 DTO 由2负责；Gateway负责认证、租户、targetHostId及执行主机授权。业务服务持有存储适配与受限路径，不由Iced持有独立存储。下表路径以仓库根为基准，来源测试用于设计场景；列出的拟新增测试本轮未创建、未执行。目录或文件实际接线可由对应任务核对最新底座后调整，不表述为已存在。

| 子任务 | 主交付位置（拟新增） | 独立验证位置（拟新增，未运行） |
| --- | --- | --- |
| 2-提示词资源共享协议 | `crates/vw-api-types/src/agent_apps/prompts.rs` | `crates/vw-api-types/src/agent_apps/prompts_tests.rs` |
| 3-应用目标路径与能力映射 | `crates/vw-agent-apps/src/prompts/targets.rs` | `crates/vw-agent-apps/src/prompts/targets_tests.rs` |
| 4-提示词条目持久化 | `crates/vw-agent/src/agent_apps/prompts_store.rs` | `crates/vw-agent/src/agent_apps/prompts_store_tests.rs` |
| 5-跨应用复制为停用副本 | `crates/vw-agent-apps/src/prompts/copy.rs` | `crates/vw-agent-apps/src/prompts/copy_tests.rs` |
| 6-稳定排序与检索 | `crates/vw-agent-apps/src/prompts/listing.rs` | `crates/vw-agent-apps/src/prompts/listing_tests.rs` |
| 7-当前文件读取与安全预览 | `crates/vw-agent-apps/src/prompts/live_file.rs` | `crates/vw-agent-apps/src/prompts/live_file_tests.rs` |
| 8-现有文件显式导入 | `crates/vw-agent-apps/src/prompts/import.rs` | `crates/vw-agent-apps/src/prompts/import_tests.rs` |
| 9-首次导入幂等触发 | `crates/vw-agent/src/agent_apps/prompts_bootstrap.rs` | `crates/vw-agent/src/agent_apps/prompts_bootstrap_tests.rs` |
| 10-普通提示词激活与备份 | `crates/vw-agent-apps/src/prompts/activation.rs` | `crates/vw-agent-apps/src/prompts/activation_tests.rs` |
| 11-停用删除与外部修改保护 | `crates/vw-agent-apps/src/prompts/deactivation.rs` | `crates/vw-agent-apps/src/prompts/deactivation_tests.rs` |
| 12-MCode限长与事务写入 | `crates/vw-agent-apps/src/prompts/mcode.rs` | `crates/vw-agent-apps/src/prompts/mcode_tests.rs` |
| 13-Pi指令选择的文件真值 | `crates/vw-agent-apps/src/prompts/pi_agents.rs` | `crates/vw-agent-apps/src/prompts/pi_agents_tests.rs` |
| 14-Pi系统原生文件管理 | `crates/vw-agent-apps/src/prompts/pi_system_files.rs` | `crates/vw-agent-apps/src/prompts/pi_system_files_tests.rs` |
| 15-模板slug的Rust校验 | `crates/vw-agent-apps/src/prompts/template_slug.rs` | `crates/vw-agent-apps/src/prompts/template_slug_tests.rs` |
| 16-模板frontmatter转换 | `crates/vw-agent-apps/src/prompts/template_metadata.rs` | `crates/vw-agent-apps/src/prompts/template_metadata_tests.rs` |
| 17-Pi模板目录增改删 | `crates/vw-agent-apps/src/prompts/templates.rs` | `crates/vw-agent-apps/src/prompts/templates_tests.rs` |
| 18-模板改名与失败恢复 | `crates/vw-agent-apps/src/prompts/template_rename.rs` | `crates/vw-agent-apps/src/prompts/template_rename_tests.rs` |
| 19-OMP配置目录隔离 | `crates/vw-agent-apps/src/prompts/omp.rs` | `crates/vw-agent-apps/src/prompts/omp_tests.rs` |
| 20-同步全部目标与取消 | `crates/vw-agent-apps/src/prompts/sync.rs` | `crates/vw-agent-apps/src/prompts/sync_tests.rs` |
| 21-提示词只读网关接口 | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_read.rs` | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_read_tests.rs` |
| 22-提示词条目写入网关接口 | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_write.rs` | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_write_tests.rs` |
| 23-激活停用网关接口 | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_activation.rs` | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_activation_tests.rs` |
| 24-Pi系统文件网关接口 | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_files.rs` | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_files_tests.rs` |
| 25-Pi模板网关接口 | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_templates.rs` | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_templates_tests.rs` |
| 26-全部同步操作网关接口 | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_sync.rs` | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_sync_tests.rs` |
| 27-提示词SDK条目与激活 | `crates/vw-gateway-client/src/client/agent_app_prompts_api.rs` | `crates/vw-gateway-client/src/client/agent_app_prompts_api_tests.rs` |
| 28-原生文件模板与同步SDK | `crates/vw-gateway-client/src/client/agent_app_prompt_resources_api.rs` | `crates/vw-gateway-client/src/client/agent_app_prompt_resources_api_tests.rs` |
| 29-桌面提示词列表与目标预览 | `crates/vw-desktop/src/app/components/system_settings_prompts/list.rs` | `crates/vw-desktop/src/app/components/system_settings_prompts/list_tests.rs` |
| 30-桌面提示词编辑弹窗 | `crates/vw-desktop/src/app/components/system_settings_prompts/editor.rs` | `crates/vw-desktop/src/app/components/system_settings_prompts/editor_tests.rs` |
| 31-桌面激活停用与删除反馈 | `crates/vw-desktop/src/app/components/system_settings_prompts/activation.rs` | `crates/vw-desktop/src/app/components/system_settings_prompts/activation_tests.rs` |
| 32-桌面Pi系统文件弹窗 | `crates/vw-desktop/src/app/components/system_settings_prompts/pi_files.rs` | `crates/vw-desktop/src/app/components/system_settings_prompts/pi_files_tests.rs` |
| 33-桌面原生模板编辑 | `crates/vw-desktop/src/app/components/system_settings_prompts/templates.rs` | `crates/vw-desktop/src/app/components/system_settings_prompts/templates_tests.rs` |
| 34-桌面复制导入与文件定位 | `crates/vw-desktop/src/app/components/system_settings_prompts/transfer.rs` | `crates/vw-desktop/src/app/components/system_settings_prompts/transfer_tests.rs` |
| 35-桌面全部同步与取消反馈 | `crates/vw-desktop/src/app/components/system_settings_prompts/sync.rs` | `crates/vw-desktop/src/app/components/system_settings_prompts/sync_tests.rs` |
| 36-分享提示词载荷导入 | `crates/vw-agent-apps/src/prompts/shared_import.rs` | `crates/vw-agent-apps/src/prompts/shared_import_tests.rs` |
| 37-分享载荷网关与SDK入口 | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_shared_import.rs` | `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_shared_import_tests.rs`、`crates/vw-gateway-client/src/client/agent_app_prompts_api_tests.rs` |
| 38-桌面分享载荷确认 | `crates/vw-desktop/src/app/components/system_settings_prompts/shared_import.rs` | `crates/vw-desktop/src/app/components/system_settings_prompts/shared_import_tests.rs` |

API21–26/37的验证需覆盖未授权主机/租户、非法应用/路径、revision冲突、缺失与空文件、取消及部分失败；SDK27–28/37覆盖请求编码、状态与错误；Iced29–35/38覆盖上述来源交互、暗黑主题及实际中文渲染。macOS人工验收归母计划 `人工验收.md`，记录应用版本、目标文件、原生reload/模板调用与结果；OMP只能验有证据的AGENTS路径。以上均为待执行位置，不是本轮完成证据。

## 外部前置产物盘点

母任务卡1–7、9–11当前已不在编号目录，不能仅凭删除判定全部验收；8与12母卡仍为未开始。本项无直接依赖，只盘点可见产物，不替它们修改状态。

| 前置所有者 | 本次可见产物/消费位置 | 后续就绪要求 |
| --- | --- | --- |
| 母1 来源 | crates/vw-agent-apps/来源说明.md、来源文件.json、upstream/ | 当前固定提交与本项来源一致；未收录手册从固定源仓读取 |
| 母2 原生包 | crates/vw-agent-apps/src/lib.rs及实际业务模块 | 原生包已存在，纠正旧卡“包不存在”的过时假设；本项没有注册prompts模块 |
| 母3 应用/执行主机 | apps目录、gateway/api/handlers/agent_apps/access.rs、API agent_apps DTO | 任务3/21消费实际AppId和主机授权；不推测通用host trait |
| 母4 路径/事务 | providers/paths.rs、providers/live/write_guard.rs与live目录 | 路径受HOME/无符号链接约束；AppWriteGuard不能单独证明完整恢复事务，任务10–14/17–20核对真实提交/恢复接口 |
| 母5/6/7 模型、账号与配额 | src/models、auth、usage 等领域模块 | 本项不读取密钥或复制凭据；提示词不承担账号/模型业务 |
| 母8/9/10 路由、故障转移、聚合 | src/routing、high_availability、diagnostics、aggregation | 不是本文盘点的直接依赖；全部同步不重造这些运行时 |
| 母11 MCP | src/mcp、vw-agent/src/agent_apps/mcp | MCP权限与工具执行不由模板授予 |
| 母12 Skills | src/skills及相关工作区改动 | 并行会话所有；本项不修改、暂存或提交 |
| 母15/18/19/20 | 工作区编辑、备份同步、OS动作所有者 | OpenClaw编辑、配置恢复、设备同步、协议分发按各自生产接口消费，不在本领域代造 |

## 本轮交付状态

子任务1契约资料已编写，覆盖34个主责文件、13个命令、14应用矩阵及现有2–38任务链；未归属来源不留空。没有行为代码修改。按用户最新指令，编号任务1在资料完成后删除；不以未运行验证保留任务卡，不把本文件表述为母任务13完成。未执行任何Cargo操作、单元测试、实机测试、其他测试或最终回归，后续统一验证。
