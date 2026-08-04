# Workflow Dev Lifecycle Demo

这个 demo 模拟一个需求从提出到完成的过程：

```text
需求 -> 评审 -> 开发 -> 测试 -> 完成
```

它只使用 `start`、`code`、`if-else`、`answer` 节点，不依赖 LLM 或外部 API。

## 启动网关

```bash
cargo run -p vw-cli --bin vibewindow -- gateway --host 127.0.0.1 --port 42617
```

## 跑通过路径

```bash
curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @demo/vw-workflow-dev-lifecycle-request.json
```

预期结果：

```text
stage=完成
requirement=新增一个订单导出按钮，支持按日期筛选并导出 CSV
reviewer=产品评审会
branch=feature/dev-lifecycle-demo
test=all lifecycle checks passed
result=需求已评审、开发、测试并完成交付
```

## 跑评审失败路径

```bash
jq '.inputs.review_result="rejected"' demo/vw-workflow-dev-lifecycle-request.json \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

## 跑测试失败路径

```bash
jq '.inputs.test_result="failed"' demo/vw-workflow-dev-lifecycle-request.json \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

## 本地试跑结果

已实际跑通三条路径：

- 通过路径：最后节点 `done`，输出 `stage=完成`
- 评审失败：最后节点 `review_failed`，输出 `stage=评审未通过`
- 测试失败：最后节点 `test_failed`，输出 `stage=测试未通过`
