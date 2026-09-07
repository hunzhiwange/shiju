# TS 编辑器 editor2 计划

目标：在 `examples/editor2` 新建一个零依赖、严格类型检查通过的 TS 内存编辑器核心，作为 editor1 能力的扩展版本。

范围：`examples/editor2/` 内新增编辑核心 `src/editor.ts` 与 `test/*.test.ts`；用命令栈实现撤销/重做，保留 editor1 全部语义（索引校验、空操作不入历史、光标管理、按行读取、基于光标操作、setText 清历史、deleteForward/Backward 越界安全），补齐 editor1 未覆盖的插入级联删除、极限 count 与直接文本变换的测试。统一环境差异：tsconfig 增加 `erasableSyntaxOnly: true`、包名改为 `editor2` 并更新 README 为 editor2 专属能力清单。

范围外：不新增 UI/DOM/浏览器构建层，不支持 `words`、查找替换、多选区、piece table / gap buffer 重构，不引入任何运行时依赖，不改动其他 examples/editor 目录。

假设：仓库为 Rust 优先，TS 示例仅作为零依赖教学演示；`tsconfig` 的 `noUncheckedIndexedAccess` 维持 editor1 的严格开启状态；Node ≥ 22 依赖 `--experimental-strip-types` 且不支持 `erasableSyntaxOnly` 选项时，由验证脚本检测 Node 主版本并将该标志的编译选项置为可省略（见任务 2）。

## 设计决策

- 采用 editor1 的显式 `EditorChange`（type/start/payload/inserted/cursorBefore）与 `forwardApply/reverseApply` 模式，不引入录制/快照等抽象。
- 复用 editor1 的 `npm test`/`npm run typecheck` 契约；`package-lock.json` 沿用既有镜像源。
- 不引入 `words`、查找替换等新能力，保证计划完全基于 editor1 已验证的真实行为。
- 从命令栈角度可验证的最小行为集，全部通过聚焦单元测试验证，不使用 mock。

## 架构说明

- `src/editor.ts` 单一职责：`EditorChange` 接口 + `TextEditor` 类，所有编辑操作统一走 `record/forwardApply/reverseApply`。
- 关键约束：历史栈只记录真实变更；越界/非法索引一律抛 `RangeError`；`setText` 与任何新编辑都会清空重做栈。
- 验证命令必须落在 `examples/editor2`，不触碰仓库根级 Rust/Clippy。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 任务 1 先新增失败测试命令并确认失败，任务 2 实现使全部测试通过的最小代码，任务 3 收敛交付。