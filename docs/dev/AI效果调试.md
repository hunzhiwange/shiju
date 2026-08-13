# LLM 效果调试

## 你可以参考这个调试LLM效果

```
curl -sS -N --max-time 180 -H 'Content-Type: application/json' -H 'X-Platform-ID: 10000' -H 'X-Company-ID: 100000000' -H 'X-VibeWindow-Directory: /Users/Shared/work/dir/data/codes/vibe-window' --data-binary '{"model":"shiju/deepseek-v4-flash","agent":"main","messages":[{"role":"user","content":"定位 assemble_session_protocol_messages 的调用者，不要修改代码，简要回答。"}]}' http://127.0.0.1:42617/v1/chat/stream | rg 'context_loaded|tool_call|tool_result|chat\.delta|chat\.done|chat\.error';
```
