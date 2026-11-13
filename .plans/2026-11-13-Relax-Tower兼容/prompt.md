# Relax Tower 兼容任务提示词

按批次顺序执行，每批仅一项，无并行文件写入。

## 批次 1：求解上下文 依赖：无

```text
请使用技能 do-task-plans 执行 `.plans/2026-11-13-Relax-Tower兼容/1-求解上下文.md`。
```

## 批次 2：两端联动 依赖：批次1

```text
请使用技能 do-task-plans 执行 `.plans/2026-11-13-Relax-Tower兼容/2-两端联动.md`。
```

## 批次 3：补丁与证据 依赖：批次2

```text
请使用技能 do-task-plans 执行 `.plans/2026-11-13-Relax-Tower兼容/3-补丁与证据.md`。
```
