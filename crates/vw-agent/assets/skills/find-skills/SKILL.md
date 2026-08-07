---
name: find-skills
description: 当用户询问“如何做 X”、“帮我找个 X 的技能”、“有没有技能可以……”，或表达想扩展能力时，帮助用户发现并安装代理技能。用户在寻找可能以可安装技能形式存在的功能时应使用该技能。
---

# 查找技能

该技能帮助你从开放的代理技能生态中发现并安装技能。

## 何时使用该技能

当用户出现以下情况时使用该技能：

- 询问“如何做 X”，其中 X 可能是已有技能覆盖的常见任务
- 说“找一个 X 的技能”或“有没有 X 的技能”
- 问“你能做 X 吗”，且 X 是专业能力
- 表达想扩展代理能力
- 想搜索工具、模板或工作流
- 提到希望在特定领域获得帮助（设计、测试、部署等）

## 什么是 Skills CLI？

Skills CLI（`npx skills`）是开放代理技能生态的包管理器。技能是模块化包，用专门知识、工作流和工具扩展代理能力。

**关键命令：**

- `npx skills find [query]` - 交互式或按关键词搜索技能
- `npx skills add <package>` - 从 GitHub 或其他来源安装技能
- `npx skills check` - 检查技能更新
- `npx skills update` - 更新所有已安装技能

**浏览技能：** https://skills.sh/

## 如何帮助用户查找技能

### 第一步：理解需求

当用户请求帮助时，识别以下要素：

1. 领域（如 React、测试、设计、部署）
2. 具体任务（如写测试、做动画、评审 PR）
3. 任务是否常见到很可能已有技能

### 第二步：搜索技能

用相关查询运行 find 命令：

```bash
npx skills find [query]
```

例如：

- 用户问“如何让我的 React 应用更快？” → `npx skills find react performance`
- 用户问“你能帮我做 PR 评审吗？” → `npx skills find pr review`
- 用户问“我需要创建 changelog” → `npx skills find changelog`

命令会返回类似结果：

```
Install with npx skills add <owner/repo@skill>

vercel-labs/agent-skills@vercel-react-best-practices
└ https://skills.sh/vercel-labs/agent-skills/vercel-react-best-practices
```

### 第三步：向用户展示选项

当你找到相关技能时，向用户展示：

1. 技能名称与用途
2. 可运行的安装命令
3. skills.sh 的了解链接

示例回复：

```
I found a skill that might help! The "vercel-react-best-practices" skill provides
React and Next.js performance optimization guidelines from Vercel Engineering.

To install it:
npx skills add vercel-labs/agent-skills@vercel-react-best-practices

Learn more: https://skills.sh/vercel-labs/agent-skills/vercel-react-best-practices
```

### 第四步：提出安装

若用户愿意继续，你可以为其安装技能：

```bash
npx skills add <owner/repo@skill> -g -y
```

`-g` 表示全局安装（用户级），`-y` 跳过确认提示。

## 常见技能类别

搜索时可参考以下常见类别：

| Category        | Example Queries                          |
| --------------- | ---------------------------------------- |
| Web Development | react, nextjs, typescript, css, tailwind |
| Testing         | testing, jest, playwright, e2e           |
| DevOps          | deploy, docker, kubernetes, ci-cd        |
| Documentation   | docs, readme, changelog, api-docs        |
| Code Quality    | review, lint, refactor, best-practices   |
| Design          | ui, ux, design-system, accessibility     |
| Productivity    | workflow, automation, git                |

## 有效搜索技巧

1. **使用具体关键词**：`react testing` 比只写 `testing` 更好
2. **尝试替代词**：`deploy` 不行就试 `deployment` 或 `ci-cd`
3. **查看热门来源**：许多技能来自 `vercel-labs/agent-skills` 或 `ComposioHQ/awesome-claude-skills`

## 找不到技能时

如果没有相关技能：

1. 明确告知未找到现有技能
2. 提出用你的通用能力直接协助完成任务
3. 建议用户使用 `npx skills init` 创建自己的技能

示例：

```
I searched for skills related to "xyz" but didn't find any matches.
I can still help you with this task directly! Would you like me to proceed?

If this is something you do often, you could create your own skill:
npx skills init my-xyz-skill
```
