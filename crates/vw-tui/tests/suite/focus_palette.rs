//! 通过真实 PTY 验证终端重新获得焦点时，调色板查询不会阻塞或吞掉即时输入。
//! fixture 缓存启动阶段的终端应答，再分别覆盖同批和延迟到达的焦点输入。

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::path::Path;
use std::process::Child;
use std::process::Command;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use tempfile::TempDir;

// Full startup continues after the composer first appears and can be slower under Rosetta in CI.
// 完整启动会在输入框首次出现后继续执行，在 CI 的 Rosetta 环境中可能更慢。
const STARTUP_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 30);
const FOCUS_INPUT_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 5);
const FOCUS_PROBE_INPUT: &str = "focus-palette-24527";

#[test]
// 已有未回答调色板查询时，焦点事件后的即时和稍后输入都必须进入编辑器。
fn focus_gained_with_unanswered_palette_queries_preserves_immediate_input() -> Result<()> {
    let repo_root = codex_utils_cargo_bin::repo_root()?;
    let codex_home = tempfile::tempdir()?;
    write_test_config(codex_home.path(), &repo_root)?;

    let mut terminal = PtyCodex::start(&repo_root, codex_home)?;
    terminal.wait_for_startup()?;

    let startup_output_len = terminal.output.len();
    let focus_started = Instant::now();
    terminal.write_input(format!("\u{1b}[I{FOCUS_PROBE_INPUT}").as_bytes())?;
    terminal.wait_for_focus_input(FOCUS_PROBE_INPUT, focus_started, startup_output_len)?;

    let delayed_input = format!("{FOCUS_PROBE_INPUT}-delayed");
    let delayed_focus_started = Instant::now();
    terminal.write_input(b"\x1b[I")?;
    terminal.read_output(Duration::from_millis(/*millis*/ 20))?;
    terminal.write_input(delayed_input.as_bytes())?;
    terminal.wait_for_focus_input(&delayed_input, delayed_focus_started, startup_output_len)?;

    Ok(())
}

struct PtyCodex {
    // 真实子进程、主 PTY 与 vt100 解析器共同保留字节级和屏幕级观察面。
    master: File,
    child: Child,
    parser: vt100::Parser,
    output: Vec<u8>,
    cursor_answered: bool,
    palette_answered: bool,
    keyboard_answered: bool,
    _codex_home: TempDir,
}

impl PtyCodex {
    fn start(repo_root: &Path, codex_home: TempDir) -> Result<Self> {
        // 创建隔离 PTY 和配置目录，启动禁用替代屏幕的真实 TUI 子进程。
        let mut master_fd = -1;
        let mut slave_fd = -1;
        let mut window_size = libc::winsize {
            ws_row: 32,
            ws_col: 120,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        // SAFETY: `openpty` initializes both file descriptors on success, and the supplied window
        // size remains valid for the duration of the call.
        // 安全性：成功时会初始化两个描述符，窗口尺寸在调用期间保持有效。
        let result = unsafe {
            libc::openpty(
                &mut master_fd,
                &mut slave_fd,
                /*name*/ std::ptr::null_mut(),
                /*termp*/ std::ptr::null_mut(),
                &raw mut window_size,
            )
        };
        if result == -1 {
            return Err(std::io::Error::last_os_error()).context("open focus-test pseudo-terminal");
        }

        // SAFETY: a successful `openpty` transfers ownership of both unique file descriptors.
        // 安全性：成功调用会转移两个互不重复描述符的所有权。
        let master = File::from(unsafe { OwnedFd::from_raw_fd(master_fd) });
        // SAFETY: `slave_fd` is the second unique descriptor initialized by `openpty`.
        // 安全性：`slave_fd` 是 `openpty` 初始化的第二个唯一描述符。
        let slave = File::from(unsafe { OwnedFd::from_raw_fd(slave_fd) });
        let stdin = slave.try_clone().context("clone pseudo-terminal stdin")?;
        let stdout = slave.try_clone().context("clone pseudo-terminal stdout")?;

        let codex = codex_utils_cargo_bin::cargo_bin("codex")
            .or_else(|_| codex_utils_cargo_bin::cargo_bin("codex-tui"))?;
        let child = Command::new(codex)
            .arg("--no-alt-screen")
            .arg("-C")
            .arg(repo_root)
            .arg("-c")
            .arg("analytics.enabled=false")
            .env("TERM", "xterm-256color")
            .env("OPENAI_API_KEY", "focus-palette-test")
            .env("CODEX_HOME", codex_home.path())
            .stdin(stdin)
            .stdout(stdout)
            .stderr(slave)
            .spawn()
            .context("start Codex in focus-test pseudo-terminal")?;

        Ok(Self {
            master,
            child,
            parser: vt100::Parser::new(
                /*rows*/ 32, /*cols*/ 120, /*scrollback_len*/ 0,
            ),
            output: Vec::new(),
            cursor_answered: false,
            palette_answered: false,
            keyboard_answered: false,
            _codex_home: codex_home,
        })
    }

    fn wait_for_startup(&mut self) -> Result<()> {
        // 回答启动查询直到输入框可见，并对提前退出或超时给出屏幕诊断。
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        while Instant::now() < deadline {
            self.read_output(Duration::from_millis(/*millis*/ 50))?;
            self.answer_startup_queries()?;

            if self.palette_answered && self.screen_contains("OpenAI Codex") {
                return Ok(());
            }

            if let Some(status) = self.child.try_wait()? {
                bail!(
                    "Codex exited before the focus test started ({status}); screen:\n{}",
                    self.screen_contents(),
                );
            }
        }

        bail!(
            "Codex did not initialize within {:?}; screen:\n{}",
            STARTUP_TIMEOUT,
            self.screen_contents(),
        );
    }

    fn wait_for_focus_input(
        &mut self,
        input: &str,
        focus_started: Instant,
        startup_output_len: usize,
    ) -> Result<()> {
        // 等待目标文本出现，同时禁止焦点恢复后重新发起已缓存的颜色查询。
        while focus_started.elapsed() < FOCUS_INPUT_TIMEOUT {
            self.read_output(Duration::from_millis(/*millis*/ 20))?;
            let focus_output = &self.output[startup_output_len..];
            ensure!(
                !contains_bytes(focus_output, b"\x1b]10;?")
                    && !contains_bytes(focus_output, b"\x1b]11;?"),
                "focus regain queried terminal colors after the startup palette was cached",
            );
            if self.screen_contains(input) {
                return Ok(());
            }
        }

        bail!(
            "focus-time palette refresh blocked or discarded {input:?} for more than {:?}; \
             screen:\n{}",
            FOCUS_INPUT_TIMEOUT,
            self.screen_contents(),
        );
    }

    fn answer_startup_queries(&mut self) -> Result<()> {
        // 对光标、键盘模式和前后景色查询各应答一次，模拟可预测终端。
        if !self.cursor_answered && contains_bytes(&self.output, b"\x1b[6n") {
            self.write_input(b"\x1b[1;1R")?;
            self.cursor_answered = true;
        }

        if !self.keyboard_answered && contains_bytes(&self.output, b"\x1b[?u") {
            self.write_input(b"\x1b[?0u\x1b[?1;2c")?;
            self.keyboard_answered = true;
        }

        if !self.palette_answered
            && contains_bytes(&self.output, b"\x1b]10;?")
            && contains_bytes(&self.output, b"\x1b]11;?")
        {
            self.write_input(b"\x1b]10;rgb:ffff/ffff/ffff\x1b\\\x1b]11;rgb:0000/0000/0000\x1b\\")?;
            self.palette_answered = true;
        }

        Ok(())
    }

    fn read_output(&mut self, timeout: Duration) -> Result<()> {
        let timeout_ms = timeout.as_millis().min(libc::c_int::MAX as u128) as libc::c_int;
        let mut descriptor = libc::pollfd {
            fd: self.master.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        // SAFETY: `descriptor` points to one initialized poll descriptor.
        // 安全性：`descriptor` 指向一个已初始化的 poll 描述符。
        let ready = unsafe {
            libc::poll(&mut descriptor, /*nfds*/ 1, timeout_ms)
        };
        if ready == -1 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                return Ok(());
            }
            return Err(error).context("poll focus-test pseudo-terminal");
        }
        if ready == 0 || descriptor.revents & libc::POLLIN == 0 {
            return Ok(());
        }

        let mut chunk = [0_u8; 8192];
        let count = self.master.read(&mut chunk)?;
        self.output.extend_from_slice(&chunk[..count]);
        self.parser.process(&chunk[..count]);
        Ok(())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<()> {
        self.master.write_all(bytes)?;
        self.master.flush()?;
        Ok(())
    }

    fn screen_contains(&self, text: &str) -> bool {
        self.parser.screen().contents().contains(text)
    }

    fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }
}

impl Drop for PtyCodex {
    // 无论断言如何结束都回收子进程，避免集成测试留下后台 TUI。
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn contains_bytes(buffer: &[u8], needle: &[u8]) -> bool {
    buffer.windows(needle.len()).any(|window| window == needle)
}

fn write_test_config(codex_home: &Path, repo_root: &Path) -> Result<()> {
    // 写入最小可信项目配置和伪认证，确保测试不读取宿主用户状态。
    let repo_root = repo_root.display();
    let config = format!(
        "model = \"gpt-5.6-terra\"\nmodel_provider = \"openai\"\n\
         suppress_unstable_features_warning = true\n\n\
         [projects.\"{repo_root}\"]\ntrust_level = \"trusted\"\n"
    );
    std::fs::write(codex_home.join("config.toml"), config)
        .context("write focus-test Codex configuration")?;
    std::fs::write(
        codex_home.join("auth.json"),
        r#"{"OPENAI_API_KEY":"focus-palette-test","tokens":null,"last_refresh":null}"#,
    )
    .context("write focus-test API-key authentication")
}
