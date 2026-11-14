# 来源覆盖与实现交接

2026-11-14，任务1源码审查。后续任务在本表对应位置补实际落点；本轮不运行 Cargo、单元测试、实机或回归，所有运行证据均待统一验证，不能把审查或任务卡删除视为验证通过。

## 来源完整性

源仓 HEAD 与冻结提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 一致。只读母来源清单中主责14共80文件：仓内77文件 SHA256 全部对应；三份 user-manual 会话文档未包含在 upstream，读取固定源仓对应文件并核对哈希，不能因此删掉手册需求。下文来源路径相对 cc-switch；正式实现须与冻结快照分离。

## 适配器与恢复边界

`session_manager/mod.rs::scan_sessions` 实际调用13来源；`load_messages` 为 MCode、OpenCode/Hermes SQLite 设独立分支，再按应用分发文件解析。`utils.ts::SESSION_APP_IDS` 实际13项，注释“10个”已滞后。Claude Desktop 没有独立来源，只映射 Claude Code。

| 应用 key | 发现/读取任务 | 删除任务 | 来源恢复能力 | 实现/API/SDK/UI/测试/实测 |
| --- | --- | --- | --- | --- | --- |
| claude | 6、7 | 32 | claude --resume ID | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| codex | 4、5、8、9 | 33 | codex resume ID | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| opencode | 10、11 | 40、42 | opencode -s ID | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| hermes | 12、13 | 41、43 | 不支持 | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| pi | 14、15 | 37 | pi --session SOURCE | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| gemini | 16 | 34 | gemini --resume ID | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| openclaw | 17 | 35 | 不支持（gateway 管理） | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| grokbuild | 18 | 36 | grok --resume ID | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| kimi | 19 | 39 | 受控 KIMI_CODE_HOME + kimi --session ID | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| omp | 20 | 38 | 受控 profile 与来源路径 | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| mcode | 21 | 44 拒绝 | 受控 runtime/session ID | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| dsh | 22 | 44 拒绝 | 不支持 | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |
| agy | 23 | 44 拒绝 | 受控 AGY 配置/账号上下文 | 待实现 / 待接线 / 待接线 / 待接线 / 未运行 / 未运行 |

十种文件删除和两种 SQLite 删除独立实现，MCode/AGY/DSH 后端明确拒绝。恢复是否可用不得仅由 UI 按钮推断。受控重建归46，身份隔离归31。

## 唯一业务所有者

| 能力及核对符号 | 领域实现任务 | API / SDK | Iced | 测试 / 实测 |
| --- | --- | --- | --- | --- |
| model.rs::SessionMessage/SessionBlock/ContentRef/ImageRef、手写 Serialize | 2 | 61–72引用共享DTO | 73–82消费 | 未运行 / 未运行 |
| providers/blocks.rs、utils.rs 工具归一化与内容投影 | 3 | 消息接口 | 75 | 未运行 / 未运行 |
| codex_rollout_file.rs；codex_state_db.rs | 4、5 | 间接 | 无重复逻辑 | 未运行 / 未运行 |
| content.rs::validate_source/resolve_under_roots/resolve_content_ref/load_image | 24–27 | 64、65 | 76 | 未运行 / 未运行 |
| cache.rs::Transcript/TranscriptCache/chunk_ranges | 28 | 62、63 | 只消费分块 | 未运行 / 未运行 |
| reader/turns.ts、toolSummary.ts、上下文/步骤/代码/diff/注入/思考/最终回复 | 29（领域投影） | 62 | 74、75、76（展示） | 未运行 / 未运行 |
| utils.ts::getSessionKey/getSessionTime/isArchivedSession/groupSessionsByProject | 30（索引/确定顺序） | 61 | 73（分组展示/日期本地化） | 未运行 / 未运行 |
| useSessionSearch.ts::useSessionSearch | 30（元数据搜索） | 61 | 73（输入/取消/陈旧响应） | 未运行 / 未运行 |
| mod.rs::delete_session/delete_sessions/collect_delete_session_outcomes | 32–45，各应用仅一个所有者 | 66 | 77 | 未运行 / 未运行 |
| terminal/mod.rs::launch_terminal/shell_escape | 46（命令）、47–53（终端） | 67 | 78 | 未运行 / 未运行 |
| codex_history_migration.rs；settings.rs 历史选项和回滚入口 | 54–59 | 68、69 | 81 | 未运行 / 未运行 |
| codex_session_compression.rs；settings.rs 压缩/磁盘入口 | 60 | 70 | 82 | 未运行 / 未运行 |
| exportMarkdown.ts 生成与安全文件名 | 71 | 72 | 80（复制/用户选文件） | 未运行 / 未运行 |
| commands/session_manager.rs::reveal_session_path | 受控位置身份/API合同 | 由会话响应提供位置提示 | 79，同机 Finder | 未运行 / 未运行 |
| hermes_wsl.rs、hermes_wsl_reader.py | 保留来源，不在本轮验收范围 | 不宣称支持 | 不宣称支持 | 未运行 / 未运行 |

TS 搜索只索引 sessionId/title/summary/projectDir/sourcePath 元数据，不是消息全文；空查询按时间降序。来源同时间排序没有唯一 tie-breaker，新稳定分页由30补齐。项目 key 去尾分隔符，Windows key 比较时归一大小写，显示仍保留原文；未知项目最后。阅读查找、导航返回、展开、批选、复制是UI状态，正文/工具摘要/轮次领域投影不能在UI另造第二份。source-map.md 提及83–85为旧规划残留，当前 prompt.md 只调度1–82；未来验证不因此假称已交付。

## 九终端与 custom

| 来源 launch_terminal 分支 | 唯一任务 | 来源调用点 | 实测 |
| --- | --- | --- | --- |
| terminal、iterm（设置 iterm2 映射） | 47 | launch_macos_terminal、launch_iterm | 未运行 |
| ghostty、kitty | 48 | launch_ghostty、launch_kitty | 未运行 |
| wezterm、kaku | 49 | launch_wezterm、launch_kaku | 未运行 |
| otty | 50 | launch_otty；CLI候选和可执行检查 | 未运行 |
| alacritty | 51 | launch_alacritty | 未运行 |
| warp | 52 | launch_warp，要求cwd | 未运行 |
| custom | 53 | launch_custom，旧来源任意模板不可直接公开 | 未运行 |

## 十命令交接

来源 lib.rs 注册的 list_sessions→61；get_session_messages→62；stream_session_messages→63；get_session_block_content→64；get_session_image→65；delete_session/delete_sessions→66；launch_session_terminal→67；export_session_markdown→71/72/80；reveal_session_path→79。无通用 Tauri 命令分发。settings.rs::get_settings/save_settings 的历史选项、has_codex_unify_history_backup/restore_codex_unified_history→57–59/69，get/set_codex_session_compression 与磁盘查询→60/70。

`claude.rs::output_shape_matches_shared_fixture` 中“Exit code 101”是构造工具输出的测试文本，不是运行事件API。fixtures 是跨语言合同样例，测试源可以参考，不宣称已运行。

## 本轮证据与后续

只读命令：git status --short，源仓 rev-parse HEAD，RustCodeGraph status/explore，以及读取/搜索冻结源码与相邻测试；graph explore 未覆盖本次入口，回退原始文件。Python只读SHA256核对77个快照文件。正式 sessions 模块、共享DTO、API/SDK/UI尚未实现；现有 grant 未含会话操作，须领域接线时显式新增。母1/2/3/4卡已缺失，母13未开始，代码事实与缺卡分开记录。没有需要用户新增决策的外部阻塞。

按用户本轮覆盖，任务1合同写完后删除任务卡；不调用 git-commit、不执行Cargo或任何测试，不将删除解释为验证通过。下一步任务2实现外部会话数据合同，验证统一后补。

手册源仓哈希核对：`docs/user-manual/en/3-extensions/3.4-sessions.md`：一致；`docs/user-manual/ja/3-extensions/3.4-sessions.md`：一致；`docs/user-manual/zh/3-extensions/3.4-sessions.md`：一致。

## 任务2：共享数据合同（实现，未验证）

`vw-api-types::external_session` 已注册共享 DTO。`ExternalSessionIdentity` 显式绑定 ProjectScope、主机、实例、13来源类型、独立 ExternalSessionId 和受控来源定位符；不复用识矩/ACP 身份。结构化消息、工具/事件/diff、ContentRef、ImageRef、TranscriptChunk 和 TurnIndex 沿用固定来源字段语义；blocks 非空省略重复 content；redacted thinking 序列化清除 text/summary/full，类型不定义 encrypted_content/signature 载荷。删除结果区分 Deleted/AlreadyAbsent/Failed。

落点：`crates/vw-api-types/src/external_session.rs`、`external_session/model.rs`、`external_session/model/wire.rs`、`lib.rs`。数据模型只声明引用，不授予文件/数据库访问权限；来源校验归后续领域任务。本轮依用户指示未运行 Cargo、单元/实机测试或回归，不自动提交；序列化往返、边界和编译统一后验。

## 任务3实现交接

共用块工具位于 `crates/vw-agent-apps/src/sessions/blocks.rs`，拆分 `tool_normalization.rs`、`block_preview.rs` 和纯文本 `utils.rs`。正式包已注册 sessions。复用 vw-api-types 的块及投影类型，不复制身份或序列化模型。来源工具配对键、MCP 分类、diff 统计、Unicode 预览与延迟引用均保留；纯文本提取过滤 thinking/reasoning/redacted。读文件、缓存与轮次编排由后续任务负责。

按用户本轮要求，未执行 Cargo、单元测试、实机测试或提交，验证留待统一进行；任务卡在代码写完后删除，不表示运行验收通过。

## 任务6：Claude 目录发现（实现，验证后补）

- `sessions/providers/claude.rs::scan_sessions` 接收执行主机选定实例的绝对配置根与身份；仅扫描其 projects JSONL，不回退环境 HOME，不跟随扫描项符号链接。
- 迁入来源头10/尾30行元数据、64KiB relocated 窗口、自定义标题优先级、注入文本过滤及损坏JSON行跳过；结果绑定相对来源定位符与执行主机身份，无原始恢复命令。
- 按用户要求仅写代码，未执行 Cargo、单元测试或实机验证，未提交；后续统一验证。

任务12：`sessions/providers/hermes.rs::{scan_sessions,load_messages}` 已迁入 Hermes 文件发现（`.jsonl`/`.json`）、30/10行元数据窗口、扁平/嵌套消息、工具调用与结果引用。实例根显式传入，目录/文件符号链接不参与扫描；CLI 恢复能力为 false。SQLite 归任务13；无文件条数上限（来源500条限制仅适用于SQLite）。按用户要求未运行Cargo、单元或实机测试，待以后统一验证。

## 任务10：OpenCode 文件会话

已迁入 `sessions/providers/opencode.rs` 的显式 XDG 数据根解析、文件发现及消息加载，`opencode_blocks.rs` 复用来源 part 映射；parts按文件名稳定排序，消息按时间和ID排序，跳过synthetic/ignored正文，保留工具、步骤、图片和redacted推理。文件locator相对storage，读取限制在主机提供的storage根内。SQLite由任务11实现，轮次由任务29统一生成。按用户要求未运行Cargo、单元或实机测试，未提交；待未来统一验证。

### 任务7：Claude结构化消息（仅代码）

`providers/claude.rs::load_messages` 接入快照的宽松JSONL解析，拆分 `claude_messages.rs`、`claude_blocks.rs`、`claude_wire.rs`、`claude_jsonl.rs`，保留工具结果回填、思考签名/隐藏正文、图片引用、注入、compact/hook/中断事件及轮次。读取错误明确返回，sidecar相对引用拒绝父目录组件。按本次用户要求未运行Cargo、单元测试、实机或回归，也未提交；全部验证留待以后。

任务11：`sessions/providers/opencode_sqlite.rs::{scan_sessions,load_messages}` 迁入只读 SQLite V1/V2 及混合迁移标记规则，数据库结果优先于同实例文件来源；复用 OpenCode parts 映射与全文引用，V2 按 seq/rowid 排序。数据库固定为执行主机数据根内 opencode.db，拒绝符号链接；无源记录修改。按用户要求未运行 Cargo、单元或实机测试，未提交，留待以后统一验证。

## 任务8：Codex发现与标题（代码交付，未验证）

`src/sessions/providers/codex.rs::scan_sessions` 从显式执行主机根扫描活动与归档 rollout，复用压缩读取与状态库路径解析；同ID活动来源优先。标题按状态库显式标题、session_index 最新名称、用户首问、项目名排序；不按model_provider筛选。SQLite只读查询排除与首问相同的标题，不读取首问大字段；数据库不可用返回明确错误。不读取桌面进程环境。新增 rusqlite 0.37 bundled 依赖供后续数据库适配器复用。未运行Cargo、测试或实机，统一验证留待以后。

任务13：`sessions/providers/hermes_sqlite.rs::{scan_sessions,load_messages}` 迁入只读 state.db，会话最近500条及 `sqlite_truncated` 标识、SQLite同ID优先于JSONL；消息保留插入顺序、压缩历史去重、rewind/model_only过滤、结构化内容与工具/推理引用。实例根由目标主机显式传入，固定state.db且拒绝数据库符号链接；无WSL reader或写操作，resume=false。按用户要求未运行Cargo、单元测试、实机或回归，未提交，验证留待以后。

任务9：`providers/codex.rs::load_messages` 接入普通/压缩 rollout 的借用式结构化解析，`codex_messages` 与拆分状态机保留 response_item/event_msg、工具配对/迟到结果、apply_patch、子代理工具及压缩事件；`codex_items` 复用来源命令/补丁归一化。encrypted_content 不声明、不保存、不解密；图片路径只生成引用，不在解析阶段探测目标主机以外的文件。新增 regex 依赖。按用户要求未运行 Cargo、单元测试、实机或回归，未提交，验证留待以后。

任务17：`sessions/providers/openclaw.rs::{scan_sessions,load_messages}` 读取显式执行主机根下 agents/<agent>/sessions JSONL 与 sessions.json 标题，agent 保留在来源定位符、项目保留在 project_dir；复用 PiTranscript，按文件顺序读取全部消息，不进行树分支裁剪。拒绝越界/符号链接，恢复能力 resume=false，不迁入删除实现。按用户要求未运行 Cargo、单元测试、实机或回归，未提交，留待以后统一验证。

## 任务19：Kimi读取（代码已写，未验证）

- 实现：`sessions/providers/kimi.rs` 与 `kimi_requests.rs`，显式使用目标实例数据根；扫描原生 state/main wire，排除 archived/child，保留活动分支、committed human 优先、工具与思考块、JSONL偏移引用和请求模型/耗时关联。
- 注册：`sessions/providers/mod.rs`；未迁入删除/任意命令执行。受控恢复由后续恢复命令任务负责。
- 按用户要求未运行 Cargo、单元测试、实机或回归，未提交；编译及行为验证留待统一进行。

## 任务21：MCode运行时只读历史

- `sessions/providers/mcode.rs` 复用固定快照MCode可见根会话过滤与迁移标记优先消息SQL；`data_dir` 显式接收目标主机 MINIMAX_DATA_DIR/MAVIS_DATA_DIR 与 home，固定 `v2/sqlite/runtime-state.sqlite`，只读打开并拒绝符号链接路径。
- `scan_sessions` 输出领域身份与只读能力；`load_messages` 仅展示 user/assistant 文本，迁移行存在时忽略旧JSON；`delete_session` 明确返回不支持，无数据库写入。父模块已注册。
- 按本轮用户要求仅写代码，未运行 Cargo、单元测试或实机测试，未提交；验证留待统一进行。

## 任务22：DSH只读会话

- `sessions/providers/dsh.rs` 迁入最高 session.vN.jsonl[.zstd] 选择、独立zstd帧/未完成尾帧、结构化消息、工具结果、JSONL引用及model-only替换过滤；目标主机数据根显式传入，拒绝符号链接，读取有128MiB边界。
- 原生Web UI保有恢复/删除；领域能力delete/resume为false，delete_session明确拒绝；父模块已注册。
- 按用户要求未运行Cargo、单元测试、实机或回归，未提交；编译与行为验证留待统一执行。

### 任务25：文件长内容（代码交付，未验证）

- `sessions::content_file::{resolve_content_ref,paginate}` 复用 `ValidatedSource`，按会话归属读取 JSONL/File/Claude Sidecar，支持压缩逻辑偏移、行边界校验、UTF-8 字符页、32MiB/512Ki 限制。OpenCode part 通过消息记录核对 sessionID。
- 按用户本次要求未运行 Cargo、单元测试或实机测试，未提交；统一验证留后续进行。

### 任务26：数据库长内容（代码交付，未验证）

- `sessions::content_sqlite::{resolve_content_ref,read_block_content}` 仅接受 `ValidatedSource`，按 OpenCode/Hermes 表列白名单读取，行 ID 与当前会话 ID 参数绑定，其他应用、其他会话、任意表列及非 SQLite 引用明确拒绝。只读打开前复验路径与符号链接；单条 SQL 的长度/CASE 检查避免复制超过 32MiB 的单元格，拒绝重复记录和非文本，JSON pointer 后复用 UTF-8 字符分页。
- 按用户本次要求未运行 Cargo、单元测试、实机测试或回归，未提交；编译与行为验证留待以后统一进行。

### 任务29：对话轮次与阅读投影（代码交付，未验证）

- `sessions::turns::{project,build_turns,build_turn_index,flatten_rows,effective_blocks}` 迁入固定来源 Rust/TS 的原生轮 ID/旧消息兜底、提问与最后调用后答复定位、早到结果配对、注入索引、前置/尾随事件、中断和分段成本。连续成功 Codex read/search 与 Claude 同服务器 MCP 按风格合并；失败/中断独立保留。阅读行支持全部/对话/改动过滤及显式注入显示。
- 已阅读来源 `reader/turns.test.ts` 的六份契约故事与 TS 合并规则；按用户要求未运行 Cargo、单元测试、实机或回归，不声称已通过六份 fixture；验证留待统一进行。

## 任务30：元数据目录

`catalog::{Catalog::from_batches,Catalog::page,CatalogQuery,CatalogPage}` 汇总授权项目内各适配器扫描结果，保留失败、拒绝冲突身份，按活跃时间和完整身份稳定排序。元数据分词全包含匹配，支持应用/主机/实例/项目目录过滤。不可变快照五分钟有效，随机游标服务端绑定筛选与页大小，跨作用域拒绝，换代/过期明确 CursorExpired；5万条和512游标上限，不静默截断。API任务61负责授权扫描与持有快照。按本次用户要求未运行 Cargo、单元测试、实机或回归验证，不声明通过。
