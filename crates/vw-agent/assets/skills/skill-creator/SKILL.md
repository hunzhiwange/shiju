---
name: skill-creator
description: 创建新技能、修改并改进已有技能，并衡量技能表现。当用户想从零创建技能、更新或优化已有技能、运行评测测试技能、用方差分析做性能基准，或优化技能描述以提升触发准确性时使用该技能。
---

# 技能创建器

用于创建新技能并迭代改进的技能。

从高层来看，创建技能的流程如下：

- 决定技能要做什么，以及大致如何实现
- 写一个技能草稿
- 创建一些测试提示词并用“可访问技能的 Claude”运行
- 帮助用户从定性与定量角度评估结果
  - 运行在后台进行时，如果还没有定量评测就起草一些（已有就直接用或根据需要调整），并向用户解释（若已有则解释已存在的）
  - 使用 `eval-viewer/generate_review.py` 脚本给用户展示结果，并查看定量指标
- 根据用户评估结果的反馈重写技能（以及定量基准暴露出的明显问题）
- 重复直到满意
- 扩大测试集并在更大规模上尝试

使用该技能时，你的任务是判断用户处在流程的哪个阶段，然后介入帮助他们推进。例如用户说“我想为 X 做个技能”，你可以帮助明确含义、写草稿、写测试用例、确定评估方式、运行所有提示词并迭代。

另一方面，如果他们已有技能草稿，就直接进入评估/迭代阶段。

当然要保持灵活，如果用户说“我不需要跑很多评测，就随性一点”，也可以照做。

技能完成后（顺序仍可灵活调整），还可以运行技能描述优化器——我们有单独脚本，用于优化技能触发效果。

可以吗？可以。

## 与用户沟通

技能创建器可能被对编码术语熟悉度跨度很大的人使用。如果你没听说（也合理，这个趋势刚出现），现在 Claude 的能力正在激励水管工打开终端、父母和祖父母去搜索“如何安装 npm”。另一方面，大多数用户可能相当懂电脑。

所以请注意上下文线索来决定表达方式！默认情况下，供参考：

- “evaluation”和“benchmark”是边界术语，但可以用
- “JSON”和“assertion”需要用户明显知道其含义时再不解释直接使用

不确定时可以简要解释术语，也可以用短定义澄清。

---

## 创建技能

### 捕捉意图

先理解用户意图。当前对话可能已包含用户想固化的流程（如“把这个变成技能”）。若是如此，先从对话历史中提取答案——使用了哪些工具、步骤顺序、用户做过的修正、观察到的输入/输出格式。用户可能需要补充空缺，并在进入下一步前确认。

1. 该技能应让 Claude 能做什么？
2. 该技能何时触发？（哪些用户措辞/上下文）
3. 预期输出格式是什么？
4. 是否要设置测试用例验证技能有效？具有客观可验证输出的技能（文件转换、数据提取、代码生成、固定流程步骤）适合测试用例。主观输出（写作风格、艺术）通常不需要。根据技能类型建议默认方案，但让用户决定。

### 访谈与调研

主动询问边界情况、输入/输出格式、示例文件、成功标准和依赖。先把这些理清再写测试提示词。

检查可用的 MCP——若对调研有帮助（查文档、找类似技能、找最佳实践），有子代理就并行调研，否则就地进行。准备好上下文，降低用户负担。

### 编写 SKILL.md

基于用户访谈，填充以下组件：

- **name**：技能标识
- **description**：何时触发、做什么。这是主要触发机制——描述里要包含技能做什么以及具体适用场景。“何时使用”的信息全部放在这里，而不是正文。注意：目前 Claude 有“触发不足”的倾向——在有用时不使用。为对抗这一点，请让技能描述稍微“主动”一点。例如不要写“如何构建一个简单快速的仪表盘用于展示 Anthropic 内部数据”，而应写“如何构建一个简单快速的仪表盘用于展示 Anthropic 内部数据。只要用户提到仪表盘、数据可视化、内部指标或想展示任何公司数据，即便没有明确提到‘仪表盘’，也要使用该技能。”
- **compatibility**：所需工具、依赖（可选，较少需要）
- **技能其余部分 :)**

### 技能写作指南

#### 技能结构

```
skill-name/
├── SKILL.md (required)
│   ├── YAML frontmatter (name, description required)
│   └── Markdown instructions
└── Bundled Resources (optional)
    ├── scripts/    - Executable code for deterministic/repetitive tasks
    ├── references/ - Docs loaded into context as needed
    └── assets/     - Files used in output (templates, icons, fonts)
```

#### 逐层披露

技能使用三层加载系统：
1. **元数据**（name + description）— 始终在上下文中（约 100 词）
2. **SKILL.md 正文** — 技能触发时加载（理想 <500 行）
3. **打包资源** — 按需加载（不限制，脚本可不加载也能执行）

字数为参考，如有需要可更长。

**关键模式：**
- 保持 SKILL.md 少于 500 行；若接近此限制，增加一层层级，并清晰指出使用技能的模型下一步应查看哪里。
- 在 SKILL.md 中清楚引用文件，并说明何时阅读它们
- 对大型参考文件（>300 行）加入目录

**领域组织**：当技能支持多个领域/框架时，按变体组织：
```
cloud-deploy/
├── SKILL.md (workflow + selection)
└── references/
    ├── aws.md
    ├── gcp.md
    └── azure.md
```
Claude 只读取相关参考文件。

#### 不惊讶原则

不言自明，技能不得包含恶意软件、漏洞利用代码或任何可能危害系统安全的内容。若按描述使用，技能内容不应让用户意图之外感到意外。不要配合创建误导性技能或用于未授权访问、数据外传或其他恶意活动的技能。“扮演某角色”类请求是可以的。

#### 写作模式

指令中优先使用祈使句。

**定义输出格式** — 可这样写：
```markdown
## 报告结构
始终使用此模板：
# [标题]
## 执行摘要
## 关键发现
## 建议
```

**示例模式** — 建议包含示例，可这样格式化（若示例含 “Input/Output”，可适当调整）：
```markdown
## 提交信息格式
**示例 1：**
输入：Added user authentication with JWT tokens
输出：feat(auth): implement JWT-based authentication
```

### 写作风格

尽量解释“为什么重要”，而不是生硬地写 MUST。运用心理模型，让技能尽量通用而非过度局限于特定例子。先写草稿，再用新视角审视并改进。

### 测试用例

写完技能草稿后，设计 2-3 个现实的测试提示词——真实用户会说的那种。与用户共享：[不必用完全相同措辞]“这里有几个我想试的测试用例，是否合适？要不要再加？”然后运行它们。

将测试用例保存到 `evals/evals.json`。先不要写断言——只写提示词。断言在下一步运行期间起草。

```json
{
  "skill_name": "example-skill",
  "evals": [
    {
      "id": 1,
      "prompt": "User's task prompt",
      "expected_output": "Description of expected result",
      "files": []
    }
  ]
}
```

完整 schema 见 `references/schemas.md`（包含稍后添加的 `assertions` 字段）。

## 运行与评估测试用例

本节为连续流程——中途不要停。不要使用 `/skill-test` 或任何其他测试技能。

将结果放在与技能目录同级的 `<skill-name>-workspace/` 中。在 workspace 内按迭代组织（`iteration-1/`、`iteration-2/` 等），每个测试用例一个目录（`eval-0/`、`eval-1/` 等）。不要一次性创建全部目录，随进度创建即可。

### 第一步：同一回合启动所有运行（with-skill 与 baseline）

对每个测试用例，在同一回合启动两个子代理——一个带技能，一个不带技能。注意：不要先跑带技能再回头跑基线。应一次性启动，确保结束时间接近。

**带技能运行：**

```
Execute this task:
- Skill path: <path-to-skill>
- Task: <eval prompt>
- Input files: <eval files if any, or "none">
- Save outputs to: <workspace>/iteration-<N>/eval-<ID>/with_skill/outputs/
- Outputs to save: <what the user cares about — e.g., "the .docx file", "the final CSV">
```

**基线运行**（同一提示词，基线取决于场景）：
- **创建新技能**：不使用技能。提示词相同，无技能路径，保存到 `without_skill/outputs/`。
- **改进已有技能**：使用旧版本。编辑前先快照技能（`cp -r <skill-path> <workspace>/skill-snapshot/`），让基线子代理指向该快照，保存到 `old_skill/outputs/`。

为每个测试用例写 `eval_metadata.json`（断言暂时可为空）。按测试内容为每个 eval 起描述性名称，而不是仅“eval-0”。目录也用该名称。若本次迭代新增或修改了 eval 提示词，为每个新 eval 目录创建这些文件——不要假设会从上一轮继承。

```json
{
  "eval_id": 0,
  "eval_name": "descriptive-name-here",
  "prompt": "The user's task prompt",
  "assertions": []
}
```

### 第二步：运行期间起草断言

不要只是等待运行结束——可利用这段时间起草每个测试用例的定量断言并向用户解释。若 `evals/evals.json` 已有断言，检查并解释其检验内容。

好的断言应客观可验证且命名清晰——在基准查看器中一眼就能明白检查项。主观技能（写作风格、设计质量）更适合定性评估——不要强行加断言。

断言起草完成后更新 `eval_metadata.json` 与 `evals/evals.json`。同时向用户解释他们在查看器中会看到什么——包括定性输出与定量基准。

### 第三步：运行完成时捕获耗时数据

每个子代理任务完成时会收到包含 `total_tokens` 和 `duration_ms` 的通知。立即将数据保存到运行目录的 `timing.json`：

```json
{
  "total_tokens": 84852,
  "duration_ms": 23332,
  "total_duration_seconds": 23.3
}
```

这是唯一捕获该数据的机会——它只出现在任务通知中，不会被持久化。通知到达即处理，不要批量处理。

### 第四步：评分、汇总并启动查看器

所有运行完成后：

1. **为每次运行评分** — 启动 grader 子代理（或内联评分），读取 `agents/grader.md` 并对输出评估每条断言。结果保存到每个运行目录的 `grading.json`。`grading.json` 的 expectations 数组必须使用 `text`、`passed`、`evidence` 字段（不要用 `name`/`met`/`details` 等），因为查看器依赖这些字段。对可程序化检查的断言，写并运行脚本，不要肉眼判断——脚本更快、更可靠、可复用。

2. **汇总成基准** — 在 skill-creator 目录运行汇总脚本：
   ```bash
   python -m scripts.aggregate_benchmark <workspace>/iteration-N --skill-name <name>
   ```
   该脚本生成 `benchmark.json` 与 `benchmark.md`，包含每个配置的 pass_rate、时间与 token，带均值 ± 标准差与差值。若手动生成 benchmark.json，请参见 `references/schemas.md` 的精确 schema。
将每个 with_skill 版本放在对应 baseline 版本之前。

3. **进行分析师复盘** — 阅读基准数据，找出汇总统计可能掩盖的模式。参见 `agents/analyzer.md` 的 “Analyzing Benchmark Results” 部分，关注：无论是否使用技能都通过的断言（无区分度）、高方差评测（可能不稳定）、时间/token 的权衡等。

4. **启动查看器**，展示定性输出与定量数据：
   ```bash
   nohup python <skill-creator-path>/eval-viewer/generate_review.py \
     <workspace>/iteration-N \
     --skill-name "my-skill" \
     --benchmark <workspace>/iteration-N/benchmark.json \
     > /dev/null 2>&1 &
   VIEWER_PID=$!
   ```
   迭代 2+ 时，额外传入 `--previous-workspace <workspace>/iteration-<N-1>`。

   **Cowork / 无界面环境：**如果 `webbrowser.open()` 不可用或环境无显示，使用 `--static <output_path>` 输出独立 HTML 文件而不是启动服务器。用户点击 “Submit All Reviews” 后会下载 `feedback.json`，下载后将其复制到 workspace 目录供下一轮使用。

注意：请使用 generate_review.py 创建查看器，无需编写自定义 HTML。

5. **告知用户**类似：“我已在浏览器中打开结果。有两个标签页——‘Outputs’ 可逐条查看测试用例并反馈，‘Benchmark’ 展示定量对比。完成后回来告诉我。”

### 用户在查看器中看到的内容

“Outputs” 标签页一次显示一个测试用例：
- **提示词**：给定的任务
- **输出**：技能产出的文件（可用时内联呈现）
- **上一次输出**（迭代 2+）：折叠区展示上轮输出
- **正式评分**（若已评分）：折叠区显示断言通过/失败
- **反馈**：自动保存的输入框
- **上一次反馈**（迭代 2+）：上轮评论，显示在输入框下方

“Benchmark” 标签页展示统计汇总：每个配置的通过率、耗时与 token 使用，包含每个 eval 的细分与分析师观察。

导航可通过前/后按钮或方向键完成。完成后点击 “Submit All Reviews”，会把所有反馈保存到 `feedback.json`。

### 第五步：读取反馈

用户告知完成后，读取 `feedback.json`：

```json
{
  "reviews": [
    {"run_id": "eval-0-with_skill", "feedback": "图表缺少坐标轴标签", "timestamp": "..."},
    {"run_id": "eval-1-with_skill", "feedback": "", "timestamp": "..."},
    {"run_id": "eval-2-with_skill", "feedback": "完美，很喜欢", "timestamp": "..."}
  ],
  "status": "complete"
}
```

空反馈表示用户认为没问题。改进应集中在用户提出具体问题的测试用例上。

完成后关闭查看器服务器：

```bash
kill $VIEWER_PID 2>/dev/null
```

---

## 改进技能

这是循环的核心。你已经运行测试用例，用户评审了结果，现在需要基于反馈改进技能。

### 如何思考改进

1. **从反馈中泛化。** 我们的目标是创建能在多种提示词下被大量使用的技能。你与用户反复迭代少量示例是为了加速验证，用户对这些例子很熟，评估很快。但如果技能只对这些例子有效，就毫无价值。与其加入过拟合的小改动或强硬 MUST，不如尝试不同隐喻或工作模式解决顽固问题。尝试成本不高，可能效果更好。

2. **保持提示精简。** 移除不必要内容。务必阅读对话记录而不仅是最终输出——如果技能让模型花时间做无效工作，尝试删掉导致这些行为的部分并观察效果。

3. **解释“为什么”。** 努力解释你让模型做某事的原因。当前 LLM 很聪明，有良好的心理模型，若引导得当可超越机械指令并产生真正效果。即便用户反馈简短或沮丧，也要理解任务、用户写了什么、为何这么写，然后把理解融入指令。如果你开始用全大写 ALWAYS/NEVER 或过于僵硬结构，这是黄灯——尽量改成解释原因，让模型理解重要性，这更人性、更强大、更有效。

4. **寻找跨用例的重复工作。** 阅读测试运行的对话记录，观察子代理是否都写了类似辅助脚本或采用相同多步流程。若 3 个测试用例都写了 `create_docx.py` 或 `build_chart.py`，这强烈表明技能应打包该脚本。只写一次，放入 `scripts/`，并指示技能使用它，避免每次重复造轮子。

这项任务非常重要（我们试图每年创造数十亿经济价值！），你的思考时间不是瓶颈；请花时间认真思考。我建议先写一版修订草案，再用新视角审视并改进。尽力站在用户角度理解他们真正想要什么、需要什么。

### 迭代循环

改进技能后：

1. Apply your improvements to the skill
2. Rerun all test cases into a new `iteration-<N+1>/` directory, including baseline runs. If you're creating a new skill, the baseline is always `without_skill` (no skill) — that stays the same across iterations. If you're improving an existing skill, use your judgment on what makes sense as the baseline: the original version the user came in with, or the previous iteration.
3. 使用 `--previous-workspace` 指向上一轮并启动评审器
4. 等待用户审阅并告知完成
5. 读取新反馈，继续改进并重复

持续迭代直到：
- The user says they're happy
- The feedback is all empty (everything looks good)
- You're not making meaningful progress

---

## 高级：盲测比较

当你需要更严格地比较两个技能版本（例如用户问“新版本真的更好吗？”），可以使用盲测比较系统。阅读 `agents/comparator.md` 与 `agents/analyzer.md` 了解细节。基本思路是：把两份输出交给独立代理，不告知哪个是哪个，让其判断质量，再分析胜出的原因。

该功能可选，且需要子代理，大多数用户不需要。通常人类评审循环已足够。

---

## 描述优化

SKILL.md frontmatter 中的 description 字段是决定 Claude 是否调用技能的主要机制。创建或改进技能后，可提供描述优化以提升触发准确性。

### 第一步：生成触发评测查询

创建 20 条 eval 查询——混合应触发与不应触发。保存为 JSON：

```json
[
  {"query": "the user prompt", "should_trigger": true},
  {"query": "another prompt", "should_trigger": false}
]
```

查询必须真实，且是 Claude Code 或 Claude.ai 用户可能会输入的内容。不要抽象请求，要具体、细节充分，比如：文件路径、用户工作/处境背景、列名与数值、公司名、URL。可以带一点背景。有些可全小写，包含缩写、错别字或口语。长度混合，重点放在边界情况，不要过于明确（用户会在后续确认）。

不好的例子：`"Format this data"`、`"Extract text from PDF"`、`"Create a chart"`

好的例子：`"ok so my boss just sent me this xlsx file (its in my downloads, called something like 'Q4 sales final FINAL v2.xlsx') and she wants me to add a column that shows the profit margin as a percentage. The revenue is in column C and costs are in column D i think"`

对于 **应触发** 查询（8-10 条），考虑覆盖面。需要同一意图的不同表述——有正式有口语。包含用户未明确提到技能或文件类型但显然需要的场景。加入一些不常见用例，以及与其他技能竞争但应胜出的场景。

对于 **不应触发** 查询（8-10 条），最有价值的是“擦边球”——与技能共享关键词或概念但实际上需要不同能力。考虑相邻领域、容易被关键词误触发的模糊表述，以及涉及技能能力但在该语境下更应使用其他工具的情况。

要避免的是让不应触发的查询明显无关。例如用 “Write a fibonacci function” 作为 PDF 技能的负例太容易，毫无测试价值。负例应真正具有迷惑性。

### 第二步：与用户审阅

使用 HTML 模板向用户呈现 eval 集进行审阅：

1. Read the template from `assets/eval_review.html`
2. Replace the placeholders:
   - `__EVAL_DATA_PLACEHOLDER__` → the JSON array of eval items (no quotes around it — it's a JS variable assignment)
   - `__SKILL_NAME_PLACEHOLDER__` → the skill's name
   - `__SKILL_DESCRIPTION_PLACEHOLDER__` → the skill's current description
3. Write to a temp file (e.g., `/tmp/eval_review_<skill-name>.html`) and open it: `open /tmp/eval_review_<skill-name>.html`
4. The user can edit queries, toggle should-trigger, add/remove entries, then click "Export Eval Set"
5. The file downloads to `~/Downloads/eval_set.json` — check the Downloads folder for the most recent version in case there are multiple (e.g., `eval_set (1).json`)

这一步很关键——糟糕的 eval 查询会导致糟糕的描述。

### 第三步：运行优化循环

告诉用户：“这需要一些时间——我会在后台运行优化循环并定期查看进度。”

将 eval 集保存到 workspace，然后后台运行：

```bash
python -m scripts.run_loop \
  --eval-set <path-to-trigger-eval.json> \
  --skill-path <path-to-skill> \
  --model <model-id-powering-this-session> \
  --max-iterations 5 \
  --verbose
```

使用系统提示中的模型 ID（驱动当前会话的那个），保证触发测试与用户实际体验一致。

运行期间定期查看输出，告知用户当前迭代与分数情况。

该流程会自动完成完整优化循环。它将 eval 集按 60% 训练、40% 测试拆分，评估当前描述（每条查询运行 3 次以获得可靠触发率），然后调用 Claude 的 extended thinking 基于失败项提出改进。每个新描述会在训练集与测试集上重新评估，最多迭代 5 次。完成后会在浏览器打开 HTML 报告展示每轮结果，并返回包含 `best_description` 的 JSON——以测试集分数选取以避免过拟合。

### 技能触发机制

理解触发机制有助于设计更好的 eval 查询。技能会以 name + description 出现在 Claude 的 `available_skills` 列表中，Claude 基于描述决定是否调用技能。关键点：Claude 只有在无法轻松自行完成任务时才会调用技能——简单的一步请求如“读取这个 PDF”即使描述完全匹配，也可能不触发，因为 Claude 可用基础工具直接处理。复杂、多步骤或专业化查询在描述匹配时会可靠触发。

这意味着 eval 查询必须足够“有分量”，让 Claude 需要借助技能。像“读文件 X”这类简单请求不是好用例——无论描述多好都不太触发技能。

### 第四步：应用结果

取 JSON 输出中的 `best_description` 并更新技能的 SKILL.md frontmatter。向用户展示前后对比并报告分数。

---

### 打包并呈现（仅在 `present_files` 工具可用时）

检查是否可使用 `present_files` 工具。若不可用，跳过此步骤。若可用，打包技能并将 .skill 文件提供给用户：

```bash
python -m scripts.package_skill <path/to/skill-folder>
```

打包后，告知用户生成的 `.skill` 文件路径以便安装。

---

## Claude.ai 特定说明

在 Claude.ai 中核心流程相同（草稿 → 测试 → 评审 → 改进 → 重复），但由于没有子代理，部分机制需要调整：

**运行测试用例**：无子代理意味着无法并行。每个测试用例都要读取技能 SKILL.md 并按指令亲自完成。逐个执行。这不如独立子代理严格（你写了技能又执行，拥有完整上下文），但仍是有用的 sanity check，人类评审可弥补。跳过基线运行——只用技能完成任务。

**评审结果**：若无法打开浏览器（如 Claude.ai 的 VM 无显示或在远程服务器），跳过浏览器查看器，改为在对话中直接呈现结果。每个测试用例展示提示词与输出。若输出是用户需要查看的文件（如 .docx 或 .xlsx），保存到文件系统并告知路径以便下载检查。内联询问反馈：“看起来如何？需要改吗？”

**基准评测**：跳过定量基准——它依赖基线对比，没有子代理时意义不大。专注于用户定性反馈。

**迭代循环**：与之前相同——改进技能、重跑测试用例、请求反馈——只是中间没有浏览器查看器。若有文件系统，仍可按迭代目录组织结果。

**描述优化**：该部分需要 `claude` CLI（尤其是 `claude -p`），仅在 Claude Code 可用。Claude.ai 下跳过。

**盲测比较**：需要子代理，跳过。

**打包**：`package_skill.py` 只要有 Python 与文件系统就能运行。在 Claude.ai 中可运行并让用户下载生成的 `.skill` 文件。

---

## Cowork 特定说明

若在 Cowork，主要注意：

- 有子代理，因此主流程（并行启动测试用例、运行基线、评分等）可用。（若超时问题严重，可改为串行运行测试提示词。）
- 没有浏览器或显示器，生成评测查看器时使用 `--static <output_path>` 输出独立 HTML 文件而非启动服务器，再给用户链接打开。
- 出于某些原因，Cowork 环境下 Claude 似乎不愿在测试后生成查看器，因此再强调：无论在 Cowork 还是 Claude Code，运行测试后都应使用 `generate_review.py` 先生成查看器供人类查看，再自己改技能（不要写自定义 html）。提前道歉，我要全大写强调：在你自己评估输入之前先生成评测查看器，尽快给人类看！
- 反馈机制不同：由于没有运行中的服务器，查看器的 “Submit All Reviews” 会下载 `feedback.json` 文件。随后从该文件读取（可能需要先请求访问）。
- 打包可用——`package_skill.py` 只需要 Python 与文件系统。
- 描述优化（`run_loop.py` / `run_eval.py`）在 Cowork 中应可正常工作，因为它通过子进程调用 `claude -p`，不依赖浏览器，但请等技能完成且用户认可后再做。

---

## 参考文件

agents/ 目录包含专用子代理说明。需要启动相关子代理时再阅读。

- `agents/grader.md` — How to evaluate assertions against outputs
- `agents/comparator.md` — How to do blind A/B comparison between two outputs
- `agents/analyzer.md` — How to analyze why one version beat another

references/ 目录包含额外文档：
- `references/schemas.md` — JSON structures for evals.json, grading.json, etc.

---

再次强调核心循环：

- 明确技能内容
- 起草或编辑技能
- 用可访问技能的 Claude 运行测试提示词
- 与用户一起评估输出：
  - 生成 benchmark.json 并运行 `eval-viewer/generate_review.py` 方便用户审阅
  - 运行定量评测
- 重复直到你和用户满意
- 打包最终技能并交付给用户。

如果你有 TodoList，请把步骤加入以免遗忘。如果在 Cowork，请明确把“创建 evals JSON 并运行 `eval-viewer/generate_review.py` 以便人类审阅测试用例”写入 TodoList。

祝好运！
