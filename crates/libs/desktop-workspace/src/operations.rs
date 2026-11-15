//! Ordered file effects; the desktop injects its existing gateway executor.
#[derive(Clone)]
pub enum Operation {
    Duplicate { root: String, path: String },
    Copy { root: String, source: String, target: String },
    Move { root: String, source: String, target: String },
    Delete { root: String, path: String, recursive: bool },
}
impl Operation {
    fn source(&self) -> &str {
        match self {
            Self::Duplicate { path, .. } | Self::Delete { path, .. } => path,
            Self::Copy { source, .. } | Self::Move { source, .. } => source,
        }
    }
}
pub struct Outcome {
    pub completed: Vec<String>,
    pub errors: Vec<String>,
}
/// Each effect is attempted once, in order, even after an earlier failure.
pub async fn run<Execute, Pending>(operations: Vec<Operation>, mut execute: Execute) -> Outcome
where
    Execute: FnMut(Operation) -> Pending,
    Pending: std::future::Future<Output = Result<(), String>>,
{
    let mut outcome = Outcome { completed: Vec::new(), errors: Vec::new() };
    for operation in operations {
        let source = operation.source().to_string();
        match execute(operation).await {
            Ok(()) => outcome.completed.push(source),
            Err(error) => outcome.errors.push(format!("{source}: {error}")),
        }
    }
    outcome
}
