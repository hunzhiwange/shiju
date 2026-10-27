//! Isolated suite hook counter. The adapter initializes state once per attempt.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

fn claim(state: &Path, event: &str, attempt: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(event, "model" | "tool") || attempt.is_empty() {
        return Err("invalid gate identity".into());
    }
    let lock = OpenOptions::new().read(true).write(true).open(state.with_extension("lock"))?;
    lock.lock()?;
    let raw = std::fs::read_to_string(state)?;
    let fields: Vec<_> = raw.split_whitespace().collect();
    if fields.len() != 6
        || fields[0] != "attempt"
        || fields[1] != attempt
        || fields[2] != "models"
        || fields[4] != "tools"
    {
        return Err("invalid gate state".into());
    }
    let mut models = fields[3].parse::<u64>()?;
    let mut tools = fields[5].parse::<u64>()?;
    if models > 24
        || tools > 32
        || (event == "model" && models == 24)
        || (event == "tool" && tools == 32)
    {
        return Err("attempt budget exhausted".into());
    }
    if event == "model" {
        models += 1;
    } else {
        tools += 1;
    }
    let temporary = state.with_extension(format!("{}-pending", std::process::id()));
    // A leftover pending write from a crashed process is never treated as state.
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    write!(file, "attempt {attempt}\nmodels {models}\ntools {tools}\n")?;
    file.sync_all()?;
    std::fs::rename(&temporary, state)?;
    File::open(state.parent().ok_or("state has no parent")?)?.sync_all()?;
    Ok(())
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let allowed = if args.len() == 3 {
        args[1]
            .to_str()
            .zip(args[2].to_str())
            .is_some_and(|(event, attempt)| claim(Path::new(&args[0]), event, attempt).is_ok())
    } else {
        false
    };
    if !allowed {
        eprintln!("suite execution budget rejected");
        std::process::exit(23);
    }
}

#[cfg(test)]
#[path = "budget_gate_tests.rs"]
mod tests;
