use std::{path::Path, process::Command};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let mut command = Command::new("python3");
    command.arg(root.join("scripts/relax_dev.py")).args(std::env::args_os().skip(1));
    command.current_dir(root);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        eprintln!("无法启动 Relax 开发入口（需要 Python 3）：{error}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        match command.status() {
            Ok(status) => std::process::exit(status.code().unwrap_or(1)),
            Err(error) => {
                eprintln!("无法启动 Relax 开发入口（需要 Python 3）：{error}");
                std::process::exit(1);
            }
        }
    }
}
