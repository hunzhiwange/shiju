//! Shared black-box harness for the actual evidence summarizer.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(super) fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/plugin_compat/t66")
}

pub(super) fn summarize(baseline: &Path, sources: &Path, output: &Path) -> Output {
    Command::new("python3")
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../scripts/plugin_compat/summarize_evidence.py"),
        )
        .arg("--baseline")
        .arg(baseline)
        .arg("--evidence-dir")
        .arg(sources)
        .arg("--output")
        .arg(output)
        .output()
        .expect("real evidence summarizer")
}
