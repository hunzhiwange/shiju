# 任务321：修复vw-agent宿主的chrome_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：继续执行宿主单测；诊断编号56, 57, 58（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/tool-computer/src/computer_control/chrome_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断56：E0624

位置：`crates/libs/tool-computer/src/computer_control/chrome_tests.rs:85`。

```text
error[E0624]: method `consume_transfer` is private
   --> crates/vw-agent/src/tools/../../../libs/tool-computer/src/computer_control/chrome_tests.rs:85:14
    |
 85 |           host.consume_transfer(&r).unwrap();
    |                ^^^^^^^^^^^^^^^^ private method
    |
   ::: crates/libs/computer-control/src/host.rs:337:5
    |
337 | /     pub(crate) fn consume_transfer(
338 | |         &self,
339 | |         request: &ComputerControlRequest,
340 | |     ) -> Result<(), ControlErrorCode> {
    | |_____________________________________- private method defined here
```

当前源码片段：

```rust
81:         },
82:     };
83:     for _ in 0..2 {
84:         scope_tool_use_context(ctx.clone(), approve_transfer(&host, &r)).await.unwrap();
85:         host.consume_transfer(&r).unwrap();
86:     }
87:     let entries = seen.lock();
88:     assert_eq!(entries.len(), 2);
89:     assert!(entries.iter().all(|(name, args)| name == "computer_control.chrome_upload"
90:         && !args.to_string().contains("eA==")
```

### 宿主诊断57：E0624

位置：`crates/libs/tool-computer/src/computer_control/chrome_tests.rs:103`。

```text
error[E0624]: method `consume_transfer` is private
   --> crates/vw-agent/src/tools/../../../libs/tool-computer/src/computer_control/chrome_tests.rs:103:21
    |
103 |       assert_eq!(host.consume_transfer(&r), Err(ControlErrorCode::PermissionDenied));
    |                       ^^^^^^^^^^^^^^^^ private method
    |
   ::: crates/libs/computer-control/src/host.rs:337:5
    |
337 | /     pub(crate) fn consume_transfer(
338 | |         &self,
339 | |         request: &ComputerControlRequest,
340 | |     ) -> Result<(), ControlErrorCode> {
    | |_____________________________________- private method defined here
```

当前源码片段：

```rust
99:             .with_bypass_non_cli_approval_for_turn(true)
100:             .with_channel("slack"),
101:     );
102:     assert!(scope_tool_use_context(ctx, approve_transfer(&host, &r)).await.is_err());
103:     assert_eq!(host.consume_transfer(&r), Err(ControlErrorCode::PermissionDenied));
104: }
105: #[tokio::test]
106: async fn cc_control_10_site_approval_cannot_override_explicit_deny() {
107:     let (_, p, _) = authority();
108:     let (ctx, seen) = approval_context();
```

### 宿主诊断58：E0624

位置：`crates/libs/tool-computer/src/computer_control/chrome_tests.rs:176`。

```text
error[E0624]: method `stop` is private
   --> crates/vw-agent/src/tools/../../../libs/tool-computer/src/computer_control/chrome_tests.rs:176:17
    |
176 |         runtime.stop(&(context.tenant_scope(),"cc10".into(),None)).await;
    |                 ^^^^ private method
    |
   ::: crates/libs/tool-computer/src/lib.rs:283:5
    |
283 |     async fn stop(&self, owner: &Owner) {
    |     ----------------------------------- private method defined here
```

当前源码片段：

```rust
172:         let denied=execute_tool_from_registry(&tools,"computer_control",invoke(json!({"kind":"snapshot"})),context.clone()).await;
173:         anyhow::ensure!(denied.is_err(),"old handle survived redirect");println!("real Chrome redirect refused old handle: {}",denied.err().unwrap());
174:         anyhow::ensure!(seen.lock().iter().filter(|(name,_)|name=="computer_control.chrome_upload").count()==1,"upload approval missing");
175:         anyhow::ensure!(seen.lock().iter().filter(|(name,_)|name=="computer_control.chrome_download").count()==1,"download approval missing");
176:         runtime.stop(&(context.tenant_scope(),"cc10".into(),None)).await;
177:         Ok(())
178:     }.await;
179:     std::fs::write(&done, b"done")?;
180:     let status = tokio::time::timeout(Duration::from_secs(15), driver.wait()).await??;
181:     while let Some(line) = lines.next_line().await? {
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_control_10_full_access_and_turn_bypass_cannot_grant_transfers -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_control_10_real_chrome_tool_transfer_and_redirect_denial -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_control_10_transfer_approval_is_fresh_and_redacts_file_bytes -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。
