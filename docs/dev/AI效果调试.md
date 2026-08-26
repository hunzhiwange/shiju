# Agent 效果调试

日常效果回归使用 [Promptfoo 评测](../../evals/promptfoo/README.md)。它会重复调用
`/v1/chat/stream`，自动检查回答、工具轨迹、token 和耗时。

```bash
node --test evals/promptfoo/*.test.mjs
npx --yes promptfoo@0.122.0 eval -c evals/promptfoo/promptfooconfig.yaml --no-cache -j 1
```

修改 Agent prompt、工具描述、上下文装配或模型配置后，应先运行聚焦用例，再比较
Promptfoo 报告中的成功率、工具调用次数、step、token 与耗时。新增线上问题时，把它简化为
`evals/promptfoo/promptfooconfig.yaml` 中的确定性回归用例。

## 固定的前后对比闭环

结果可能包含源码与工具载荷，只写入已忽略的 `evals/promptfoo/results/`，不要提交或共享。

```bash
# 1. Gateway 已运行，修改产品代码前导出基线
mkdir -p evals/promptfoo/results
npx --yes promptfoo@0.122.0 eval \
  -c evals/promptfoo/promptfooconfig.yaml --no-cache -j 1 \
  -o evals/promptfoo/results/before.json

# 2. 修改产品代码并完成聚焦验证；终止旧进程后，在独立终端重启 Gateway
cargo run -p vw-cli --bin shiju -- gateway --host 127.0.0.1 --port 42617

# 3. 用相同矩阵导出并比较
npx --yes promptfoo@0.122.0 eval \
  -c evals/promptfoo/promptfooconfig.yaml --no-cache -j 1 \
  -o evals/promptfoo/results/after.json
node evals/promptfoo/compare-results.mjs \
  evals/promptfoo/results/before.json \
  evals/promptfoo/results/after.json \
  > evals/promptfoo/results/comparison.md
npx --yes promptfoo@0.122.0 view
```

先看对比报告定位失分层，再用 viewer 下钻具体轨迹：direct-call 低时查工具注册与暴露；
operation 低时改工具描述和示例；success 低时修工具实现或输入；facts 低时改结果表达；
fallback 低时排查内置工具与 CLI、`grep`、`rg` 指导冲突。完整字段说明和错误语义见
[Promptfoo 评测操作文档](../../evals/promptfoo/README.md)。

## 人工排障

需要观察完整 SSE 事件时，仍可使用下面的单任务命令：

```
curl -sS -N --max-time 180 -H 'Content-Type: application/json' -H 'X-Platform-ID: 10000' -H 'X-Company-ID: 100000000' -H 'X-VibeWindow-Directory: /Users/Shared/work/dir/data/codes/vibe-window' --data-binary '{"model":"token-plan/qwen3.7-plus","agent":"main","messages":[{"role":"user","content":"定位 assemble_session_protocol_messages 的调用者，不要修改代码，简要回答。"}]}' http://127.0.0.1:42617/v1/chat/stream | rg 'chat\.(context_loaded|tool|post_tool_round|step_start|step_finish|delta|done|error)';
```
