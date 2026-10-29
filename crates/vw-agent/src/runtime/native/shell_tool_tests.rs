use super::*;
use crate::tools::shell::ShellTool;
use crate::tools::traits::Tool;
use std::sync::Arc;

#[test]
fn native_shell_tool_reports_selected_interpreter_and_syntax() {
    for (kind, expected) in [
        (ShellKind::PowerShell, "Windows PowerShell 5.1"),
        (ShellKind::Pwsh, "PowerShell 7"),
        (ShellKind::Cmd, "Windows CMD"),
        (ShellKind::Bash, "Bash"),
        (ShellKind::Sh, "POSIX sh"),
        (ShellKind::Zsh, "Zsh"),
    ] {
        let runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind,
            program: PathBuf::from("unused-test-shell"),
        }));
        let tool =
            ShellTool::new(Arc::new(crate::security::SecurityPolicy::default()), Arc::new(runtime));
        let spec = tool.spec();
        assert!(spec.description.contains(expected));
        let command = spec.input_schema["properties"]["command"]["description"].as_str().unwrap();
        assert!(command.contains(expected));
        if kind == ShellKind::PowerShell {
            assert!(command.contains("&& and || operators are unsupported"));
            assert!(command.contains("pwd; ls -Force"));
        }
    }
}
