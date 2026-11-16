# 任务10实施证据（未完成）

本轮遵守禁Cargo、禁测试要求；仅源码审查与差异检查，不表示编译、运行或绘制通过。编号卡保留，不能按完整静态交付删除。

## 已落实调用链

宿主 update → update::local::update_local：TaskUpdated、InsertDemoData、CardReleased、EditTask、CloseTaskLogs、CloseTaskPanel。领域持有这些分支的转换，日志同步效果交回既有 set_viewing_logs 接线；不反向依赖宿主。LocalState 增加任务可变借用、编辑ID、详情标签及日志自动滚动借用，没有复制 App。

CardReleased 原先模型 reasoning 验证结果立即被任务值覆盖；迁移保留最终任务值，模型选项查询没有外部效果。EditTask 原先先同步日志缓存再填写草稿，缓存只修改 logs，不修改草稿字段。

创建草稿重置集中到 draft::reset_create_draft，宿主 helper 和 local 都调用此真实入口。修复已有 LocalState selected_tasks 两处字段拼接语法错误。

## 断言归属

helpers_tests 整组4例逐字迁至 host_module_10_tests::helpers_tests，保留注册断言及 JSON/CSV/TSV 所有断言，仅替换领域类型路径；删除旧挂载与文件。

draft_adapter_tests 的 reset_create_draft_rebuilds_state_and_synchronizes_prompt_editor 四条断言迁至 host_module_10_tests::draft_adapter_tests，App 夹具改为真实 TaskDraft 与 text_editor::Content，调用生产重置入口。

## 未完成的必要实施

update.rs 的批量操作、任务创建后状态、持久化确认、投影后处理、工作树快照状态和设置截止时间仍需继续收口；bulk_tests、execution_tests、settings_tests、update_tests 中领域断言仍需完整迁移与逐项去重。当前不能将残留全部认定为系统效果装配，不宣称完整职责验收。

## 行数（splitlines，来源口径）

| 来源 | 基线 | 当前 | 减量 |
|---|---:|---:|---:|
| `crates/vw-desktop/src/app/message/task_board/helpers/bulk.rs` | 301 | 301 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers/bulk_tests.rs` | 221 | 221 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers/draft.rs` | 14 | 10 | 4 |
| `crates/vw-desktop/src/app/message/task_board/helpers/draft_adapter_tests.rs` | 15 | 0 | 15 |
| `crates/vw-desktop/src/app/message/task_board/helpers/execution.rs` | 559 | 559 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs` | 597 | 597 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers/logs.rs` | 55 | 55 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers/review.rs` | 2 | 2 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers/settings.rs` | 186 | 186 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers/settings_tests.rs` | 146 | 146 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers_tests.rs` | 74 | 0 | 74 |
| `crates/vw-desktop/src/app/message/task_board/update.rs` | 1907 | 1913 | -6 |
| `crates/vw-desktop/src/app/message/task_board/update_tests.rs` | 78 | 78 | 0 |

本任务未新增宿主替代文件。生产候选与测试须分开审计；当前表是中间状态，不是最终净减量。
