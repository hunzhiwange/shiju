# 任务311：修复vw-agent宿主的desktop_mapping_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

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

## 2026-11-18 执行记录

- 已核对宿主父模块注册、Fixture 和 GatewayAppProviderClient 接口；三处直接复用 f.client 调用 provider_switch/provider_save，不新增 transport 访问器，不改变断言或业务接口。
- 首次定向命令日志 `/tmp/shiju-task311-before.log`；发现其他宿主编译后中断自有入口，退出130，已等待清理完成，不作为红灯通过证据。原始诊断47—49为修复依据。
- 修改后在槽位1执行以下串行入口，两缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`：

```sh
python3 scripts/cargo_slot.py sh -c 'cargo test -p vw-agent --lib cc04_desktop_mapping_sdk_api_import_validates_and_never_activates -- --nocapture && cargo test -p vw-agent --lib cc04_desktop_mapping_imported_native_profile_reaches_transactional_files -- --nocapture && cargo test -p vw-agent --lib cc04_ui_desktop_form_read_is_redacted_and_permission_checked -- --nocapture'
```

- 日志 `/tmp/shiju-task311-after.log`，退出101；vw-agent lib test 仍有61条兄弟编译错误（包括视频 VideoContext 契约及 IntoResponse trait 缺失），本文件不再出现诊断。有效测试数0，后两命令因首条失败未执行，不记行为通过。
- 自有入口已退出并释放槽位；未执行全量、桌面构建或额外 check。待兄弟任务解除宿主编译阻塞后重跑上述三条命令，必要验证通过前保留任务卡。

## 2026-11-18 复用公共产物验证

- 编译修复提交 `b8e9794ae` 已核对，包含在公共构建起始 HEAD `3382a35f8`，结束 HEAD `17e53b009`。编译退出 0，槽位 1，直接使用 `target/rust-slot-1/debug/deps/vw_agent-e6b6e74ae78b3318`。SHA-256 为 `5548b4e2c673a564f9d12af91061c0e01e0334f68881fd8cf360a1cbf5933e78`，凭据为 `/tmp/shiju-host-common-304-LYrH28/receipt.md`。
- 每条命令均为上述二进制加 `--exact --nocapture` 和完整测试名，模块前缀为 `gateway::api::handlers::app_providers::desktop_mapping_tests::`。每个子进程独立设置 HOME、CODEX_HOME、XDG、VIBEWINDOW_TEST_ROOT、VIBEWINDOW_CONFIG_DIR、CLAUDE_CONFIG_DIR、TMPDIR 和工作目录。源码核对确认 SQLite、策略、Claude 配置、凭证路径、监听端口均来自本地临时夹具。
- `cc04_desktop_mapping_sdk_api_import_validates_and_never_activates`：运行 1，失败 1，退出 101。第 103 行，错误令牌的导入调用返回成功。日志 `/tmp/shiju-task311-recheck-BVh9Wb/1.log`。
- `cc04_desktop_mapping_imported_native_profile_reaches_transactional_files`：运行 1，失败 1，退出 101。第 177 行，返回 Applied 后读取预期 profile 文件得到 ENOENT。日志 `/tmp/shiju-task311-recheck-BVh9Wb/2.log`。
- `cc04_ui_desktop_form_read_is_redacted_and_permission_checked`：运行 1，失败 1，退出 101。第 226 行，表单读取返回 HTTP 409。日志 `/tmp/shiju-task311-recheck-BVh9Wb/3.log`。
- 合计 3 项有效测试，0 通过，3 失败。执行和产物记录在 `/tmp/shiju-task311-recheck-BVh9Wb/results.json` 和 `artifact.txt`。槽位已释放。三个运行期故障需按实际业务归属修复后用新产物复验，当前授权禁止重编，保留任务卡。

## 2026-11-18 迁移运行期测试契约

- 应用路由的执行上下文已由 Gateway 配对、进程运行时路径和请求租户决定。移除仅对旧契约有效的授权文件，让真实 PairingGuard 校验端点携带的 skey。错误配对的读写均断言 401，并核查目录、记录及文件副作用。旧参数仅用于维持现有 SDK 签名。
- 四个用例在独立子进程中使用全新临时目录作为 HOME、CODEX_HOME、XDG 和应用状态路径。事务和原生 profile 跟随该运行时目录。表单测试使用项目绑定事务及现行 scoped 请求，检查脱敏、有效 revision、跨项目 403、非法 URL 422，以及读取不改变状态和配置文件。
- 本轮仅修改本编号拥有的测试文件与任务卡。已运行该文件 rustfmt 和 git diff --check。重型编译窗口由其他任务持有，未重编或运行旧产物。
- 新公共产物包含本轮提交后，按完全限定名称分别验证原三项及夹具影响的 `cc04_desktop_mapping_permissions_refuse_catalog_side_effects`，记录四项实际执行数、退出码和日志。每项需观察 1 passed / 0 failed，退出 0。
