# 任务311：修复vw-agent宿主的desktop_mapping_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：继续执行宿主单测；诊断编号47, 48, 49（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/gateway-app-providers/src/host_tests/desktop_mapping_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断47：E0599

位置：`crates/libs/gateway-app-providers/src/host_tests/desktop_mapping_tests.rs:138`。

```text
error[E0599]: no method named `transport` found for struct `GatewayAppProviderClient` in the current scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-app-providers/src/host_tests/desktop_mapping_tests.rs:138:74
    |
138 |     assert!(gateway_app_provider::GatewayAppProviderClient::new(f.client.transport().clone()).claude_desktop_provider_switch("one",...
    |                                                                          ^^^^^^^^^ private field, not a method
```

当前源码片段：

```rust
134:     let saved = f.record("one").unwrap();
135:     request.mapping.routes.values_mut().next().unwrap().model = "bad\nmodel".into();
136:     assert!(f.client.claude_desktop_mapping_save("one", &request, TOKEN).await.is_err());
137:     assert_eq!(f.record("one").unwrap(), saved);
138:     assert!(gateway_app_provider::GatewayAppProviderClient::new(f.client.transport().clone()).claude_desktop_provider_switch("one", &f.target(), TOKEN).await.is_err());
139:     assert_eq!(std::fs::read(f.file()).unwrap(), before);
140:     assert!(!f.root.path().join("private/provider-live").exists());
141: }
142: #[tokio::test]
143: async fn cc04_desktop_mapping_imported_native_profile_reaches_transactional_files() {
```

### 宿主诊断48：E0599

位置：`crates/libs/gateway-app-providers/src/host_tests/desktop_mapping_tests.rs:171`。

```text
error[E0599]: no method named `transport` found for struct `GatewayAppProviderClient` in the current scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-app-providers/src/host_tests/desktop_mapping_tests.rs:171:70
    |
171 | ...   gateway_app_provider::GatewayAppProviderClient::new(f.client.transport().clone()).claude_desktop_provider_switch("native", &f...
    |                                                                    ^^^^^^^^^ private field, not a method
```

当前源码片段：

```rust
167:         f.client.claude_desktop_import_claude(&f.target(), TOKEN).await.unwrap().imported,
168:         1
169:     );
170:     let result =
171:         gateway_app_provider::GatewayAppProviderClient::new(f.client.transport().clone()).claude_desktop_provider_switch("native", &f.target(), TOKEN).await.unwrap();
172:     assert_eq!(result.live, ClaudeDesktopLiveResult::Applied);
173:     let profile = f.root.path().join(format!(
174:         "Library/Application Support/Claude-3p/configLibrary/{}.json",
175:         vw_agent_apps::providers::claude_desktop::PROFILE_ID
176:     ));
```

### 宿主诊断49：E0599

位置：`crates/libs/gateway-app-providers/src/host_tests/desktop_mapping_tests.rs:224`。

```text
error[E0599]: no method named `transport` found for struct `GatewayAppProviderClient` in the current scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-app-providers/src/host_tests/desktop_mapping_tests.rs:224:66
    |
224 |     gateway_app_provider::GatewayAppProviderClient::new(f.client.transport().clone()).claude_desktop_provider_save("form", &write, ...
    |                                                                  ^^^^^^^^^ private field, not a method
```

当前源码片段：

```rust
220:             },
221:         },
222:         activation: ClaudeDesktopActivation::CatalogOnly,
223:     };
224:     gateway_app_provider::GatewayAppProviderClient::new(f.client.transport().clone()).claude_desktop_provider_save("form", &write, TOKEN).await.unwrap();
225:     let before = f.record("form").unwrap();
226:     let view = f.client.claude_desktop_form_read("form", &f.target(), TOKEN).await.unwrap();
227:     assert_eq!(view.name, "中文直连");
228:     assert!(view.direct_editable && view.mapping_editable);
229:     assert_eq!(view.mapping.mode, DesktopMappingMode::Direct);
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc04_desktop_mapping_imported_native_profile_reaches_transactional_files -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc04_desktop_mapping_sdk_api_import_validates_and_never_activates -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc04_ui_desktop_form_read_is_redacted_and_permission_checked -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。
